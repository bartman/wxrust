use mockall::mock;
use wxrust::workouts::{get_jday, get_day, get_dates};
use wxrust::models::{GraphQLResponse, WorkoutData, JDay, EBlock, ExerciseWrapper, Exercise, Set};
use base64::{Engine, engine::general_purpose};

mock! {
    #[derive(Clone)]
    ApiClient {}

    #[async_trait::async_trait]
    impl wxrust::api::ApiClient for ApiClient {
        async fn login_request(&self, request: &wxrust::models::GraphQLRequest) -> Result<wxrust::models::GraphQLResponse<wxrust::models::LoginData>, Box<dyn std::error::Error>>;
        async fn graphql_request<T: serde::de::DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<wxrust::models::GraphQLResponse<T>, Box<dyn std::error::Error>>;
    }
}

#[tokio::test]
async fn test_get_jday_success() {
    // Create a valid JWT token
    let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"id":123,"exp":2000000000}"#.as_bytes());
    let token = format!("{}.{}.{}", header, payload, "signature");

    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_graphql_request::<wxrust::models::WorkoutData>()
        .times(1)
        .returning(move |_, _, _| {
            Ok(GraphQLResponse {
                data: Some(WorkoutData {
                    jday: Some(JDay {
                        log: "Date: 2023-10-01".to_string(),
                        bw: Some(180.0),
                        eblocks: vec![EBlock {
                            eid: "ex1".to_string(),
                            sets: vec![Set {
                                w: Some(135.0),
                                r: Some(5),
                                s: Some(1),
                                lb: Some(0.0),
                                rpe: None,
                                ..Default::default()
                            }],
                        }],
                        exercises: vec![ExerciseWrapper {
                            exercise: Exercise {
                                id: "ex1".to_string(),
                                name: "Squat".to_string(),
                                ex_type: Some("strength".to_string()),
                            },
                        }],
                    }),
                }),
                errors: None,
            })
        });

    let result = get_jday(&mock_client, &token, "2023-10-01").await;
    assert!(result.is_ok());
    let jday = result.unwrap();
    assert_eq!(jday.log, "Date: 2023-10-01");
    assert_eq!(jday.bw, Some(180.0));
}

#[tokio::test]
async fn test_get_jday_no_workout() {
    let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"id":123,"exp":2000000000}"#.as_bytes());
    let token = format!("{}.{}.{}", header, payload, "signature");

    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_graphql_request::<wxrust::models::WorkoutData>()
        .times(1)
        .returning(|_, _, _| {
            Ok(GraphQLResponse {
                data: Some(WorkoutData { jday: None }),
                errors: None,
            })
        });

    let result = get_jday(&mock_client, &token, "2023-10-01").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No workout found"));
}

#[tokio::test]
async fn test_get_dates_success() {
    let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"id":123,"exp":2000000000}"#.as_bytes());
    let token = format!("{}.{}.{}", header, payload, "signature");

    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_graphql_request::<wxrust::models::GetJRangeData>()
        .times(1)
        .returning(|_, _, _| {
            Ok(GraphQLResponse {
                data: Some(wxrust::models::GetJRangeData {
                    jrange: Some(wxrust::models::JRangeData {
                        days: Some(vec![
                            wxrust::models::JRangeDayData { on: Some("2023-10-01".to_string()) },
                            wxrust::models::JRangeDayData { on: Some("2023-10-02".to_string()) },
                        ]),
                    }),
                }),
                errors: None,
            })
        });

    let result = get_dates(&mock_client, &token, None, None, 2, false).await;
    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates, vec!["2023-10-01", "2023-10-02"]);
}

#[tokio::test]
async fn test_get_dates_invalid_token() {
    let mock_client = MockApiClient::new();
    // No expectations needed

    let result = get_dates(&mock_client, "invalid_token", None, None, 2, false).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid token format"));
}

#[tokio::test]
async fn test_get_jday_invalid_token() {
    let mock_client = MockApiClient::new();
    // No need to set expectations since decode_token will fail first

    let result = get_jday(&mock_client, "invalid_token", "2023-10-01").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid token format"));
}

#[tokio::test]
async fn test_get_jday_graphql_error() {
    let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"id":123,"exp":2000000000}"#.as_bytes());
    let token = format!("{}.{}.{}", header, payload, "signature");

    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_graphql_request::<wxrust::models::WorkoutData>()
        .times(1)
        .returning(|_, _, _| {
            Ok(GraphQLResponse {
                data: None,
                errors: Some(vec![wxrust::models::GraphQLError { message: "GraphQL error".to_string() }]),
            })
        });

    let result = get_jday(&mock_client, &token, "2023-10-01").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("GraphQL error"));
}

#[tokio::test]
async fn test_get_day_success() {
    let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"id":123,"exp":2000000000}"#.as_bytes());
    let token = format!("{}.{}.{}", header, payload, "signature");

    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_graphql_request::<wxrust::models::WorkoutData>()
        .times(1)
        .returning(move |_, _, _| {
            Ok(GraphQLResponse {
                data: Some(WorkoutData {
                    jday: Some(JDay {
                        log: "Date: 2023-10-01\nEBLOCK:ex1\n".to_string(),
                        bw: Some(180.0),
                        eblocks: vec![EBlock {
                            eid: "ex1".to_string(),
                            sets: vec![Set {
                                w: Some(135.0),
                                r: Some(5),
                                s: Some(1),
                                lb: Some(0.0),
                                rpe: None,
                                ..Default::default()
                            }],
                        }],
                        exercises: vec![ExerciseWrapper {
                            exercise: Exercise {
                                id: "ex1".to_string(),
                                name: "Squat".to_string(),
                                ex_type: Some("strength".to_string()),
                            },
                        }],
                    }),
                }),
                errors: None,
            })
        });

    unsafe { std::env::set_var("WXRUST_COLOR", "never"); } // Disable colors for test
    let result = get_day(&mock_client, &token, "2023-10-01").await;
    assert!(result.is_ok());
    let workout = result.unwrap();
    assert!(workout.contains("Date: 2023-10-01"));
    assert!(workout.contains("#Squat"));
    assert!(workout.contains("135 x 5"));
}