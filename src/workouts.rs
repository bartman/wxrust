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
    let ymd = format!("{:04}-{:02}-{:02}", today.year(), today.month(), today.day());

    let query = r#"
query GetJRange($uid: ID!, $ymd: YMD!, $range: Int!) {
  jrange(uid: $uid, ymd: $ymd, range: $range) {
    days {
      on
    }
  }
}
"#;

    let variables = serde_json::json!({ "uid": uid.to_string(), "ymd": ymd, "range": 32 });

    let response: models::GraphQLResponse<models::GetJRangeData> = api::graphql_request(token, query, Some(variables)).await.map_err(|e| e.to_string())?;

    if let Some(errors) = response.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
    }

    if let Some(data) = response.data {
        if let Some(jrange) = data.jrange {
            if let Some(days) = jrange.days {
                let mut date_strings: Vec<String> = days.into_iter()
                    .filter_map(|day| day.on)
                    .map(|d| format!("{}-{}-{}", &d[0..4], &d[5..7], &d[8..10]))
                    .collect();
                date_strings.sort();
                Ok(date_strings)
            } else {
                Ok(vec![])
            }
        } else {
            Ok(vec![])
        }
    } else {
        Err("Unexpected response.".to_string())
    }
}
