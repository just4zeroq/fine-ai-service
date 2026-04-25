use sqlx::MySqlPool;

use crate::error::{AppError, Result};
use crate::models::{KlineRecord, Stock};

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