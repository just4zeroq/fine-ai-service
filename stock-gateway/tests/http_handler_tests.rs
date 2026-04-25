use stock_gateway::config::Config;

#[test]
fn test_config_loads() {
    // This test verifies config parsing works
    let config = Config::load("config.toml").unwrap();
    assert_eq!(config.http.port, 8081);
    assert_eq!(config.mcp.port, 8080);
    assert_eq!(config.rate_limit.ip_limit, 60);
    assert_eq!(config.rate_limit.key_limit, 1000);
}

#[test]
fn test_config_database_settings() {
    let config = Config::load("config.toml").unwrap();
    assert_eq!(config.database.host, "rm-uf6cpg7cwe8xu3i6oso.mysql.rds.aliyuncs.com");
    assert_eq!(config.database.port, 3306);
    assert_eq!(config.database.user, "fintools");
    assert_eq!(config.database.database, "cn_stocks");
}

#[test]
fn test_auth_keys_loaded() {
    let config = Config::load("config.toml").unwrap();
    assert!(config.auth.apikeys.len() >= 2);
    assert!(config.auth.apikeys.contains(&"sk-test-key-001".to_string()));
}