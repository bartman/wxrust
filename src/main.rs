use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, ClientId, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenUrl,
};
use reqwest::Client;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // WeightXReps OAuth2 configuration
    let client_id = ClientId::new("your-service-id".to_string()); // Replace with your service ID
    let auth_url = AuthUrl::new("https://weightxreps.net/api/auth".to_string())?;
    let token_url = TokenUrl::new("https://weightxreps.net/api/auth/token".to_string())?;

    let client = BasicClient::new(client_id)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(RedirectUrl::new("urn:ietf:wg:oauth:2.0:oob".to_string())?); // For manual code entry

    // Generate PKCE challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate authorization URL
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("jwrite".to_string())) // Example scope for writing workouts
        .add_scope(Scope::new("jread".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("Open this URL in your browser:\n{}", auth_url);

    print!("Enter the authorization code: ");
    io::stdout().flush()?;
    let mut code = String::new();
    io::stdin().read_line(&mut code)?;
    let code = code.trim();

    // Exchange code for token
    let token_result = client
        .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?;

    println!("Access token: {:?}", token_result.access_token().secret());

    // Now you can use the token to make API calls
    let http_client = Client::new();
    let response = http_client
        .post("https://weightxreps.net/api/graphql") // Example endpoint
        .header("Authorization", format!("Bearer {}", token_result.access_token().secret()))
        .json(&serde_json::json!({"query": "query { user { id } }"})) // Example query
        .send()
        .await?;

    println!("API Response: {}", response.text().await?);

    Ok(())
}
