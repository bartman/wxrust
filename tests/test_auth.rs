use wxrust::auth::decode_token;
use base64::{Engine, engine::general_purpose};

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