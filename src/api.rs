use async_trait::async_trait;
use serde::de::DeserializeOwned;
use ansi_term::Colour;
use tokio::sync::OnceCell;

use crate::models::{GraphQLRequest, GraphQLResponse, WorkoutRequest, WorkoutResponse, UserBasicInfoData, User};
use crate::formatters::STDERR_COLOR_ENABLED;
use crate::workouts::{write_cached_user_wants_kg, read_cached_user_wants_kg};

#[cfg_attr(tarpaulin, ignore)]
#[async_trait]
pub trait ApiClient: Send + Sync {
    async fn login_request(&self, request: &GraphQLRequest) -> Result<GraphQLResponse<crate::models::LoginData>, Box<dyn std::error::Error>>;
    async fn graphql_request<T: DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>>;
    async fn get_user_info(&self, token: &str) -> Result<crate::models::User, Box<dyn std::error::Error>>;
    async fn user_wants_kg(&self, token: &str) -> bool;
}

fn log_verbose_request(query: &str, variables: Option<&serde_json::Value>, verbose: bool) {
    if verbose {
        let mut output = format!("Query:\n{}", query);
        if let Some(vars) = variables {
            output += &format!("\nVariables: {}", serde_json::to_string_pretty(vars).unwrap_or("Failed".to_string()));
        }
        let colored = if *STDERR_COLOR_ENABLED {
            Colour::Blue.paint(output).to_string()
        } else {
            output
        };
        eprintln!("{}", colored);
    }
}

fn log_verbose_response(text: &str, status: reqwest::StatusCode, verbose: bool) {
    if verbose {
        let colored = if status.is_success() {
            if *STDERR_COLOR_ENABLED {
                Colour::Green.paint(text).to_string()
            } else {
                text.to_string()
            }
        } else if *STDERR_COLOR_ENABLED {
            Colour::Red.paint(text).to_string()
        } else {
            text.to_string()
        };
        eprintln!("{}", colored);
    }
}

pub struct DataAccess<'a, C: ApiClient> {
    pub client: &'a C,
    pub token: Option<&'a str>,
    pub uid: Option<u32>,
    pub use_network: bool,
    pub use_cache: bool,
    pub write_cache: bool,
}

#[derive(Clone)]
pub struct ReqwestClient {
    client: reqwest::Client,
    verbose: bool,
    user_info: OnceCell<crate::models::User>,
}

impl ReqwestClient {
    pub fn new_with_verbose(verbose: bool) -> Self {
        ReqwestClient {
            client: reqwest::Client::builder()
                .pool_max_idle_per_host(32)
                .tcp_nodelay(true)
                .build()
                .expect("failed to build HTTP client"),
            verbose,
            user_info: OnceCell::new(),
        }
    }
}

#[cfg_attr(tarpaulin, ignore)]
#[async_trait]
impl ApiClient for ReqwestClient {
    async fn login_request(&self, request: &GraphQLRequest) -> Result<GraphQLResponse<crate::models::LoginData>, Box<dyn std::error::Error>> {
        log_verbose_request(&request.query, Some(&serde_json::to_value(&request.variables).unwrap()), self.verbose);
        let response = self.client
            .post("https://weightxreps.net/api/graphql")
            .json(request)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        log_verbose_response(&text, status, self.verbose);
        let body: GraphQLResponse<crate::models::LoginData> = serde_json::from_str(&text)?;
        Ok(body)
    }

    async fn graphql_request<T: DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>> {
        log_verbose_request(query, variables.as_ref(), self.verbose);
        let request_body = if let Some(vars) = variables {
            serde_json::json!({ "query": query, "variables": vars })
        } else {
            serde_json::json!({ "query": query })
        };
        let response = self.client
            .post("https://weightxreps.net/api/graphql")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_body)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        log_verbose_response(&text, status, self.verbose);
        let body: GraphQLResponse<T> = serde_json::from_str(&text)?;
        Ok(body)
    }

    async fn get_user_info(&self, token: &str) -> Result<crate::models::User, Box<dyn std::error::Error>> {
        let user =         self.user_info.get_or_try_init(|| async {
            let query = r#"
            query {
                getSession {
                    user {
                        usekg
                    }
                }
            }
            "#;
            let response: GraphQLResponse<UserBasicInfoData> = self.graphql_request(token, query, None).await?;
            if let Some(errors) = response.errors {
                return Err::<User, Box<dyn std::error::Error>>(format!("GraphQL errors: {:?}", errors).into());
            }
            // Default to kg if not available
            if let Some(data) = response.data {
                let mut usekg = 1;
                if let Some(session) = data.get_session
                    && let Some(val) = session.user.usekg {
                        write_cached_user_wants_kg(val != 0);
                        usekg = val;
                    }
                Ok(User { usekg: Some(usekg) })
            } else {
                Err("No data in response".into())
            }
        }).await?;
        Ok(user.clone())
    }

    async fn user_wants_kg(&self, token: &str) -> bool {
        if let Some(val) = read_cached_user_wants_kg() {
            return val;
        }
        let user = self.get_user_info(token).await;
        match user {
            Ok(ref u) => return u.usekg.unwrap_or(1) == 1,
            Err(_) => return false
        }
    }
}

#[cfg_attr(tarpaulin, ignore)]
pub async fn login_request<C: ApiClient>(client: &C, request: &GraphQLRequest) -> Result<GraphQLResponse<crate::models::LoginData>, Box<dyn std::error::Error>> {
    client.login_request(request).await
}

#[cfg_attr(tarpaulin, ignore)]
pub async fn graphql_request<T: DeserializeOwned + 'static, C: ApiClient>(client: &C, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>> {
    client.graphql_request(token, query, variables).await
}

#[cfg_attr(tarpaulin, ignore)]
#[allow(dead_code)]
pub async fn workout_request(client: &reqwest::Client, token: &str, request: &WorkoutRequest) -> Result<WorkoutResponse, Box<dyn std::error::Error>> {
    let response = client
        .post("https://weightxreps.net/api/graphql")
        .header("Authorization", format!("Bearer {}", token))
        .json(request)
        .send()
        .await?;
    let body: WorkoutResponse = response.json().await?;
    Ok(body)
}
