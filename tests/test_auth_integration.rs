use mockall::mock;
use std::fs;
use tempfile::TempDir;
use wxrust::auth::login;
use wxrust::models::{GraphQLResponse, LoginData, User};
use base64::{Engine, engine::general_purpose};

mock! {
    #[derive(Clone)]
    ApiClient {}

    #[async_trait::async_trait]
    impl wxrust::api::ApiClient for ApiClient {
        async fn login_request(&self, request: &wxrust::models::GraphQLRequest) -> Result<wxrust::models::GraphQLResponse<wxrust::models::LoginData>, Box<dyn std::error::Error>>;
        async fn graphql_request<T: serde::de::DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<wxrust::models::GraphQLResponse<T>, Box<dyn std::error::Error>>;
        async fn get_user_info(&self, token: &str) -> Result<User, Box<dyn std::error::Error>>;
    }
}

#[tokio::test]
async fn test_login_success() {
    // Create a valid JWT token: header {"alg":"HS256","typ":"JWT"}, payload {"id":123,"exp":2000000000}
    let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"id":123,"exp":2000000000}"#.as_bytes());
    let token = format!("{}.{}.{}", header, payload, "signature");

    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_login_request()
        .times(1)
        .returning(move |_| {
            Ok(GraphQLResponse {
                data: Some(LoginData { login: token.clone() }),
                errors: None,
            })
        });

    let temp_dir = TempDir::new().unwrap();
    let credentials_path = temp_dir.path().join("credentials.txt");
    fs::write(&credentials_path, "email@example.com\npassword").unwrap();

    let token_path = temp_dir.path().join("token");

    let result = login(&mock_client, &credentials_path.to_string_lossy(), &token_path.to_string_lossy()).await;
    assert!(result.is_ok());
    let returned_token = result.unwrap();
    assert!(returned_token.starts_with(&header));

    // Check token file was written
    assert!(token_path.exists());
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_login_request()
        .times(1)
        .returning(|_| {
            Ok(GraphQLResponse {
                data: None,
                errors: Some(vec![wxrust::models::GraphQLError { message: "Invalid credentials".to_string() }]),
            })
        });

    let temp_dir = TempDir::new().unwrap();
    let credentials_path = temp_dir.path().join("credentials.txt");
    fs::write(&credentials_path, "email@example.com\npassword").unwrap();

    let token_path = temp_dir.path().join("token");

    let result = login(&mock_client, &credentials_path.to_string_lossy(), &token_path.to_string_lossy()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid credentials"));
}