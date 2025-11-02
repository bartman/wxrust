use async_trait::async_trait;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json;
use ansi_term::Colour;
use tokio::sync::OnceCell;

use crate::models::{GraphQLRequest, GraphQLResponse, WorkoutRequest, WorkoutResponse, UserBasicInfoData, User};
use crate::formatters::STDERR_COLOR_ENABLED;

#[cfg_attr(tarpaulin, ignore)]
#[async_trait]
pub trait ApiClient: Send + Sync {
    async fn login_request(&self, request: &GraphQLRequest) -> Result<GraphQLResponse<crate::models::LoginData>, Box<dyn std::error::Error>>;
    async fn graphql_request<T: DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>>;
    async fn get_user_info(&self, token: &str) -> Result<crate::models::User, Box<dyn std::error::Error>>;
}

#[derive(Clone)]
pub struct ReqwestClient {
    client: Client,
    verbose: bool,
    user_info: OnceCell<crate::models::User>,
}

impl ReqwestClient {
    pub fn new_with_verbose(verbose: bool) -> Self {
        ReqwestClient {
            client: Client::new(),
            verbose,
            user_info: OnceCell::new(),
        }
    }
}

#[cfg_attr(tarpaulin, ignore)]
#[async_trait]
impl ApiClient for ReqwestClient {
    async fn login_request(&self, request: &GraphQLRequest) -> Result<GraphQLResponse<crate::models::LoginData>, Box<dyn std::error::Error>> {
        if self.verbose {
            let mut output = format!("Query:\n{}", request.query);
            output += &format!("\nVariables: {}", serde_json::to_string_pretty(&request.variables).unwrap_or("Failed".to_string()));
            let colored = if *STDERR_COLOR_ENABLED {
                Colour::Blue.paint(output).to_string()
            } else {
                output
            };
            eprintln!("{}", colored);
        }
        let response = self.client
            .post("https://weightxreps.net/api/graphql")
            .json(request)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        if self.verbose {
            let colored = if status.is_success() {
                if *STDERR_COLOR_ENABLED {
                    Colour::Green.paint(&text).to_string()
                } else {
                    text.clone()
                }
            } else {
                if *STDERR_COLOR_ENABLED {
                    Colour::Red.paint(&text).to_string()
                } else {
                    text.clone()
                }
            };
            eprintln!("{}", colored);
        }
        let body: GraphQLResponse<crate::models::LoginData> = serde_json::from_str(&text)?;
        Ok(body)
    }

    async fn graphql_request<T: DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>> {
        if self.verbose {
            let mut output = format!("Query:\n{}", query);
            if let Some(vars) = &variables {
                output += &format!("\nVariables: {}", serde_json::to_string_pretty(vars).unwrap_or("Failed".to_string()));
            }
            let colored = if *STDERR_COLOR_ENABLED {
                Colour::Blue.paint(output).to_string()
            } else {
                output
            };
            eprintln!("{}", colored);
        }
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
        if self.verbose {
            let colored = if status.is_success() {
                if *STDERR_COLOR_ENABLED {
                    Colour::Green.paint(&text).to_string()
                } else {
                    text.clone()
                }
            } else {
                if *STDERR_COLOR_ENABLED {
                    Colour::Red.paint(&text).to_string()
                } else {
                    text.clone()
                }
            };
            eprintln!("{}", colored);
        }
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
            if let Some(data) = response.data {
                if let Some(session) = data.get_session {
                    Ok(session.user)
                } else {
                    // Default to kg if not available
                    Ok(User { usekg: Some(1) })
                }
            } else {
                Err("No data in response".into())
            }
        }).await?;
        Ok(user.clone())
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
pub async fn workout_request(client: &Client, token: &str, request: &WorkoutRequest) -> Result<WorkoutResponse, Box<dyn std::error::Error>> {
    let response = client
        .post("https://weightxreps.net/api/graphql")
        .header("Authorization", format!("Bearer {}", token))
        .json(request)
        .send()
        .await?;
    let body: WorkoutResponse = response.json().await?;
    Ok(body)
}