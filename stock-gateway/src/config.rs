use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mcp: McpConfig,
    pub http: HttpConfig,
    pub rate_limit: RateLimitConfig,
    pub auth: AuthConfig,
    pub jwt: JwtConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub ip_limit: u32,
    pub key_limit: u32,
    pub window_sec: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub apikeys: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub expiration_hours: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Arc<Self>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(Arc::new(config))
    }
}