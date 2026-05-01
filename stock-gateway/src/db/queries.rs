use chrono::Utc;
use sqlx::MySqlPool;

use crate::error::{AppError, Result};
use crate::models::{CreateUserApiKey, KlineRecord, Stock, UserApiKey, UserApiKeyResponse};

pub async fn search_stocks(pool: &MySqlPool, search: Option<&str>) -> Result<Vec<Stock>> {
    let stocks = match search {
        Some(s) if !s.is_empty() => {
            let pattern = format!("%{}%", s);
            sqlx::query_as::<_, Stock>(
                "SELECT code, name, se, type FROM stock WHERE code LIKE ? OR name LIKE ? LIMIT 100"
            )
            .bind(&pattern)
            .bind(&pattern)
            .fetch_all(pool)
            .await?
        },
        _ => {
            sqlx::query_as::<_, Stock>("SELECT code, name, se, type FROM stock LIMIT 100")
                .fetch_all(pool)
                .await?
        }
    };
    Ok(stocks)
}

pub async fn query_kline(
    pool: &MySqlPool,
    code: &str,
    start: &str,
    end: &str,
) -> Result<Vec<KlineRecord>> {
    // Validate date range (max 365 days)
    let start_date = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid start date format".into()))?;
    let end_date = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid end date format".into()))?;

    let days = (end_date - start_date).num_days();
    if days < 0 {
        return Err(AppError::BadRequest("start date must be before end date".into()));
    }
    if days > 365 {
        return Err(AppError::BadRequest("date range cannot exceed 365 days".into()));
    }

    // Query from the per-stock table
    let query = format!(
        "SELECT date, open, high, low, close, volume, turnover, turnover_rate,
         shake_rate, jlrl, zljlrl, change_rate, change_amount
         FROM `{}` WHERE date >= ? AND date <= ? ORDER BY date ASC",
        code
    );

    let records: Vec<KlineRecord> = sqlx::query_as::<_, KlineRecord>(&query)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("Stock {} not found", code)),
            other => AppError::Database(other),
        })?;

    Ok(records)
}

// User API Key queries

pub async fn create_user_api_key(
    pool: &MySqlPool,
    key: &CreateUserApiKey,
) -> Result<UserApiKeyResponse> {
    let now = Utc::now();
    let result = sqlx::query_as::<_, UserApiKey>(
        r#"INSERT INTO user_api_keys (user_id, api_key, name, is_active, created_at, expires_at)
           VALUES (?, ?, ?, true, ?, ?)
           RETURNING id, user_id, api_key, name, is_active, created_at, expires_at"#
    )
    .bind(&key.user_id)
    .bind(&key.api_key)
    .bind(&key.name)
    .bind(now)
    .bind(key.expires_at)
    .fetch_one(pool)
    .await?;

    Ok(result.into())
}

pub async fn get_user_api_keys(
    pool: &MySqlPool,
    user_id: &str,
) -> Result<Vec<UserApiKeyResponse>> {
    let keys = sqlx::query_as::<_, UserApiKey>(
        "SELECT id, user_id, api_key, name, is_active, created_at, expires_at FROM user_api_keys WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(keys.into_iter().map(|k| k.into()).collect())
}

pub async fn get_user_api_key_by_key(
    pool: &MySqlPool,
    api_key: &str,
) -> Result<UserApiKey> {
    let key = sqlx::query_as::<_, UserApiKey>(
        "SELECT id, user_id, api_key, name, is_active, created_at, expires_at FROM user_api_keys WHERE api_key = ?"
    )
    .bind(api_key)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

    Ok(key)
}

pub async fn validate_user_api_key(
    pool: &MySqlPool,
    api_key: &str,
) -> Result<UserApiKey> {
    let key = get_user_api_key_by_key(pool, api_key).await?;

    if !key.is_active {
        return Err(AppError::Unauthorized("API key is inactive".into()));
    }

    if let Some(expires_at) = key.expires_at {
        if expires_at < Utc::now() {
            return Err(AppError::Unauthorized("API key has expired".into()));
        }
    }

    Ok(key)
}

pub async fn deactivate_user_api_key(
    pool: &MySqlPool,
    id: i64,
) -> Result<()> {
    sqlx::query("UPDATE user_api_keys SET is_active = false WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_user_api_key(
    pool: &MySqlPool,
    id: i64,
) -> Result<()> {
    sqlx::query("DELETE FROM user_api_keys WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}