use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::config::JwtConfig;
use crate::error::{AppError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iss: String,
}

pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    issuer: String,
    expiration_hours: u64,
}

impl JwtAuth {
    pub fn new(cfg: &JwtConfig) -> Self {
        let mut validation = Validation::default();
        validation.set_issuer(&[&cfg.issuer]);
        Self {
            encoding_key: EncodingKey::from_secret(cfg.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(cfg.secret.as_bytes()),
            validation,
            issuer: cfg.issuer.clone(),
            expiration_hours: cfg.expiration_hours,
        }
    }

    pub fn generate_token(&self, subject: &str) -> Result<String> {
        let exp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(self.expiration_hours as i64))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: subject.into(),
            exp,
            iss: self.issuer.clone(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Internal(format!("JWT encode error: {}", e)))
    }

    pub fn validate(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| AppError::Unauthorized(format!("JWT validation failed: {}", e)))
    }
}