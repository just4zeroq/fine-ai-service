use serde::Deserialize;

use crate::auth::AuthService;
use crate::db;
use crate::error::AppError;
use crate::models::{KlineResponse, StockListResponse};

#[derive(Debug, Deserialize)]
pub struct StockListInput {
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StockKlineInput {
    pub code: String,
    pub start: String,
    pub end: String,
}

pub struct StockTools {
    pool: sqlx::MySqlPool,
    auth: AuthService,
}

impl StockTools {
    pub fn new(pool: sqlx::MySqlPool, auth: AuthService) -> Self {
        Self { pool, auth }
    }

    pub async fn stock_list(&self, input: StockListInput) -> Result<serde_json::Value, AppError> {
        let stocks = db::queries::search_stocks(&self.pool, input.search.as_deref()).await?;
        Ok(serde_json::to_value(StockListResponse { data: stocks }).unwrap())
    }

    pub async fn stock_kline(&self, input: StockKlineInput) -> Result<serde_json::Value, AppError> {
        let records = db::queries::query_kline(&self.pool, &input.code, &input.start, &input.end).await?;
        Ok(serde_json::to_value(KlineResponse {
            code: input.code,
            data: records,
        }).unwrap())
    }
}
