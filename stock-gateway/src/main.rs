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
use mcp::tools::StockMcpService;
use rate_limit::RateLimiter;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager};

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
    let mcp_handle = tokio::spawn(async move {
        let ct = tokio_util::sync::CancellationToken::new();

        let service = StreamableHttpService::new(
            move || Ok(StockMcpService::new(mcp_pool.clone(), mcp_auth.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
        );

        let router = axum::Router::new().nest_service("/mcp", service);
        let addr = format!("{}:{}", mcp_cfg.host, mcp_cfg.port);
        tracing::info!("MCP server listening on {}", addr);
        let tcp_listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                tokio::signal::ctrl_c().await.unwrap();
                ct.cancel();
            })
            .await
            .unwrap();
    });

    // Wait for both servers
    tokio::select! {
        result = http_handle => {
            if let Err(e) = result {
                tracing::error!("HTTP server error: {}", e);
            }
        }
        result = mcp_handle => {
            if let Err(e) = result {
                tracing::error!("MCP server error: {}", e);
            }
        }
    }

    Ok(())
}
