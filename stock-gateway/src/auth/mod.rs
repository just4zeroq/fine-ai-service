pub mod api_key;
pub mod jwt;

pub use api_key::ApiKeyAuth;
pub use jwt::JwtAuth;

use crate::config::Config;
use crate::error::{AppError, Result};

#[derive(Clone, Debug)]
pub struct AuthService {
    api_key: ApiKeyAuth,
    jwt: JwtAuth,
}

impl AuthService {
    pub fn new(cfg: &Config) -> Self {
        Self {
            api_key: ApiKeyAuth::new(cfg.auth.apikeys.clone()),
            jwt: JwtAuth::new(&cfg.jwt),
        }
    }

    /// Validate auth header for HTTP (supports both API Key and JWT)
    pub fn validate_http(&self, auth_header: Option<&str>) -> Result<()> {
        let header = auth_header.ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        if header.starts_with("Bearer ") {
            // Try JWT first (HTTP only)
            let token = &header[7..];
            if self.jwt.validate(token).is_ok() {
                return Ok(());
            }
            // Fallback: treat as API Key
            self.api_key.validate(token)
        } else if header.starts_with("Token ") {
            let key = &header[6..];
            self.api_key.validate(key)
        } else {
            // Plain key
            self.api_key.validate(header)
        }
    }

    /// Validate auth header for MCP (API Key only)
    pub fn validate_mcp(&self, auth_header: Option<&str>) -> Result<()> {
        let header = auth_header.ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        if header.starts_with("Bearer ") {
            let key = &header[7..];
            self.api_key.validate(key)
        } else {
            self.api_key.validate(header)
        }
    }
}