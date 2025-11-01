use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json;

use crate::models::{GraphQLRequest, GraphQLResponse, WorkoutRequest, WorkoutResponse};

pub async fn login_request(client: &Client, request: &GraphQLRequest) -> Result<GraphQLResponse<crate::models::LoginData>, Box<dyn std::error::Error>> {
    let response = client
        .post("https://weightxreps.net/api/graphql")
        .json(request)
        .send()
        .await?;
    let body: GraphQLResponse<crate::models::LoginData> = response.json().await?;
    Ok(body)
}

pub async fn graphql_request<T: DeserializeOwned>(client: &Client, token: &str, query: &str, variables: Option<serde_json::Value>) -> Result<GraphQLResponse<T>, Box<dyn std::error::Error>> {
    let request_body = if let Some(vars) = variables {
        serde_json::json!({ "query": query, "variables": vars })
    } else {
        serde_json::json!({ "query": query })
    };
    let response = client
        .post("https://weightxreps.net/api/graphql")
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .send()
        .await?;
    let body: GraphQLResponse<T> = response.json().await?;
    Ok(body)
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