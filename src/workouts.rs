use crate::api;
use crate::auth;
use crate::formatters;
use crate::models;
use chrono::{Datelike, Utc};


pub async fn get_jday<C: crate::api::ApiClient>(client: &C, token: &str, date: &str) -> Result<models::JDay, String> {
    let claims = auth::decode_token(&token).map_err(|e| e.to_string())?;
    let uid = claims.id;

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
"#, uid, date);

    let response: models::GraphQLResponse<models::WorkoutData> = api::graphql_request(client, token, &query, None).await.map_err(|e| e.to_string())?;

    if let Some(errors) = response.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
    }

    if let Some(data) = response.data {
        if let Some(jday) = data.jday {
            Ok(jday)
        } else {
            Err("No workout found for the date.".to_string())
        }
    } else {
        Err("Unexpected response.".to_string())
    }
}

pub async fn get_day<C: crate::api::ApiClient>(client: &C, token: &str, date: &str) -> Result<String, String> {
    let jday = get_jday(client, token, date).await?;
    let formatted = formatters::format_workout(&jday);
    let bw = jday.bw.unwrap_or(0.0);
    let output = format!("{}\n@ {} bw\n{}", formatters::color_date(date), formatters::color_bw(&format!("{:.0}", bw)), formatted);
    Ok(output)
}

pub async fn get_dates<C: crate::api::ApiClient>(client: &C, token: &str, from: Option<String>, count: u32, reverse: bool) -> Result<Vec<String>, String> {
    let claims = auth::decode_token(&token).map_err(|e| e.to_string())?;
    let uid = claims.id;

    let initial_ymd = from.clone().unwrap_or_else(|| {
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
    let mut remaining = count;

    while remaining > 0 {
        let batch_size = std::cmp::min(32, remaining);
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
        all_dates.extend(date_strings.clone());

        // Set next ymd to the oldest in this batch to get older dates
        if let Some(oldest) = date_strings.first() {
            current_ymd = oldest.clone();
        } else {
            break;
        }

        remaining -= batch_size;
    }

    // Remove duplicates and sort
    all_dates.sort();
    all_dates.dedup();

    let selected: Vec<String> = if from.is_none() {
        // most recent count
        all_dates.into_iter().rev().take(count as usize).collect()
    } else {
        // count before from
        let filtered: Vec<String> = all_dates.into_iter().filter(|d| d <= &initial_ymd).collect();
        filtered.into_iter().rev().take(count as usize).collect()
    };

    let mut result = selected;
    result.sort();
    if reverse {
        result.reverse();
    }
    Ok(result)
}
