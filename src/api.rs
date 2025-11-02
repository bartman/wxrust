use async_trait::async_trait;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json;
use ansi_term::Colour;

use crate::models::{GraphQLRequest, GraphQLResponse, WorkoutRequest, WorkoutResponse};
use crate::formatters::STDERR_COLOR_ENABLED;

#[async_trait]
pub trait ApiClient: Send + Sync {
    async fn login_request(&self, request: &GraphQLRequest) -> Result<GraphQLResponse<crate::models::LoginData>, Box<dyn std::error::Error>>;
    async fn graphql_request<T: DeserializeOwned + 'static>(&self, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>>;
}

pub struct ReqwestClient {
    client: Client,
    verbose: bool,
}

impl ReqwestClient {
    pub fn new_with_verbose(verbose: bool) -> Self {
        ReqwestClient {
            client: Client::new(),
            verbose,
        }
    }
}

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
}

pub async fn login_request<C: ApiClient>(client: &C, request: &GraphQLRequest) -> Result<GraphQLResponse<crate::models::LoginData>, Box<dyn std::error::Error>> {
    client.login_request(request).await
}

pub async fn graphql_request<T: DeserializeOwned + 'static, C: ApiClient>(client: &C, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>> {
    client.graphql_request(token, query, variables).await
}

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