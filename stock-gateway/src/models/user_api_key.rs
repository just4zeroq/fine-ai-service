use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserApiKey {
    pub id: i64,
    pub user_id: String,
    pub api_key: String,
    pub name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserApiKey {
    pub user_id: String,
    pub api_key: String,
    pub name: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserApiKeyResponse {
    pub id: i64,
    pub user_id: String,
    pub name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<UserApiKey> for UserApiKeyResponse {
    fn from(key: UserApiKey) -> Self {
        Self {
            id: key.id,
            user_id: key.user_id,
            name: key.name,
            is_active: key.is_active,
            created_at: key.created_at,
            expires_at: key.expires_at,
        }
    }
}
