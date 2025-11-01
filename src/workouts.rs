use crate::api;
use crate::auth;
use crate::formatters;
use crate::models;

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

    let response: models::GraphQLResponse<models::WorkoutData> = api::graphql_request(token, &query).await.map_err(|e| e.to_string())?;

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