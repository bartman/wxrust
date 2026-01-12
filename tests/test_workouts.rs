use mockall::mock;
use wxrust::workouts::{get_jday, get_dates, get_dates_from_cache, read_cached_user_wants_kg, read_cached_user_wants_kg_or, write_cached_user_wants_kg, forget_cached_user_wants_kg};
use wxrust::models::{GraphQLResponse, WorkoutData, JDay, EBlock, ExerciseWrapper, Exercise, Set, User};
use base64::{Engine, engine::general_purpose};
use tempfile::TempDir;
use lazy_static::lazy_static;
use tokio::sync::Mutex;
use std::fs;

// all tests in this file run sequentially to avoid clashing with global cached state;
// to do this we use an async-aware mutex and hold it in each test,
// essentially forcing single threaded execution
lazy_static! {
    static ref ENV_MUTEX: Mutex<()> = Mutex::new(());
}

mock! {
    #[derive(Clone)]
    ApiClient {}

    #[async_trait::async_trait]
    impl wxrust::api::ApiClient for ApiClient {
        async fn login_request(&self, request: &wxrust::models::GraphQLRequest) -> Result<wxrust::models::GraphQLResponse<wxrust::models::LoginData>, Box<dyn std::error::Error>>;
        async fn graphql_request<T: serde::de::DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<wxrust::models::GraphQLResponse<T>, Box<dyn std::error::Error>>;
        async fn get_user_info(&self, token: &str) -> Result<User, Box<dyn std::error::Error>>;
        async fn user_wants_kg(&self, token: &str) -> bool;
    }
}

#[tokio::test]
async fn test_get_jday_graphql_error() {
    let _guard = ENV_MUTEX.lock().await;
    // Set up test cache directory
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

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

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some(&token),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: true,
    };

    let result = get_jday(&data_access, "2023-10-01", false).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("GraphQL error"));

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_jday_success() {
    let _guard = ENV_MUTEX.lock().await;
    // Set up test cache directory
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

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
    mock_client
        .expect_user_wants_kg()
        .times(0)
        .returning(|_| true);

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some(&token),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: true,
    };

    let result = get_jday(&data_access, "2023-10-01", false).await;
    assert!(result.is_ok());
    let jday = result.unwrap();
    assert_eq!(jday.log, "Date: 2023-10-01");
    assert_eq!(jday.bw, Some(180.0));

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_jday_no_workout() {
    let _guard = ENV_MUTEX.lock().await;
    // Set up test cache directory
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

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
    mock_client
        .expect_user_wants_kg()
        .times(0) // shouldn't be called
        .returning(|_| true);

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some(&token),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: true,
    };

    let result = get_jday(&data_access, "2023-10-01", false).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No workout found"));

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_jday_invalid_token() {
    let _guard = ENV_MUTEX.lock().await;
    // Set up test cache directory
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    let token = "invalid";
    let mut mock_client = MockApiClient::new();
    // Expect the graphql call to fail with an auth error
    mock_client
        .expect_graphql_request::<wxrust::models::WorkoutData>()
        .times(1)
        .returning(|_, _, _| {
            Err("Authentication error".into())
        });

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some(&token),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: true,
    };
    let result = get_jday(&data_access, "2023-10-01", false).await;
    assert!(result.is_err());

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_dates_success() {
    let _guard = ENV_MUTEX.lock().await;
    // Set up test cache directory
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

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
                        days: Some(vec![wxrust::models::JRangeDayData {
                            on: Some("2023-10-01".to_string()),
                        }]),
                    }),
                }),
                errors: None,
            })
        });

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some(&token),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: true,
    };

    let result = get_dates(&data_access, None, None, 1, false).await;
    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates.len(), 1);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_dates_invalid_token() {
    let _guard = ENV_MUTEX.lock().await;
    // Set up test cache directory
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    let token = "invalid";
    let mut mock_client = MockApiClient::new();
    // Expect the graphql call to fail with an auth error
    mock_client
        .expect_graphql_request::<wxrust::models::GetJRangeData>()
        .times(1)
        .returning(|_, _, _| {
            Err("Authentication error".into())
        });

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some(&token),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: true,
    };
    let result = get_dates(&data_access, None, None, 1, false).await;
    assert!(result.is_err());

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_read_cached_user_wants_kg_none() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    forget_cached_user_wants_kg();

    // File doesn't exist
    assert_eq!(read_cached_user_wants_kg(), None);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_read_cached_user_wants_kg_true() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    forget_cached_user_wants_kg();

    // Create dir and file
    let wxrust_dir = temp_dir.path().join("wxrust");
    std::fs::create_dir_all(&wxrust_dir).unwrap();
    std::fs::write(wxrust_dir.join("user_wants_kg"), "1\n").unwrap();

    assert_eq!(read_cached_user_wants_kg(), Some(true));

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_read_cached_user_wants_kg_false() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    forget_cached_user_wants_kg();

    // Create dir and file
    let wxrust_dir = temp_dir.path().join("wxrust");
    std::fs::create_dir_all(&wxrust_dir).unwrap();
    std::fs::write(wxrust_dir.join("user_wants_kg"), "0\n").unwrap();

    assert_eq!(read_cached_user_wants_kg(), Some(false));

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_read_cached_user_wants_kg_invalid() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    forget_cached_user_wants_kg();

    // Create dir and file with invalid content
    let wxrust_dir = temp_dir.path().join("wxrust");
    std::fs::create_dir_all(&wxrust_dir).unwrap();
    std::fs::write(wxrust_dir.join("user_wants_kg"), "invalid\n").unwrap();

    // Should capture stderr, but for test, just check None
    assert_eq!(read_cached_user_wants_kg(), None);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_read_cached_user_wants_kg_or() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    forget_cached_user_wants_kg();

    // No file, return default
    assert_eq!(read_cached_user_wants_kg_or(true), true);
    assert_eq!(read_cached_user_wants_kg_or(false), false);

    // With file
    let wxrust_dir = temp_dir.path().join("wxrust");
    std::fs::create_dir_all(&wxrust_dir).unwrap();
    std::fs::write(wxrust_dir.join("user_wants_kg"), "0\n").unwrap();

    assert_eq!(read_cached_user_wants_kg_or(true), false);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_write_cached_user_wants_kg() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    forget_cached_user_wants_kg();

    write_cached_user_wants_kg(true);
    let file_path = temp_dir.path().join("wxrust").join("user_wants_kg");
    assert!(file_path.exists());
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "1\n");

    write_cached_user_wants_kg(false);
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "0\n");

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_dates_from_cache_empty_dir() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    // No cache directory exists
    let result = get_dates_from_cache(123, None, None, 10, false);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Vec::<String>::new());

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_dates_from_cache_with_files() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    // Create cache directory and files
    let cache_dir = temp_dir.path().join("wxrust").join("456");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(cache_dir.join("2025-01-01.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-05.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-10.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("invalid.txt"), "invalid").unwrap();
    fs::write(cache_dir.join("2025-01-20.dat"), "wrong extension").unwrap();

    let result = get_dates_from_cache(456, None, None, 0, false);
    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates, vec!["2025-01-01", "2025-01-05", "2025-01-10"]);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_dates_from_cache_with_filters() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    // Create cache directory and files
    let cache_dir = temp_dir.path().join("wxrust").join("789");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(cache_dir.join("2025-01-01.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-05.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-10.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-15.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-20.txt"), "workout data").unwrap();

    // Test with oldest filter
    let result = get_dates_from_cache(789, None, Some("2025-01-10".to_string()), 0, false);
    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates, vec!["2025-01-10", "2025-01-15", "2025-01-20"]);

    // Test with latest filter
    let result = get_dates_from_cache(789, Some("2025-01-10".to_string()), None, 0, false);
    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates, vec!["2025-01-01", "2025-01-05", "2025-01-10"]);

    // Test with both filters
    let result = get_dates_from_cache(789, Some("2025-01-15".to_string()), Some("2025-01-05".to_string()), 0, false);
    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates, vec!["2025-01-05", "2025-01-10", "2025-01-15"]);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_dates_from_cache_with_count() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    // Create cache directory and files
    let cache_dir = temp_dir.path().join("wxrust").join("999");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(cache_dir.join("2025-01-01.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-05.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-10.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-15.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-20.txt"), "workout data").unwrap();

    // Test with count limit
    let result = get_dates_from_cache(999, None, None, 3, false);
    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates, vec!["2025-01-10", "2025-01-15", "2025-01-20"]);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_dates_from_cache_with_reverse() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    // Create cache directory and files
    let cache_dir = temp_dir.path().join("wxrust").join("111");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(cache_dir.join("2025-01-01.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-05.txt"), "workout data").unwrap();
    fs::write(cache_dir.join("2025-01-10.txt"), "workout data").unwrap();

    // Test with reverse
    let result = get_dates_from_cache(111, None, None, 0, true);
    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates, vec!["2025-01-10", "2025-01-05", "2025-01-01"]);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_resolve_user_wants_kg_with_token() {
    let _guard = ENV_MUTEX.lock().await;
    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_user_wants_kg()
        .with(mockall::predicate::eq("token"))
        .times(1)
        .returning(|_| true);

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some("token"),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: true,
    };

    let result = wxrust::workouts::resolve_user_wants_kg(&data_access).await;
    assert_eq!(result, true);
}

#[tokio::test]
async fn test_resolve_user_wants_kg_without_token() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    forget_cached_user_wants_kg();
    
    // Set cache to false
    let wxrust_dir = temp_dir.path().join("wxrust");
    std::fs::create_dir_all(&wxrust_dir).unwrap();
    std::fs::write(wxrust_dir.join("user_wants_kg"), "0\n").unwrap();

    let mut mock_client = MockApiClient::new();
    mock_client.expect_user_wants_kg().times(0);

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: None,
        uid: Some(123),
        use_network: false,
        use_cache: true,
        write_cache: false,
    };

    let result = wxrust::workouts::resolve_user_wants_kg(&data_access).await;
    assert_eq!(result, false);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_get_dates_from_ranges() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }

    // Setup cache
    let cache_dir = temp_dir.path().join("wxrust").join("123");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(cache_dir.join("2023-10-01.txt"), "").unwrap();
    fs::write(cache_dir.join("2023-10-02.txt"), "").unwrap();
    fs::write(cache_dir.join("2023-10-05.txt"), "").unwrap();

    let mock_client = MockApiClient::new();
    // No network calls expected as we use cache

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: None,
        uid: Some(123),
        use_network: false, // Force cache usage
        use_cache: true,
        write_cache: false,
    };

    let ranges = vec!["2023-10-01..2023-10-02".to_string(), "2023-10-05".to_string()];
    let result = wxrust::workouts::get_dates_from_ranges(&data_access, &ranges).await;

    assert!(result.is_ok());
    let dates = result.unwrap();
    assert_eq!(dates, vec!["2023-10-01", "2023-10-02", "2023-10-05"]);

    // Restore original XDG_CACHE_HOME
    if let Ok(original) = original_xdg_cache {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}
