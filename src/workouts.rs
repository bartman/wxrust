use reqwest::Client;

use crate::api;
use crate::auth;
use crate::formatters;
use crate::models;

pub async fn get_day(token: &str, date: &str) -> Result<String, String> {
    let claims = auth::decode_token(&token).map_err(|e| e.to_string())?;
    let uid = claims.id;

    let workout_request = models::WorkoutRequest {
        query: r#"
query JDay($uid: ID!, $ymd: YMD) {
  jday(uid: $uid, ymd: $ymd) {
    log
    bw
    eblocks {
      eid
      sets {
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
      }
    }
    exercises {
      exercise {
        id
        name
        type
      }
    }
  }
}
        "#.to_string(),
        variables: models::WorkoutVariables { uid, ymd: Some(date.to_string()) },
    };

    let client = Client::new();
    let workout_body = api::workout_request(&client, &token, &workout_request).await.map_err(|e| e.to_string())?;

    if let Some(errors) = workout_body.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "));
    }

    if let Some(data) = workout_body.data {
        if let Some(jday) = data.jday {
            Ok(formatters::format_full_workout(&jday, date))
        } else {
            Err("No workout found for the date.".to_string())
        }
    } else {
        Err("Unexpected response.".to_string())
    }
}