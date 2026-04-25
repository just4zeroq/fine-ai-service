use salvo::prelude::*;
use std::sync::Arc;

use crate::auth::AuthService;
use crate::db;
use crate::error::{write_error_response, AppError};
use crate::models::{KlineResponse, StockListResponse};
use crate::rate_limit::{RateLimiter, RateLimitResult};

#[derive(Debug, Handler)]
pub struct StockListHandler {
    pool: sqlx::MySqlPool,
    auth: Arc<AuthService>,
    rate_limiter: Arc<RateLimiter>,
}

impl StockListHandler {
    pub fn new(pool: sqlx::MySqlPool, auth: Arc<AuthService>, rate_limiter: Arc<RateLimiter>) -> Self {
        Self { pool, auth, rate_limiter }
    }
}

#[async_trait]
impl Handler for StockListHandler {
    async fn handle(&self, req: &mut Request, res: &mut Response) -> anyhow::Result<()> {
        // Rate limit check
        let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_default();
        let auth_header = req.headers().get("authorization").and_then(|v| v.to_str().ok());

        let key = auth_header.and_then(|h| {
            if h.starts_with("Bearer ") || h.starts_with("Token ") {
                Some(&h[7..])
            } else {
                Some(h)
            }
        });

        match self.rate_limiter.check(&ip, key) {
            RateLimitResult::KeyLimited => {
                write_error_response(res, &AppError::RateLimited("Key rate limit exceeded".into()));
                return Ok(());
            }
            RateLimitResult::IpLimited => {
                write_error_response(res, &AppError::RateLimited("IP rate limit exceeded".into()));
                return Ok(());
            }
            RateLimitResult::Allowed => {}
        }

        // Auth check
        if let Err(e) = self.auth.validate_http(auth_header) {
            write_error_response(res, &e);
            return Ok(());
        }

        // Parse query
        let search = req.queries().get("search").cloned();

        // Query
        match db::queries::search_stocks(&self.pool, search.as_deref()).await {
            Ok(stocks) => {
                res.status_code(StatusCode::OK);
                res.render(Json(StockListResponse { data: stocks }));
            }
            Err(e) => {
                write_error_response(res, &e);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Handler)]
pub struct KlineHandler {
    pool: sqlx::MySqlPool,
    auth: Arc<AuthService>,
    rate_limiter: Arc<RateLimiter>,
}

impl KlineHandler {
    pub fn new(pool: sqlx::MySqlPool, auth: Arc<AuthService>, rate_limiter: Arc<RateLimiter>) -> Self {
        Self { pool, auth, rate_limiter }
    }
}

#[async_trait]
impl Handler for KlineHandler {
    async fn handle(&self, req: &mut Request, res: &mut Response) -> anyhow::Result<()> {
        // Rate limit check
        let ip = req.peer_addr().map(|a| a.to_string()).unwrap_or_default();
        let auth_header = req.headers().get("authorization").and_then(|v| v.to_str().ok());

        let key = auth_header.and_then(|h| {
            if h.starts_with("Bearer ") || h.starts_with("Token ") {
                Some(&h[7..])
            } else {
                Some(h)
            }
        });

        match self.rate_limiter.check(&ip, key) {
            RateLimitResult::KeyLimited => {
                write_error_response(res, &AppError::RateLimited("Key rate limit exceeded".into()));
                return Ok(());
            }
            RateLimitResult::IpLimited => {
                write_error_response(res, &AppError::RateLimited("IP rate limit exceeded".into()));
                return Ok(());
            }
            RateLimitResult::Allowed => {}
        }

        // Auth check
        if let Err(e) = self.auth.validate_http(auth_header) {
            write_error_response(res, &e);
            return Ok(());
        }

        // Parse query params
        let code = match req.queries().get("code").cloned() {
            Some(c) => c,
            None => {
                write_error_response(res, &AppError::BadRequest("Missing required parameter: code".into()));
                return Ok(());
            }
        };

        let start = match req.queries().get("start").cloned() {
            Some(s) => s,
            None => {
                write_error_response(res, &AppError::BadRequest("Missing required parameter: start".into()));
                return Ok(());
            }
        };

        let end = match req.queries().get("end").cloned() {
            Some(e) => e,
            None => {
                write_error_response(res, &AppError::BadRequest("Missing required parameter: end".into()));
                return Ok(());
            }
        };

        // Query
        match db::queries::query_kline(&self.pool, &code, &start, &end).await {
            Ok(records) => {
                res.status_code(StatusCode::OK);
                res.render(Json(KlineResponse { code, data: records }));
            }
            Err(e) => {
                write_error_response(res, &e);
            }
        }
        Ok(())
    }
}