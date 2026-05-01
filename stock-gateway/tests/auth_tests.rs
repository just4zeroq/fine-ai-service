use stock_gateway::auth::JwtAuth;
use stock_gateway::config::JwtConfig;

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
