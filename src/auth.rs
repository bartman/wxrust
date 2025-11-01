use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::Deserialize;
use std::fs;

use crate::api;
use crate::models;

#[derive(Deserialize)]
pub struct Claims {
    pub id: u32,
}

pub async fn login(credentials_path: &str) -> Result<String, String> {
    let credentials = fs::read_to_string(credentials_path).map_err(|_| "credentials.txt not found. Please create it with email on first line and password on second.".to_string())?;
    let lines: Vec<&str> = credentials.lines().collect();
    if lines.len() < 2 {
        return Err("credentials.txt must have at least 2 lines: email and password".to_string());
    }
    let email = lines[0].to_string();
    let password = lines[1].to_string();

    let request = models::GraphQLRequest {
        query: "mutation login($u: String!, $p: String!) { login(u: $u, p: $p) }".to_string(),
        variables: models::LoginVariables { u: email, p: password },
    };

    let client = Client::new();
    let response = api::login_request(&client, &request).await.map_err(|e| e.to_string())?;

    if let Some(data) = response.data {
        Ok(data.login)
    } else if let Some(errors) = response.errors {
        Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; "))
    } else {
        Err("Unexpected response".to_string())
    }
}

pub fn decode_token(token: &str) -> Result<Claims, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid token format".into());
    }
    let payload = parts[1];
    let decoded = general_purpose::URL_SAFE_NO_PAD.decode(payload)?;
    let claims: Claims = serde_json::from_slice(&decoded)?;
    Ok(claims)
}