use mockall::mock;
use tempfile::TempDir;
use lazy_static::lazy_static;
use tokio::sync::Mutex;
use wxrust::models::{GraphQLResponse, JDay, EBlock, ExerciseWrapper, Exercise, Set, User};
use wxrust::workouts::{forget_cached_user_wants_kg, format_cached_jday_text, read_cached_jday_text};
use wxrust::parsers::parse_workout;
use wxrust::formatters::format_workout_for_cache;
use std::fs;

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

fn sample_jday() -> JDay {
    JDay {
        log: "EBLOCK:ex1".to_string(),
        bw: Some(80.0),
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
    }
}

fn restore_xdg(original: Result<String, std::env::VarError>) {
    if let Ok(original) = original {
        unsafe { std::env::set_var("XDG_CACHE_HOME", original); }
    } else {
        unsafe { std::env::remove_var("XDG_CACHE_HOME"); }
    }
}

#[tokio::test]
async fn test_fetch_command_skips_cached() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }
    forget_cached_user_wants_kg();

    let cache_dir = temp_dir.path().join("wxrust").join("123");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(
        cache_dir.join("2023-10-01.txt"),
        "2023-10-01\n@ 80 kg bw\n#Squat\n135 x 5\n",
    ).unwrap();

    let mock_client = MockApiClient::new();
    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: None,
        uid: Some(123),
        use_network: false,
        use_cache: true,
        write_cache: false,
    };

    let dates = vec!["2023-10-01".to_string()];
    let result = wxrust::fetch::fetch_command(&data_access, &dates, false, false, None, false, false).await;
    assert!(result.is_ok());

    restore_xdg(original_xdg_cache);
}

#[tokio::test]
async fn test_fetch_command_fetches_and_caches() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }
    forget_cached_user_wants_kg();

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
    mock_client
        .expect_graphql_request::<wxrust::models::BatchJDayData>()
        .times(1)
        .returning(|_, _, _| {
            let mut data = std::collections::HashMap::new();
            data.insert("d0".to_string(), Some(sample_jday()));
            Ok(GraphQLResponse {
                data: Some(data),
                errors: None,
            })
        });

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some("token"),
        uid: Some(123),
        use_network: true,
        use_cache: false,
        write_cache: false,
    };

    let dates = vec!["2023-10-01".to_string()];
    let result = wxrust::fetch::fetch_command(&data_access, &dates, false, false, None, false, false).await;
    assert!(result.is_ok());

    let cache_path = temp_dir.path().join("wxrust").join("123").join("2023-10-01.txt");
    assert!(cache_path.exists(), "fetch should write cache even if write_cache is false");

    restore_xdg(original_xdg_cache);
}

#[tokio::test]
async fn test_fetch_command_no_dates() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }
    forget_cached_user_wants_kg();

    let mock_client = MockApiClient::new();
    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: None,
        uid: Some(123),
        use_network: false,
        use_cache: true,
        write_cache: false,
    };

    let result = wxrust::fetch::fetch_command(&data_access, &["2023-10-01".to_string()], false, false, None, false, false).await;
    assert!(result.is_ok());

    restore_xdg(original_xdg_cache);
}

#[tokio::test]
async fn test_fetch_command_force_refetches() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }
    forget_cached_user_wants_kg();

    let cache_dir = temp_dir.path().join("wxrust").join("123");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(
        cache_dir.join("2023-10-01.txt"),
        "2023-10-01\n@ 80 kg bw\n#Squat\n100 x 5\n",
    ).unwrap();

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
    mock_client
        .expect_graphql_request::<wxrust::models::BatchJDayData>()
        .times(1)
        .returning(|_, _, _| {
            let mut data = std::collections::HashMap::new();
            data.insert("d0".to_string(), Some(sample_jday()));
            Ok(GraphQLResponse {
                data: Some(data),
                errors: None,
            })
        });

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some("token"),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: false,
    };

    let dates = vec!["2023-10-01".to_string()];
    let result = wxrust::fetch::fetch_command(&data_access, &dates, false, true, None, false, false).await;
    assert!(result.is_ok());

    restore_xdg(original_xdg_cache);
}

#[tokio::test]
async fn test_fetch_command_without_force_skips_network() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }
    forget_cached_user_wants_kg();

    let cache_dir = temp_dir.path().join("wxrust").join("123");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(
        cache_dir.join("2023-10-01.txt"),
        "2023-10-01\n@ 80 kg bw\n#Squat\n100 x 5\n",
    ).unwrap();

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
    mock_client
        .expect_graphql_request::<wxrust::models::BatchJDayData>()
        .times(0);

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some("token"),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: false,
    };

    let dates = vec!["2023-10-01".to_string()];
    let result = wxrust::fetch::fetch_command(&data_access, &dates, false, false, None, false, false).await;
    assert!(result.is_ok());

    restore_xdg(original_xdg_cache);
}

#[test]
fn test_format_text_diff_identical_is_none() {
    let text = "2023-10-01\n@ 80.0000 kg bw\n#Squat\n135 kg x 5\n";
    assert!(wxrust::fetch::format_text_diff("2023-10-01", text, text).is_none());
}

#[test]
fn test_format_text_diff_detects_change() {
    let local = "2023-10-01\n@ 80.0000 kg bw\n#Squat\n135 kg x 5\n";
    let server = "2023-10-01\n@ 80.0000 kg bw\n#Squat\n140 kg x 5\n";
    let diff = wxrust::fetch::format_text_diff("2023-10-01", local, server).unwrap();
    assert!(diff.contains("Diff for 2023-10-01:"));
    assert!(diff.contains("-135 kg x 5"));
    assert!(diff.contains("+140 kg x 5"));
}

#[tokio::test]
async fn test_format_text_diff_ignores_parse_roundtrip_newline() {
    // Server logs typically omit a trailing newline. The parser always adds one
    // to `log`, so format(parse(cache)) != format(server) even when the cache
    // file is exactly what a fetch would write.
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }
    forget_cached_user_wants_kg();
    let jday = sample_jday();
    let cache_text = format_cached_jday_text("2023-10-01", &jday);
    let parsed = parse_workout(&cache_text).unwrap();
    let roundtrip = format_workout_for_cache("2023-10-01", &parsed);
    let server_fmt = format_workout_for_cache("2023-10-01", &jday);
    assert_ne!(
        roundtrip, server_fmt,
        "parse/reformat is not a round-trip; --diff must not use it"
    );
    assert!(wxrust::fetch::format_text_diff("2023-10-01", &cache_text, &cache_text).is_none());
    restore_xdg(original_xdg_cache);
}

#[tokio::test]
async fn test_fetch_diff_identical_cache_is_ok() {
    let _guard = ENV_MUTEX.lock().await;
    let temp_dir = TempDir::new().unwrap();
    let original_xdg_cache = std::env::var("XDG_CACHE_HOME");
    unsafe { std::env::set_var("XDG_CACHE_HOME", temp_dir.path()); }
    forget_cached_user_wants_kg();

    let jday = sample_jday();
    let cache_dir = temp_dir.path().join("wxrust").join("123");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(
        cache_dir.join("2023-10-01.txt"),
        format_cached_jday_text("2023-10-01", &jday),
    ).unwrap();

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
    mock_client
        .expect_graphql_request::<wxrust::models::BatchJDayData>()
        .times(1)
        .returning(|_, _, _| {
            let mut data = std::collections::HashMap::new();
            data.insert("d0".to_string(), Some(sample_jday()));
            Ok(GraphQLResponse {
                data: Some(data),
                errors: None,
            })
        });
    mock_client
        .expect_user_wants_kg()
        .returning(|_| true);

    let data_access = wxrust::api::DataAccess {
        client: &mock_client,
        token: Some("token"),
        uid: Some(123),
        use_network: true,
        use_cache: true,
        write_cache: false,
    };

    let dates = vec!["2023-10-01".to_string()];
    let result = wxrust::fetch::fetch_command(&data_access, &dates, true, false, None, false, false).await;
    assert!(result.is_ok());
    assert_eq!(
        read_cached_jday_text(123, "2023-10-01").as_deref(),
        Some(format_cached_jday_text("2023-10-01", &sample_jday()).as_str())
    );

    restore_xdg(original_xdg_cache);
}

#[test]
fn test_format_transfer_stats() {
    assert_eq!(
        wxrust::fetch::format_transfer_stats(152, 850_000, 1.15),
        "152 workouts, 1.15 seconds, 132.2 T/s, 0.74 MB/s"
    );
    assert_eq!(
        wxrust::fetch::format_transfer_stats(115, 1_150_000, 1.15),
        "115 workouts, 1.15 seconds, 100.0 T/s, 1.00 MB/s"
    );
    assert_eq!(
        wxrust::fetch::format_transfer_stats(10, 500_000, 0.0),
        "10 workouts, 0.00 seconds, 0.0 T/s, 0.00 MB/s"
    );
    assert_eq!(
        wxrust::fetch::format_transfer_stats(0, 0, 1.0),
        "0 workouts, 1.00 seconds, 0.0 T/s, 0.00 MB/s"
    );
}
