use stock_gateway::auth::{ApiKeyAuth, JwtAuth};
use stock_gateway::config::JwtConfig;

#[test]
fn test_api_key_valid() {
    let auth = ApiKeyAuth::new(vec!["key1".into(), "key2".into()]);
    assert!(auth.validate("key1").is_ok());
}

#[test]
fn test_api_key_invalid() {
    let auth = ApiKeyAuth::new(vec!["key1".into()]);
    assert!(auth.validate("wrong_key").is_err());
}

#[test]
fn test_jwt_generate_and_validate() {
    let cfg = JwtConfig {
        secret: "test-secret".into(),
        issuer: "test".into(),
        expiration_hours: 1,
    };
    let jwt = JwtAuth::new(&cfg);
    let token = jwt.generate_token("user1").unwrap();
    let claims = jwt.validate(&token).unwrap();
    assert_eq!(claims.sub, "user1");
    assert_eq!(claims.iss, "test");
}