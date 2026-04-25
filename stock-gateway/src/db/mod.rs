pub mod queries;

use sqlx::mysql::MySqlPoolOptions;

use crate::config::DatabaseConfig;

pub async fn create_pool(cfg: &DatabaseConfig) -> anyhow::Result<sqlx::MySqlPool> {
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        cfg.user, cfg.password, cfg.host, cfg.port, cfg.database
    );
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&url)
        .await?;
    Ok(pool)
}