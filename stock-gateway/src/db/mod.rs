pub mod queries;

use std::time::Duration;

use sqlx::mysql::MySqlPoolOptions;

use crate::config::DatabaseConfig;

pub async fn create_pool(cfg: &DatabaseConfig) -> anyhow::Result<sqlx::MySqlPool> {
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        cfg.user, cfg.password, cfg.host, cfg.port, cfg.database
    );
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        // Recycle connections before MySQL's wait_timeout (default 8h) to prevent
        // stale connections from hanging requests and causing CLOSE_WAIT on clients
        .max_lifetime(Some(Duration::from_secs(300)))
        // Close idle connections quickly to free DB resources
        .idle_timeout(Some(Duration::from_secs(60)))
        .connect(&url)
        .await?;
    Ok(pool)
}