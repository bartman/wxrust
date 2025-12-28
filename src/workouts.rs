use crate::api;
use crate::auth;
use crate::formatters;
use crate::models;
use crate::parsers;
use chrono::{Datelike, Utc};
use std::fs;
use std::path::PathBuf;

fn get_cache_dir(uid: u32) -> Result<PathBuf, String> {
    let cache_dir = if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
        if !dir.is_empty() {
            PathBuf::from(dir)
        } else {
            std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".cache")).ok_or("No cache dir")?
        }
    } else {
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".cache")).ok_or("No cache dir")?
    };
    let full = cache_dir.join("wxrust").join(uid.to_string());
    Ok(full)
}

fn get_cache_file_path(uid: u32, date: &str) -> Result<PathBuf, String> {
    let cache_dir = get_cache_dir(uid)?;
    Ok(cache_dir.join(format!("{}.txt", date)))
}

fn lookup_cached_jday(uid: u32, date: &str, verbose: bool) -> Option<models::JDay> {
    if let Ok(cache_path) = get_cache_file_path(uid, date) {
        if cache_path.exists() {
            if let Ok(content) = fs::read_to_string(&cache_path) {
                if let Ok(jday) = parsers::parse_workout(&content) {
                    if verbose {
                        eprintln!("\x1b[34mgetting {} from cache {}\x1b[0m", date, cache_path.display());
                    }
                    return Some(jday);
                }

                if verbose {
                    eprintln!("\x1b[34mfailed parsing {} from cache {}\x1b[0m", date, cache_path.display());
                }
            }
        }
    }
    None
}

fn write_cached_jday(uid: u32, date: &str, jday: &models::JDay) {
    if let Ok(cache_path) = get_cache_file_path(uid, date) {
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let plain = formatters::format_workout_for_cache(date, jday);
        let plain = plain + "\n";
        // Write to temp file then rename
        let temp_path = cache_path.with_extension("tmp");
        if let Ok(()) = fs::write(&temp_path, plain) {
            let _ = fs::rename(&temp_path, &cache_path);
        }
    }
}

#[allow(unreachable_code)]
pub async fn get_jday<C: crate::api::ApiClient>(client: &C, token: &str, date: &str, verbose: bool) -> Result<models::JDay, String> {
    let claims = auth::decode_token(&token).map_err(|e| e.to_string())?;
    let uid = claims.id;

    // Check cache
    if let Some(jday) = lookup_cached_jday(uid, date, verbose) {
        return Ok(jday);
    }

    let query = format!(r#"
query {{
  jday(uid: {}, ymd: "{}") {{
    log
    bw
    eblocks {{
      eid
      sets {{ w r s lb rpe pr est1rm eff int type t d dunit speed force c }}
    }}
    exercises {{
      exercise {{ id name type }}
    }}
  }}
}}
"#,
    uid, date);

    let response: models::GraphQLResponse<models::WorkoutData> = api::graphql_request(client, token, &query, None).await.map_err(|e| e.to_string())?;

    if let Some(errors) = response.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
    }

    if let Some(data) = response.data {
        if let Some(jday) = data.jday {
            // Cache the plain version, using kg
            write_cached_jday(uid, date, &jday);
            // return the jday
            Ok(jday)
        } else {
            Err("No workout found for the date.".to_string())
        }
    } else {
        Err("Unexpected response.".to_string())
    }
}

pub async fn get_dates<C: crate::api::ApiClient>(client: &C, token: &str, latest: Option<String>, oldest: Option<String>, count: u32, reverse: bool) -> Result<Vec<String>, String> {
    let claims = auth::decode_token(&token).map_err(|e| e.to_string())?;
    let uid = claims.id;

    let initial_ymd = latest.clone().unwrap_or_else(|| {
        let today = Utc::now().date_naive();
        format!("{:04}-{:02}-{:02}", today.year(), today.month(), today.day())
    });

    let query = r#"
query GetJRange($uid: ID!, $ymd: YMD!, $range: Int!) {
  jrange(uid: $uid, ymd: $ymd, range: $range) {
    days {
      on
    }
  }
}
"#;

    let mut all_dates: Vec<String> = Vec::new();
    let mut current_ymd = initial_ymd.clone();

    loop {
        let want = (count as usize) - all_dates.len();
        let batch_size = std::cmp::min(32, want);

        let variables = serde_json::json!({ "uid": uid.to_string(), "ymd": current_ymd.clone(), "range": batch_size });

        let response: models::GraphQLResponse<models::GetJRangeData> = api::graphql_request(client, token, query, Some(variables)).await.map_err(|e| e.to_string())?;

        if let Some(errors) = response.errors {
            return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
        }

        let days = if let Some(data) = response.data {
            if let Some(jrange) = data.jrange {
                jrange.days.unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            return Err("Unexpected response.".to_string());
        };

        let mut date_strings: Vec<String> = days.into_iter()
            .filter_map(|day| day.on)
            .map(|d| format!("{}-{}-{}", &d[0..4], &d[5..7], &d[8..10]))
            .collect();

        if date_strings.is_empty() {
            break;
        }

        date_strings.sort();

        // Filter dates
        let filtered: Vec<String> = date_strings.iter().cloned()
            .filter(|d| {
                if let Some(old) = &oldest {
                    d >= old
                } else {
                    true
                }
            })
            .filter(|d| {
                if let Some(lat) = &latest {
                    d <= lat
                } else {
                    true
                }
            })
            .collect();

        all_dates.extend(filtered);

        // Check if we have enough
        if count > 0 && all_dates.len() >= count as usize {
            break;
        }

        // Check if we reached the oldest
        if let Some(old) = &oldest {
            if let Some(batch_oldest) = date_strings.first() {
                if batch_oldest < old {
                    break;
                }
            }
        }

        // Set next ymd to the oldest in this batch to get older dates
        if let Some(oldest_in_batch) = date_strings.first() {
            current_ymd = oldest_in_batch.clone();
        } else {
            break;
        }
    }

    // Remove duplicates and sort
    all_dates.sort();
    all_dates.dedup();

    let mut result = if count > 0 {
        // Take the most recent count
        all_dates.into_iter().rev().take(count as usize).collect()
    } else {
        all_dates
    };

    result.sort();
    if reverse {
        result.reverse();
    }
    Ok(result)
}
