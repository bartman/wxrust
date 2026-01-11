use wxrust::auth::{decode_token, load_uid_from_cache};
use base64::{Engine, engine::general_purpose};
use tempfile::TempDir;
use std::fs;

#[test]
fn test_decode_token_valid() {
    // Create a sample JWT token
    // Header: {"alg":"HS256","typ":"JWT"}
    // Payload: {"id":123,"exp":2000000000}
    // Signature: dummy (not verified in decode_token)
    let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#.as_bytes());
    let payload = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"id":123,"exp":2000000000}"#.as_bytes());
    let signature = "dummy_signature";  // Not used in decode
    let token = format!("{}.{}.{}", header, payload, signature);

    let claims = decode_token(&token).unwrap();
    assert_eq!(claims.id, 123);
    assert_eq!(claims.exp, 2000000000);
}

#[test]
fn test_decode_token_invalid_format() {
    let token = "invalid.token";
    let result = decode_token(token);
    assert!(result.is_err());
}

#[test]
fn test_decode_token_invalid_base64() {
    let token = "header.invalid.signature";
    let result = decode_token(token);
    assert!(result.is_err());
}

#[test]
fn test_decode_token_invalid_json() {
    let header = general_purpose::URL_SAFE_NO_PAD.encode("{}".as_bytes());
    let payload = general_purpose::URL_SAFE_NO_PAD.encode("invalid json".as_bytes());
    let signature = "sig";
    let token = format!("{}.{}.{}", header, payload, signature);
    let result = decode_token(&token);
    assert!(result.is_err());
}

#[test]
fn test_load_uid_from_cache_success() {
    let temp_dir = TempDir::new().unwrap();
    let token_path = temp_dir.path().join("token");
    
    // Create a valid cached token (expires in year 2033)
    let cache_content = r#"{"token":"dummy.token.here","uid":456,"exp":2000000000}"#;
    fs::write(&token_path, cache_content).unwrap();
    
    let result = load_uid_from_cache(&token_path.to_string_lossy());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 456);
}

#[test]
fn test_load_uid_from_cache_file_not_found() {
    let result = load_uid_from_cache("/nonexistent/path/token");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Token file"));
}

#[test]
fn test_load_uid_from_cache_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let token_path = temp_dir.path().join("token");
    
    fs::write(&token_path, "not valid json").unwrap();
    
    let result = load_uid_from_cache(&token_path.to_string_lossy());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid token cache format"));
}

#[test]
fn test_load_uid_from_cache_expired() {
    let temp_dir = TempDir::new().unwrap();
    let token_path = temp_dir.path().join("token");
    
    // Create an expired token (expired in 1990)
    let cache_content = r#"{"token":"dummy.token.here","uid":789,"exp":631152000}"#;
    fs::write(&token_path, cache_content).unwrap();
    
    let result = load_uid_from_cache(&token_path.to_string_lossy());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expired"));
}