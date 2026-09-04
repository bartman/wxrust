use crate::api;
use crate::formatters;
use crate::models;
use crate::parsers;
use chrono::{Datelike, Utc};
use futures::StreamExt;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// Number of `jday` selections packed into one GraphQL request via aliases.
pub const JDAY_BATCH_SIZE: usize = 10;
/// Maximum number of batched GraphQL requests in flight during a bulk fetch.
pub const FETCH_CONCURRENCY: usize = 8;

const JDAY_FIELDS: &str = r#"    log
    bw
    eblocks {
      eid
      sets { w r s lb rpe pr est1rm eff int type t d dunit speed force c }
    }
    exercises {
      exercise { id name type }
    }"#;

lazy_static! {
    static ref USER_WANTS_KG: Mutex<Option<bool>> = Mutex::new(None);
}

fn filter_dates_by_range(dates: Vec<String>, oldest: Option<&str>, latest: Option<&str>) -> Vec<String> {
    dates.into_iter()
        .filter(|d| oldest.is_none_or(|old| d.as_str() >= old))
        .filter(|d| latest.is_none_or(|lat| d.as_str() <= lat))
        .collect()
}

fn limit_and_sort_dates(mut dates: Vec<String>, count: u32, reverse: bool) -> Vec<String> {
    if count > 0 {
        dates = dates.into_iter().rev().take(count as usize).collect();
        dates.sort();
    }
    if reverse {
        dates.reverse();
    }
    dates
}

pub fn get_cache_base_dir() -> Result<PathBuf, String> {
    let cache_dir = if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
        if !dir.is_empty() {
            PathBuf::from(dir)
        } else {
            std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".cache")).ok_or("No cache dir")?
        }
    } else {
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".cache")).ok_or("No cache dir")?
    };
    Ok(cache_dir.join("wxrust"))
}

#[allow(dead_code)]
pub fn read_cached_user_wants_kg() -> Option<bool> {
    let mut guard = USER_WANTS_KG.lock().unwrap();
    if guard.is_none() {
        *guard = {
            let cache_dir = get_cache_base_dir().ok()?;
            let file_path = cache_dir.join("user_wants_kg");
            if !file_path.exists() {
                None
            } else {
                let content = fs::read_to_string(&file_path).ok()?;
//eprintln!("#### RD {:?} -> {}", file_path, content.trim());
                match content.trim() {
                    "0" => Some(false),
                    "1" => Some(true),
                    _ => {
                        eprintln!("Warning: invalid content in user_wants_kg file");
                        None
                    }
                }
            }
        };
    }
//eprintln!("#### RD cache -> {:?}", *guard);
    *guard
}

#[allow(dead_code)]
pub fn read_cached_user_wants_kg_or(default: bool) -> bool {
    read_cached_user_wants_kg().unwrap_or(default)
}

#[allow(dead_code)]
pub fn write_cached_user_wants_kg(value: bool) {
    if let Ok(cache_dir) = get_cache_base_dir() {
        let file_path = cache_dir.join("user_wants_kg");
        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = if value { "1\n" } else { "0\n" };
        let temp_path = file_path.with_extension("tmp");
        if let Ok(()) = fs::write(&temp_path, content) {
            let _ = fs::rename(&temp_path, &file_path);
        }
    }
//eprintln!("#### WR cache <- {}", value);
    *USER_WANTS_KG.lock().unwrap() = Some(value);
}

#[allow(dead_code)]
pub fn forget_cached_user_wants_kg() {
    *USER_WANTS_KG.lock().unwrap() = None;
}

fn get_cache_dir(uid: u32) -> Result<PathBuf, String> {
    Ok(get_cache_base_dir()?.join(uid.to_string()))
}

fn get_cache_file_path(uid: u32, date: &str) -> Result<PathBuf, String> {
    let cache_dir = get_cache_dir(uid)?;
    Ok(cache_dir.join(format!("{}.txt", date)))
}

/// True if a cache file exists for this uid/date (does not parse the contents).
pub fn cached_jday_exists(uid: u32, date: &str) -> bool {
    get_cache_file_path(uid, date).map(|p| p.exists()).unwrap_or(false)
}

pub fn jday_alias(index: usize) -> String {
    format!("d{}", index)
}

pub fn chunk_dates(dates: &[String], batch_size: usize) -> Vec<Vec<String>> {
    let batch_size = batch_size.max(1);
    dates.chunks(batch_size).map(|c| c.to_vec()).collect()
}

pub fn build_jday_query(uid: u32, date: &str) -> String {
    format!(
        "query {{\n  jday(uid: {}, ymd: \"{}\") {{\n{}\n  }}\n}}\n",
        uid, date, JDAY_FIELDS
    )
}

pub fn build_batch_jday_query(uid: u32, dates: &[String]) -> String {
    let mut q = String::from("query {\n");
    for (i, date) in dates.iter().enumerate() {
        q.push_str(&format!(
            "  {}: jday(uid: {}, ymd: \"{}\") {{\n{}\n  }}\n",
            jday_alias(i),
            uid,
            date,
            JDAY_FIELDS
        ));
    }
    q.push_str("}\n");
    q
}

pub fn get_dates_from_cache(uid: u32, latest: Option<String>, oldest: Option<String>, count: u32, reverse: bool) -> Result<Vec<String>, String> {
    let cache_dir = get_cache_dir(uid)?;
    if !cache_dir.exists() {
        return Ok(vec![]);
    }

    let mut dates: Vec<String> = vec![];
    let entries = fs::read_dir(&cache_dir).map_err(|e| format!("Failed to read cache dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
                && ext == "txt"
                    && let Some(stem) = path.file_stem()
                        && let Some(date_str) = stem.to_str() {
                            // Basic validation: should be YYYY-MM-DD format
                            if date_str.len() == 10 && date_str.chars().nth(4) == Some('-') && date_str.chars().nth(7) == Some('-') {
                                dates.push(date_str.to_string());
                            }
                        }
    }

    // Sort dates
    dates.sort();

    let filtered = filter_dates_by_range(dates, oldest.as_deref(), latest.as_deref());
    let result = limit_and_sort_dates(filtered, count, reverse);

    Ok(result)
}

pub fn lookup_cached_jday(uid: u32, date: &str, verbose: bool) -> Option<models::JDay> {
    if let Ok(cache_path) = get_cache_file_path(uid, date)
        && cache_path.exists()
            && let Ok(content) = fs::read_to_string(&cache_path) {
                // Use cached user preference to parse the cached workout
                let user_wants_kg = read_cached_user_wants_kg_or(true);
                let options = parsers::ParserOptions::new(user_wants_kg);
                if let Ok(jday) = parsers::parse_workout_with_options(&content, &options) {
                    if verbose {
                        eprintln!("\x1b[34mgetting {} from cache {}\x1b[0m", date, cache_path.display());
                    }
                    return Some(jday);
                }

                if verbose {
                    eprintln!("\x1b[34mfailed parsing {} from cache {}\x1b[0m", date, cache_path.display());
                }
            }
    None
}

pub fn write_cached_jday(uid: u32, date: &str, jday: &models::JDay) {
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
pub async fn get_jday<C: crate::api::ApiClient>(data_access: &crate::api::DataAccess<'_, C>, date: &str, verbose: bool) -> Result<models::JDay, String> {
    let uid = data_access.uid.ok_or("No user ID available")?;
    let client = data_access.client;

    // Check cache if allowed
    if data_access.use_cache
        && let Some(jday) = lookup_cached_jday(uid, date, verbose) {
            return Ok(jday);
        }

    if !data_access.use_network {
        return Err(format!("No workout found for {} (network access disabled)", date));
    }

    let token = data_access.token.ok_or("No token available for network request")?;

    let query = build_jday_query(uid, date);

    let response: models::GraphQLResponse<models::WorkoutData> = api::graphql_request(client, token, &query, None).await.map_err(|e| e.to_string())?;

    if let Some(errors) = response.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
    }

    if let Some(data) = response.data {
        if let Some(jday) = data.jday {
            // Cache the plain version if allowed
            if data_access.write_cache {
                write_cached_jday(uid, date, &jday);
            }
            // return the jday
            Ok(jday)
        } else {
            Err("No workout found for the date.".to_string())
        }
    } else {
        Err("Unexpected response.".to_string())
    }
}

async fn fetch_jdays_from_network<C: crate::api::ApiClient>(
    data_access: &crate::api::DataAccess<'_, C>,
    uid: u32,
    dates: &[String],
    _verbose: bool,
) -> Result<Vec<(String, models::JDay)>, String> {
    let token = data_access.token.ok_or("No token available for network request")?;
    let query = build_batch_jday_query(uid, dates);
    let response: models::GraphQLResponse<models::BatchJDayData> =
        api::graphql_request(data_access.client, token, &query, None)
            .await
            .map_err(|e| e.to_string())?;

    if let Some(errors) = response.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
    }

    let mut data = response.data.ok_or_else(|| "Unexpected response.".to_string())?;
    let mut results = Vec::with_capacity(dates.len());
    for (i, date) in dates.iter().enumerate() {
        let key = jday_alias(i);
        match data.remove(&key) {
            Some(Some(jday)) => results.push((date.clone(), jday)),
            Some(None) => return Err(format!("No workout found for {}", date)),
            None => return Err(format!("Unexpected response: missing {} for {}", key, date)),
        }
    }
    Ok(results)
}

/// Fetch a group of dates in a single GraphQL request (via aliases).
///
/// Cached dates are served locally when `use_cache` is set. Newly fetched
/// workouts are written to cache when `write_cache` is set.
pub async fn get_jdays_batch<C: crate::api::ApiClient>(
    data_access: &crate::api::DataAccess<'_, C>,
    dates: &[String],
    verbose: bool,
) -> Result<Vec<(String, models::JDay)>, String> {
    if dates.is_empty() {
        return Ok(vec![]);
    }

    let uid = data_access.uid.ok_or("No user ID available")?;
    let mut found: HashMap<String, models::JDay> = HashMap::new();
    let mut missing: Vec<String> = Vec::new();

    for date in dates {
        if data_access.use_cache {
            if let Some(jday) = lookup_cached_jday(uid, date, verbose) {
                found.insert(date.clone(), jday);
                continue;
            }
        }
        missing.push(date.clone());
    }

    if !missing.is_empty() {
        if !data_access.use_network {
            return Err(format!("No workout found for {} (network access disabled)", missing[0]));
        }
        let fetched = fetch_jdays_from_network(data_access, uid, &missing, verbose).await?;
        for (date, jday) in fetched {
            if data_access.write_cache {
                write_cached_jday(uid, &date, &jday);
            }
            found.insert(date, jday);
        }
    }

    let mut results = Vec::with_capacity(dates.len());
    for date in dates {
        match found.remove(date) {
            Some(jday) => results.push((date.clone(), jday)),
            None => return Err(format!("No workout found for {}", date)),
        }
    }
    Ok(results)
}

/// Fetch many dates concurrently, packing up to `JDAY_BATCH_SIZE` into each request.
pub async fn get_jdays<C: crate::api::ApiClient>(
    data_access: &crate::api::DataAccess<'_, C>,
    dates: &[String],
    verbose: bool,
) -> Result<Vec<(String, models::JDay)>, String> {
    get_jdays_with_callback(
        data_access,
        dates,
        JDAY_BATCH_SIZE,
        FETCH_CONCURRENCY,
        verbose,
        |_, _| {},
    )
    .await
}

pub async fn get_jdays_with_callback<C, F>(
    data_access: &crate::api::DataAccess<'_, C>,
    dates: &[String],
    batch_size: usize,
    concurrency: usize,
    verbose: bool,
    mut on_workout: F,
) -> Result<Vec<(String, models::JDay)>, String>
where
    C: crate::api::ApiClient,
    F: FnMut(&str, &models::JDay),
{
    if dates.is_empty() {
        return Ok(vec![]);
    }

    let chunks = chunk_dates(dates, batch_size);
    let concurrency = concurrency.max(1);
    let mut stream = futures::stream::iter(chunks)
        .map(|chunk| async move { get_jdays_batch(data_access, &chunk, verbose).await })
        .buffer_unordered(concurrency);

    let mut collected: HashMap<String, models::JDay> = HashMap::new();
    while let Some(result) = stream.next().await {
        for (date, jday) in result? {
            on_workout(&date, &jday);
            collected.insert(date, jday);
        }
    }

    let mut ordered = Vec::with_capacity(dates.len());
    for date in dates {
        match collected.remove(date) {
            Some(jday) => ordered.push((date.clone(), jday)),
            None => return Err(format!("No workout found for {}", date)),
        }
    }
    Ok(ordered)
}

pub async fn get_dates<C: crate::api::ApiClient>(data_access: &crate::api::DataAccess<'_, C>, latest: Option<String>, oldest: Option<String>, count: u32, reverse: bool) -> Result<Vec<String>, String> {
    let uid = data_access.uid.ok_or("No user ID available")?;
    let client = data_access.client;

    if !data_access.use_network {
        return get_dates_from_cache(uid, latest, oldest, count, reverse);
    }

    let token = data_access.token.ok_or("No token available for network request")?;

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

        let date_count_before = all_dates.len();

        let filtered = filter_dates_by_range(date_strings.clone(), oldest.as_deref(), latest.as_deref());
        all_dates.extend(filtered);

        // Remove duplicates and sort
        all_dates.sort();
        all_dates.dedup();

        let date_count_after = all_dates.len();
        if date_count_before == date_count_after {
            break;
        }

        // Check if we have enough
        if count > 0 && all_dates.len() >= count as usize {
            break;
        }

        // Check if we reached the oldest
        if let Some(old) = &oldest
            && let Some(batch_oldest) = date_strings.first()
                && batch_oldest < old {
                    break;
                }

        // Set next ymd to the oldest in this batch to get older dates
        if let Some(oldest_in_batch) = date_strings.first() {
            current_ymd = oldest_in_batch.clone();
        } else {
            break;
        }
    }

    let result = limit_and_sort_dates(all_dates, count, reverse);
    Ok(result)
}

pub async fn resolve_user_wants_kg<C: crate::api::ApiClient>(
    data_access: &crate::api::DataAccess<'_, C>,
) -> bool {
    if let Some(token) = data_access.token {
        data_access.client.user_wants_kg(token).await
    } else {
        read_cached_user_wants_kg_or(true)
    }
}

pub async fn get_dates_from_ranges<C: crate::api::ApiClient>(
    data_access: &crate::api::DataAccess<'_, C>,
    ranges: &[String],
) -> Result<Vec<String>, String> {
    let mut all_dates: Vec<String> = vec![];
    for range_str in ranges {
        let (oldest, latest) = match crate::utils::parse_date_range(range_str) {
            Ok(start_end) => start_end,
            Err(e) => return Err(format!("Invalid date range '{}': {}", range_str, e)),
        };

        // don't ask for dates in the future
        let now = Utc::now().date_naive();
        let latest = if latest > now { now } else { latest };

        // limit the query to the number of days in the range
        let count = ((oldest - latest).num_days().abs() + 1) as u32;
        let dates = match get_dates(data_access, Some(latest.to_string()), Some(oldest.to_string()), count, false).await {
            Ok(d) => d,
            Err(e) => return Err(e),
        };
        all_dates.extend(dates);
    }
    all_dates.sort();
    all_dates.dedup();
    Ok(all_dates)
}
