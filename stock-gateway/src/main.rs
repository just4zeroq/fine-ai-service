use salvo::prelude::*;
use std::sync::Arc;
use tracing_subscriber;

mod auth;
mod config;
mod db;
mod error;
mod http;
mod mcp;
mod models;
mod rate_limit;

use auth::AuthService;
use config::Config;
use db::create_pool;
use http::{KlineHandler, StockListHandler};
use rate_limit::RateLimiter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init logging
    tracing_subscriber::fmt()
        .with_env_filter("stock_gateway=debug,info")
        .init();

    // Load config
    let cfg = Config::load("config.toml")?;

    // Create DB pool
    let pool = create_pool(&cfg.database).await?;

    // Init services
    let auth = Arc::new(AuthService::new(&cfg));
    let rate_limiter = Arc::new(RateLimiter::new(&cfg.rate_limit));

    // Spawn HTTP server
    let http_cfg = cfg.http.clone();
    let http_pool = pool.clone();
    let http_auth = auth.clone();
    let http_rate_limiter = rate_limiter.clone();
    let http_handle = tokio::spawn(async move {
        let router = Router::new()
            .push(Router::with_path("/api/v1").push(
                Router::with_path("stocks").get(StockListHandler::new(
                    http_pool.clone(),
                    http_auth.clone(),
                    http_rate_limiter.clone(),
                ))
            ))
            .push(Router::with_path("/api/v1").push(
                Router::with_path("kline").get(KlineHandler::new(
                    http_pool.clone(),
                    http_auth.clone(),
                    http_rate_limiter.clone(),
                ))
            ));

        let addr = format!("{}:{}", http_cfg.host, http_cfg.port);
        tracing::info!("HTTP server listening on {}", addr);
        let listener = TcpListener::new(addr.as_str());
        let acceptor = listener.try_bind().await.unwrap();
        Server::new(acceptor).serve(router).await;
    });

    // Spawn MCP server
    let mcp_cfg = cfg.mcp.clone();
    let mcp_pool = pool.clone();
    let mcp_auth = auth.clone();
    let mcp_rate_limiter = rate_limiter.clone();
    let _mcp_handle = tokio::spawn(async move {
        let addr = format!("{}:{}", mcp_cfg.host, mcp_cfg.port);
        tracing::info!("MCP server listening on {}", addr);

        // Placeholder: actual rmcp server setup goes here
        // For now, we just log the address since rmcp API is not yet wired up
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    // Wait for HTTP server (MCP is placeholder)
    http_handle.await?;

    Ok(())
}
