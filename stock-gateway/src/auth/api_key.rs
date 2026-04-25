use crate::error::{AppError, Result};

#[derive(Clone, Debug)]
pub struct ApiKeyAuth {
    valid_keys: Vec<String>,
}

impl ApiKeyAuth {
    pub fn new(keys: Vec<String>) -> Self {
        Self { valid_keys: keys }
    }

    pub fn validate(&self, key: &str) -> Result<()> {
        if self.valid_keys.contains(&key.to_string()) {
            Ok(())
        } else {
            Err(AppError::Unauthorized("Invalid API key".into()))
        }
    }
}