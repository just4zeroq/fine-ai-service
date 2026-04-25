use async_trait::async_trait;
use salvo::prelude::*;
use std::sync::Arc;

use crate::auth::AuthService;
use crate::db;
use crate::error::{write_error_response, AppError};
use crate::models::{KlineResponse, StockListResponse};
use crate::rate_limit::{RateLimiter, RateLimitResult};

#[derive(Debug)]
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
    async fn handle(&self, req: &mut Request, _depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
        // Rate limit check
        // Try to get real IP from X-Forwarded-For, X-Real-IP, or fallback to remote addr
        let ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .or_else(|| {
                req.headers()
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "127.0.0.1".to_string());
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
                return;
            }
            RateLimitResult::IpLimited => {
                write_error_response(res, &AppError::RateLimited("IP rate limit exceeded".into()));
                return;
            }
            RateLimitResult::Allowed => {}
        }

        // Auth check
        if let Err(e) = self.auth.validate_http(auth_header) {
            write_error_response(res, &e);
            return;
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
    }
}

#[derive(Debug)]
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
    async fn handle(&self, req: &mut Request, _depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
        // Rate limit check
        // Try to get real IP from X-Forwarded-For, X-Real-IP, or fallback to remote addr
        let ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .or_else(|| {
                req.headers()
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "127.0.0.1".to_string());
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
                return;
            }
            RateLimitResult::IpLimited => {
                write_error_response(res, &AppError::RateLimited("IP rate limit exceeded".into()));
                return;
            }
            RateLimitResult::Allowed => {}
        }

        // Auth check
        if let Err(e) = self.auth.validate_http(auth_header) {
            write_error_response(res, &e);
            return;
        }

        // Parse query params
        let code = match req.queries().get("code").cloned() {
            Some(c) => c,
            None => {
                write_error_response(res, &AppError::BadRequest("Missing required parameter: code".into()));
                return;
            }
        };

        let start = match req.queries().get("start").cloned() {
            Some(s) => s,
            None => {
                write_error_response(res, &AppError::BadRequest("Missing required parameter: start".into()));
                return;
            }
        };

        let end = match req.queries().get("end").cloned() {
            Some(e) => e,
            None => {
                write_error_response(res, &AppError::BadRequest("Missing required parameter: end".into()));
                return;
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
    }
}
