use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::api;
use crate::models;

#[derive(Deserialize)]
pub struct Claims {
    pub id: u32,
    pub exp: u64,
}

#[derive(Serialize, Deserialize)]
struct CachedToken {
    token: String,
    uid: u32,
    exp: u64,
}

pub async fn login<C: crate::api::ApiClient>(client: &C, credentials_path: &str, token_path: &str, force_auth: bool) -> Result<String, String> {
    // Check if token file exists and is valid (unless force_auth is true)
    if !force_auth
        && let Ok(contents) = fs::read_to_string(token_path)
            && let Ok(cached) = serde_json::from_str::<CachedToken>(&contents) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if cached.exp > now {
                    return Ok(cached.token);
                }
            }

    // Perform login
    let credentials = fs::read_to_string(credentials_path).map_err(|_| format!("{} not found. Please create it with email on first line and password on second.", credentials_path))?;
    let lines: Vec<&str> = credentials.lines().collect();
    if lines.len() < 2 {
        return Err(format!("{} must have at least 2 lines: email and password", credentials_path));
    }
    let email = lines[0].to_string();
    let password = lines[1].to_string();

    let request = models::GraphQLRequest {
        query: "mutation login($u: String!, $p: String!) { login(u: $u, p: $p) }".to_string(),
        variables: models::LoginVariables { u: email, p: password },
    };

    let response = api::login_request(client, &request).await.map_err(|e| e.to_string())?;

    if let Some(data) = response.data {
        let token = data.login;
        // Decode to get uid and exp
        let claims = decode_token(&token).map_err(|e| e.to_string())?;
        let cached = CachedToken {
            token: token.clone(),
            uid: claims.id,
            exp: claims.exp,
        };
        // Write to file
        if let Some(parent) = Path::new(token_path).parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string(&cached).map_err(|e| e.to_string())?;
        fs::write(token_path, json).map_err(|e| e.to_string())?;
        Ok(token)
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

pub fn load_uid_from_cache(token_path: &str) -> Result<u32, String> {
    let contents = fs::read_to_string(token_path).map_err(|_| format!("Token file {} not found", token_path))?;
    let cached: CachedToken = serde_json::from_str(&contents).map_err(|_| "Invalid token cache format".to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if cached.exp > now {
        Ok(cached.uid)
    } else {
        Err("Cached token expired".to_string())
    }
}
