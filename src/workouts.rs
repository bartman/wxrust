use crate::api;
use crate::auth;
use crate::formatters;
use crate::models;
use chrono::{Datelike, Utc};

pub async fn get_day(token: &str, date: &str) -> Result<String, String> {
    let claims = auth::decode_token(&token).map_err(|e| e.to_string())?;
    let uid = claims.id;

    let query = format!(r#"
query {{
  jday(uid: {}, ymd: "{}") {{
    log
    bw
    eblocks {{
      eid
      sets {{
        w
        r
        s
        lb
        rpe
        pr
        est1rm
        eff
        int
        type
        t
        d
        dunit
        speed
        force
        c
      }}
    }}
    exercises {{
      exercise {{
        id
        name
        type
      }}
    }}
  }}
}}
"#, uid, date);

    let response: models::GraphQLResponse<models::WorkoutData> = api::graphql_request(token, &query, None).await.map_err(|e| e.to_string())?;

    if let Some(errors) = response.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
    }

    if let Some(data) = response.data {
        if let Some(jday) = data.jday {
            Ok(formatters::format_workout(&jday))
        } else {
            Err("No workout found for the date.".to_string())
        }
    } else {
        Err("Unexpected response.".to_string())
    }
}

pub async fn get_all_dates(token: &str) -> Result<Vec<String>, String> {
    let claims = auth::decode_token(&token).map_err(|e| e.to_string())?;
    let uid = claims.id;

    let today = Utc::now().date_naive();
    let to = format!("{:04}{:02}{:02}", today.year(), today.month(), today.day());
    let from = "20000101".to_string();

    let query = r#"
query GetCalendarDays($uid: ID!, $from: YYYYMMDD!, $to: YYYYMMDD!) {
  getCalendarDays(uid: $uid, from: $from, to: $to)
}
"#;

    let variables = serde_json::json!({ "uid": uid, "from": from, "to": to });

    let response: models::GraphQLResponse<models::GetCalendarDaysData> = api::graphql_request(token, query, Some(variables)).await.map_err(|e| e.to_string())?;

    if let Some(errors) = response.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
    }

    if let Some(data) = response.data {
        if let Some(dates) = data.get_calendar_days {
            let date_strings: Vec<String> = dates.into_iter().map(|d| {
                let s = d.to_string();
                // Assuming 8 digits: YYYYMMDD
                format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
            }).collect();
            Ok(date_strings)
        } else {
            Ok(vec![])
        }
    } else {
        Err("Unexpected response.".to_string())
    }
}