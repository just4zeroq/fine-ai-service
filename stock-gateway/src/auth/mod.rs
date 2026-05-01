pub mod api_key;
pub mod jwt;

pub use api_key::ApiKeyAuth;
pub use jwt::JwtAuth;

use crate::config::Config;
use crate::error::{AppError, Result};

#[derive(Debug)]
pub struct AuthService {
    api_key: ApiKeyAuth,
    jwt: JwtAuth,
}

impl AuthService {
    pub fn new(cfg: &Config, pool: sqlx::MySqlPool) -> Self {
        Self {
            api_key: ApiKeyAuth::new(pool, 1000, 300), // 1000 entries, 5 min TTL
            jwt: JwtAuth::new(&cfg.jwt),
        }
    }

    /// Validate auth header for HTTP (supports both API Key and JWT)
    pub async fn validate_http(&self, auth_header: Option<&str>) -> Result<()> {
        let header = auth_header.ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        if header.starts_with("Bearer ") {
            // Try JWT first (HTTP only)
            let token = &header[7..];
            if self.jwt.validate(token).is_ok() {
                return Ok(());
            }
            // Fallback: treat as API Key
            self.api_key.validate(token).await
        } else if header.starts_with("Token ") {
            let key = &header[6..];
            self.api_key.validate(key).await
        } else {
            // Plain key
            self.api_key.validate(header).await
        }
    }

    /// Validate auth header for MCP (API Key only)
    pub async fn validate_mcp(&self, auth_header: Option<&str>) -> Result<()> {
        let header = auth_header.ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        if header.starts_with("Bearer ") {
            let key = &header[7..];
            self.api_key.validate(key).await
        } else {
            self.api_key.validate(header).await
        }
    }
}
