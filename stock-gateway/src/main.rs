use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioTimer;
use salvo::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing_subscriber;

// Use an alias with :: prefix to force resolution from the external crate,
// avoiding collision with salvo::http module brought in by the prelude
use ::http as http_crate;

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
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};

/// Salvo Handler that bridges HTTP requests to the rmcp StreamableHttpService tower service.
///
/// Extracts the raw request from Salvo, constructs a `hyper::Request`, forwards it to
/// the rmcp MCP service, and maps the response back to Salvo.
struct McpHandler {
    service: StreamableHttpService<StockMcpService, LocalSessionManager>,
}

impl McpHandler {
    fn new(service: StreamableHttpService<StockMcpService, LocalSessionManager>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Handler for McpHandler {
    async fn handle(&self, req: &mut Request, _depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
        // Read request body as owned bytes
        let body_bytes: bytes::Bytes = match req.payload().await {
            Ok(bytes) => bytes.to_owned(),
            Err(_) => {
                res.status_code(StatusCode::BAD_REQUEST);
                res.write_body("Failed to read request body").ok();
                return;
            }
        };

        // Extract HTTP parts from Salvo request
        let uri = req.uri().clone();
        let method = req.method().clone();
        let headers = req.headers().clone();

        // Build a hyper::Request with Full<Bytes> body (compatible with rmcp's generic Body trait bound)
        let mut builder = http_crate::Request::builder()
            .method(method)
            .uri(uri);
        for (name, value) in &headers {
            builder = builder.header(name, value);
        }
        let hyper_req: http_crate::Request<Full<bytes::Bytes>> = match builder.body(Full::new(body_bytes)) {
            Ok(req) => req,
            Err(_) => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                res.write_body("Failed to build MCP request").ok();
                return;
            }
        };

        // Forward to the rmcp service with a timeout to prevent CLOSE_WAIT pile-up
        // when the proxy disconnects before the MCP service responds
        let hyper_res = match tokio::time::timeout(Duration::from_secs(25), self.service.handle(hyper_req)).await {
            Ok(resp) => resp,
            Err(_) => {
                tracing::warn!("MCP request timed out after 25s");
                res.status_code(StatusCode::GATEWAY_TIMEOUT);
                res.write_body("MCP request timed out").ok();
                return;
            }
        };

        // Copy status code (Salvo uses a setter, not a mut accessor)
        res.status_code(StatusCode::from_u16(hyper_res.status().as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));

        // Copy headers
        *res.headers_mut() = hyper_res.headers().clone();

        // Copy response body
        match BodyExt::collect(hyper_res.into_body()).await {
            Ok(collected) => {
                res.write_body(collected.to_bytes()).ok();
            }
            Err(_) => {
                res.write_body("Failed to read MCP response body").ok();
            }
        }
    }
}

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
    let auth = Arc::new(AuthService::new(&cfg, pool.clone()));
    let rate_limiter = Arc::new(RateLimiter::new(&cfg.rate_limit));

    // Global shutdown signal
    let shutdown = CancellationToken::new();

    // Spawn combined HTTP + MCP server on a single port
    let http_cfg = cfg.http.clone();
    let http_pool = pool.clone();
    let http_auth = auth.clone();
    let http_rate_limiter = rate_limiter.clone();
    let http_shutdown = shutdown.child_token();
    let http_handle = tokio::spawn(async move {
        // Clone pool/auth for the MCP service factory (used inside the closure)
        let mcp_pool = http_pool.clone();
        let mcp_auth = http_auth.clone();

        // Create the MCP service
        let mcp_ct = CancellationToken::new();
        let mcp_service = StreamableHttpService::new(
            move || Ok(StockMcpService::new(mcp_pool.clone(), mcp_auth.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default()
                .with_cancellation_token(mcp_ct.child_token())
                .disable_allowed_hosts(), // Disable Host header check (runs behind CLB/proxy)
        );

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
            ))
            .push(Router::with_path("/mcp").post(McpHandler::new(mcp_service)));

        let addr = format!("{}:{}", http_cfg.host, http_cfg.port);
        tracing::info!("HTTP + MCP server listening on {}", addr);
        let listener = TcpListener::new(addr.as_str());
        let acceptor = listener.try_bind().await.unwrap();
        let mut server = Server::new(acceptor);
        // Disable keepalive — the service runs behind a proxy (tproxy),
        // which manages its own connection pool. Without this, when the proxy
        // disconnects during a slow request, the connection stays in CLOSE_WAIT
        // and accumulates until FD exhaustion.
        server
            .http1_mut()
            .timer(TokioTimer::new())
            .header_read_timeout(Duration::from_secs(30))
            .keep_alive(false);
        let handle = server.handle();
        let stop_token = http_shutdown.clone();
        tokio::spawn(async move {
            stop_token.cancelled().await;
            tracing::info!("shutting down HTTP server gracefully");
            handle.stop_graceful(Duration::from_secs(30));
        });
        server.serve(router).await;
    });

    // Single ctrl_c handler for the process
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        tracing::info!("shutdown signal received, stopping all servers");
        shutdown.cancel();
    });

    // Wait for the single combined server
    tokio::select! {
        result = http_handle => {
            if let Err(e) = result {
                tracing::error!("HTTP server error: {}", e);
            }
        }
    }

    Ok(())
}
