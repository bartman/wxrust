use mockall::mock;
use wxrust::api::{login_request, graphql_request};
use wxrust::models::{GraphQLRequest, GraphQLResponse, LoginData, LoginVariables, User};

mock! {
    #[derive(Clone)]
    ApiClient {}

    #[async_trait::async_trait]
    impl wxrust::api::ApiClient for ApiClient {
        async fn login_request(&self, request: &GraphQLRequest) -> Result<GraphQLResponse<LoginData>, Box<dyn std::error::Error>>;
        async fn graphql_request<T: serde::de::DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>>;
        async fn get_user_info(&self, token: &str) -> Result<User, Box<dyn std::error::Error>>;
    }
}



#[tokio::test]
async fn test_login_request_free() {
    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_login_request()
        .times(1)
        .returning(|_| Ok(GraphQLResponse {
            data: Some(LoginData { login: "token".to_string() }),
            errors: None,
        }));

    let request = GraphQLRequest {
        query: "mutation".to_string(),
        variables: LoginVariables { u: "user".to_string(), p: "pass".to_string() },
    };

    let result = login_request(&mock_client, &request).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.data.unwrap().login, "token");
}

#[tokio::test]
async fn test_graphql_request_free() {
    let mut mock_client = MockApiClient::new();
    mock_client
        .expect_graphql_request::<serde_json::Value>()
        .times(1)
        .returning(|_, _, _| Ok(GraphQLResponse {
            data: Some(serde_json::json!({"test": "data"})),
            errors: None,
        }));

    let result = graphql_request(&mock_client, "token", "query", None).await;
    assert!(result.is_ok());
    let response: GraphQLResponse<serde_json::Value> = result.unwrap();
    assert_eq!(response.data.unwrap()["test"], "data");
}