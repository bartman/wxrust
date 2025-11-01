mod models;
mod formatters;
mod auth;
mod api;

use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = match auth::login("credentials.txt").await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("Login successful. Token: {}", token);
    // Decode token to get uid
    let claims = auth::decode_token(&token)?;
    let uid = claims.id;
    println!("User ID: {}", uid);

    // Now retrieve latest workout
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
        variables: models::WorkoutVariables { uid, ymd: Some("2025-10-31".to_string()) },
    };

    let client = Client::new();
    let workout_body = api::workout_request(&client, &token, &workout_request).await?;
    if let Some(errors) = workout_body.errors {
        for error in errors {
            eprintln!("GraphQL Error: {}", error.message);
        }
    } else if let Some(data) = workout_body.data {
        if let Some(jday) = data.jday {
            println!("Full formatted workout:\n{}", formatters::format_full_workout(&jday, "2025-10-31"));
        } else {
            println!("No workout found for the date.");
        }
    } else {
        println!("Unexpected response.");
    }

    Ok(())
}