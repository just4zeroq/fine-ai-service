use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Stock {
    pub code: String,
    pub name: Option<String>,
    pub se: Option<String>,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub stock_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KlineRecord {
    pub date: String,
    pub open: Option<f32>,
    pub high: Option<f32>,
    pub low: Option<f32>,
    pub close: Option<f32>,
    pub volume: Option<f32>,
    pub turnover: Option<f32>,
    pub turnover_rate: Option<f32>,
    pub shake_rate: Option<f32>,
    pub jlrl: Option<f32>,
    pub zljlrl: Option<f32>,
    pub change_rate: Option<f32>,
    pub change_amount: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineResponse {
    pub code: String,
    pub data: Vec<KlineRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockListResponse {
    pub data: Vec<Stock>,
}