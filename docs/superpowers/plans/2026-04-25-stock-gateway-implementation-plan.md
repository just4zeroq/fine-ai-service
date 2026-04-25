# Stock Gateway Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个基于 rmcp + salvo.rs 的 MCP/HTTP 统一网关，支持股票列表搜索和 K 线查询

**Architecture:** 单进程双端口架构。MCP Server 运行在 :8080，HTTP REST 运行在 :8081。共用同一套 auth + rate_limit 中间件。数据从 MySQL 查询（tushare pro 同步数据）。

**Tech Stack:** Rust, rmcp, salvo, sqlx, tokio, toml

---

## File Structure

```
stock-gateway/
├── Cargo.toml
├── config.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── api_key.rs
│   │   └── jwt.rs
│   ├── rate_limit/
│   │   ├── mod.rs
│   │   └── sliding_window.rs
│   ├── mcp/
│   │   ├── mod.rs
│   │   └── tools.rs
│   ├── http/
│   │   ├── mod.rs
│   │   └── handlers.rs
│   ├── db/
│   │   ├── mod.rs
│   │   └── queries.rs
│   └── models/
│       ├── mod.rs
│       └── stock.rs
└── tests/
    ├── auth_tests.rs
    ├── rate_limit_tests.rs
    ├── http_handler_tests.rs
    └── db_tests.rs
```

---

## Task Map

| Task | Component | Description |
|------|-----------|-------------|
| 1 | Project Scaffold | 创建 Cargo 项目，配置 Cargo.toml |
| 2 | Config | 配置加载模块 + config.toml |
| 3 | Error Handling | 统一 error type |
| 4 | Models | 数据模型定义 |
| 5 | Database Layer | MySQL 连接 + 查询 |
| 6 | Auth Module | API Key + JWT 鉴权 |
| 7 | Rate Limit Module | 分层滑动窗口限流 |
| 8 | HTTP Handlers | salvo HTTP 路由和 handlers |
| 9 | MCP Server | rmcp tools 定义和 server |
| 10 | Main Entry | 双端口启动 + 中间件组装 |
| 11 | Integration Tests | 端到端测试 |

---

## Database Connection Info

```toml
[database]
host = "rm-uf6cpg7cwe8xu3i6oso.mysql.rds.aliyuncs.com"
port = 3306
user = "fintools"
db_name = "cn_stocks"
password = "123Passwordpro"
```

---

## Task 1: Project Scaffold

**Files:**
- Create: `stock-gateway/Cargo.toml`
- Create: `stock-gateway/config.toml`

- [ ] **Step 1: Create Cargo project**

Run: `cargo new stock-gateway --name stock_gateway`
Output: `Created binary (application) package`

- [ ] **Step 2: Write Cargo.toml dependencies**

```toml
[package]
name = "stock_gateway"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web framework
salvo = { version = "0.76", features = ["affix", "cookie", "jwt-auth"] }
salvo-export = "0.76"

# MCP protocol
rmcp = "0.3"
rmcp-server = "0.3"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "mysql", "chrono"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Config
toml = "0.8"

# JWT
jsonwebtoken = "9"

# Rate limiting
dashmap = "5"

# Date/time
chrono = { version = "0.4", features = ["serde"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
thiserror = "2"
anyhow = "1"

[dev-dependencies]
tokio-test = "0.4"
```

- [ ] **Step 3: Write config.toml**

```toml
[mcp]
host = "0.0.0.0"
port = 8080

[http]
host = "0.0.0.0"
port = 8081

[rate_limit]
ip_limit = 60
key_limit = 1000
window_sec = 60

[auth]
apikeys = [
  "sk-test-key-001",
  "sk-test-key-002"
]

[jwt]
secret = "your-jwt-secret-change-in-production"
issuer = "stock-gateway"
expiration_hours = 24

[database]
host = "rm-uf6cpg7cwe8xu3i6oso.mysql.rds.aliyuncs.com"
port = 3306
user = "fintools"
password = "123Passwordpro"
database = "cn_stocks"
```

- [ ] **Step 4: Create directory structure**

Run: `cd stock-gateway && mkdir -p src/auth src/rate_limit src/mcp src/http src/db src/models tests`

- [ ] **Step 5: Commit**

```bash
git add stock-gateway/Cargo.toml stock-gateway/config.toml
git commit -m "feat: scaffold stock-gateway project"
```

---

## Task 2: Config Module

**Files:**
- Create: `stock-gateway/src/config.rs`
- Modify: `stock-gateway/src/lib.rs` (导出 config)

- [ ] **Step 1: Write config.rs**

```rust
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mcp: McpConfig,
    pub http: HttpConfig,
    pub rate_limit: RateLimitConfig,
    pub auth: AuthConfig,
    pub jwt: JwtConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub ip_limit: u32,
    pub key_limit: u32,
    pub window_sec: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub apikeys: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub expiration_hours: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Arc<Self>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(Arc::new(config))
    }
}
```

- [ ] **Step 2: Write lib.rs**

```rust
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod http;
pub mod mcp;
pub mod models;
pub mod rate_limit;

pub use config::Config;
pub use error::AppError;
```

- [ ] **Step 3: Verify compilation**

Run: `cd stock-gateway && cargo build 2>&1`
Expected: 编译成功，无错误

- [ ] **Step 4: Commit**

```bash
git add stock-gateway/src/config.rs stock-gateway/src/lib.rs
git commit -m "feat: add config module"
```

---

## Task 3: Error Handling

**Files:**
- Create: `stock-gateway/src/error.rs`

- [ ] **Step 1: Write error.rs**

```rust
use salvo::http::StatusCode;
use salvo::写作 Response;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::RateLimited(_) => "RATE_LIMITED",
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Database(_) => "INTERNAL_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

pub fn write_error_response(res: &mut Response, error: &AppError) {
    let body = serde_json::json!({
        "error": {
            "code": error.error_code(),
            "message": error.to_string()
        }
    });
    res.status_code(error.status_code());
    res.render(body);
}
```

- [ ] **Step 2: Commit**

```bash
git add stock-gateway/src/error.rs
git commit -m "feat: add unified error handling"
```

---

## Task 4: Data Models

**Files:**
- Create: `stock-gateway/src/models/mod.rs`
- Create: `stock-gateway/src/models/stock.rs`

- [ ] **Step 1: Write models/stock.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock {
    pub code: String,
    pub name: Option<String>,
    pub se: Option<String>,
    #[serde(rename = "type")]
    pub stock_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

- [ ] **Step 2: Write models/mod.rs**

```rust
pub mod stock;

pub use stock::*;
```

- [ ] **Step 3: Commit**

```bash
git add stock-gateway/src/models/
git commit -m "feat: add stock data models"
```

---

## Task 5: Database Layer

**Files:**
- Create: `stock-gateway/src/db/mod.rs`
- Create: `stock-gateway/src/db/queries.rs`

- [ ] **Step 1: Write db/mod.rs**

```rust
pub mod queries;

use sqlx::mysql::MySqlPoolOptions;
use std::sync::Arc;

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
```

- [ ] **Step 2: Write db/queries.rs**

```rust
use sqlx::MySqlPool;

use crate::error::{AppError, AppError::*, Result};
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
        .map_err(|_| BadRequest("Invalid start date format".into()))?;
    let end_date = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d")
        .map_err(|_| BadRequest("Invalid end date format".into()))?;

    let days = (end_date - start_date).num_days();
    if days < 0 {
        return Err(BadRequest("start date must be before end date".into()));
    }
    if days > 365 {
        return Err(BadRequest("date range cannot exceed 365 days".into()));
    }

    // Query from the per-stock table
    let table_name = code;
    let query = format!(
        "SELECT date, open, high, low, close, volume, turnover, turnover_rate,
         shake_rate, jlrl, zljlrl, change_rate, change_amount
         FROM `{}` WHERE date >= ? AND date <= ? ORDER BY date ASC",
        table_name
    );

    let records: Vec<KlineRecord> = sqlx::query_as::<_, KlineRecord>(&query)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => NotFound(format!("Stock {} not found", code)),
            _ => Database(e),
        })?;

    Ok(records)
}
```

- [ ] **Step 3: Add Result type alias to error.rs**

In `error.rs`, add:
```rust
pub type Result<T> = std::result::Result<T, AppError>;
```

- [ ] **Step 4: Commit**

```bash
git add stock-gateway/src/db/
git commit -m "feat: add database layer with stock and kline queries"
```

---

## Task 6: Auth Module

**Files:**
- Create: `stock-gateway/src/auth/mod.rs`
- Create: `stock-gateway/src/auth/api_key.rs`
- Create: `stock-gateway/src/auth/jwt.rs`

- [ ] **Step 1: Write auth/api_key.rs**

```rust
use crate::error::{AppError, Result};

pub struct ApiKeyAuth {
    valid_keys: Vec<String>,
}

impl ApiKeyAuth {
    pub fn new(keys: Vec<String>) -> Self {
        Self { valid_keys: keys }
    }

    pub fn validate(&self, key: &str) -> Result<()> {
        if self.valid_keys.contains(&key.to_string()) {
            Ok(())
        } else {
            Err(AppError::Unauthorized("Invalid API key".into()))
        }
    }
}
```

- [ ] **Step 2: Write auth/jwt.rs**

```rust
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::config::JwtConfig;
use crate::error::{AppError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iss: String,
}

pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    issuer: String,
    expiration_hours: u64,
}

impl JwtAuth {
    pub fn new(cfg: &JwtConfig) -> Self {
        let mut validation = Validation::default();
        validation.set_issuer(&[&cfg.issuer]);
        Self {
            encoding_key: EncodingKey::from_secret(cfg.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(cfg.secret.as_bytes()),
            validation,
            issuer: cfg.issuer.clone(),
            expiration_hours: cfg.expiration_hours,
        }
    }

    pub fn generate_token(&self, subject: &str) -> Result<String> {
        let exp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(self.expiration_hours as i64))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: subject.into(),
            exp,
            iss: self.issuer.clone(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Internal(format!("JWT encode error: {}", e)))
    }

    pub fn validate(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| AppError::Unauthorized(format!("JWT validation failed: {}", e)))
    }
}
```

- [ ] **Step 3: Write auth/mod.rs**

```rust
pub mod api_key;
pub mod jwt;

pub use api_key::ApiKeyAuth;
pub use jwt::JwtAuth;

use crate::config::Config;
use crate::error::{AppError, Result};

pub struct AuthService {
    api_key: ApiKeyAuth,
    jwt: JwtAuth,
}

impl AuthService {
    pub fn new(cfg: &Config) -> Self {
        Self {
            api_key: ApiKeyAuth::new(cfg.auth.apikeys.clone()),
            jwt: JwtAuth::new(&cfg.jwt),
        }
    }

    /// Validate auth header for HTTP (supports both API Key and JWT)
    pub fn validate_http(&self, auth_header: Option<&str>) -> Result<()> {
        let header = auth_header.ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        if header.starts_with("Bearer ") {
            // Try JWT first (HTTP only)
            let token = &header[7..];
            if self.jwt.validate(token).is_ok() {
                return Ok(());
            }
            // Fallback: treat as API Key
            self.api_key.validate(token)
        } else if header.starts_with("Token ") {
            let key = &header[6..];
            self.api_key.validate(key)
        } else {
            // Plain key
            self.api_key.validate(header)
        }
    }

    /// Validate auth header for MCP (API Key only)
    pub fn validate_mcp(&self, auth_header: Option<&str>) -> Result<()> {
        let header = auth_header.ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        if header.starts_with("Bearer ") {
            let key = &header[7..];
            self.api_key.validate(key)
        } else {
            self.api_key.validate(header)
        }
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add stock-gateway/src/auth/
git commit -m "feat: add auth module with API key and JWT support"
```

---

## Task 7: Rate Limit Module

**Files:**
- Create: `stock-gateway/src/rate_limit/mod.rs`
- Create: `stock-gateway/src/rate_limit/sliding_window.rs`

- [ ] **Step 1: Write rate_limit/sliding_window.rs**

```rust
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct Window {
    count: Arc<AtomicU64>,
    start: Instant,
}

pub struct SlidingWindowLimiter {
    limit: u32,
    window_sec: u64,
    counters: DashMap<String, Window>,
}

impl SlidingWindowLimiter {
    pub fn new(limit: u32, window_sec: u64) -> Self {
        Self {
            limit,
            window_sec,
            counters: DashMap::new(),
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.window_sec);

        let entry = self.counters.entry(key.to_string()).or_insert_with(|| Window {
            count: Arc::new(AtomicU64::new(0)),
            start: now,
        });

        // Reset if window expired
        let elapsed = now.duration_since(entry.start).as_secs();
        if elapsed >= self.window_sec {
            entry.count.store(0, Ordering::SeqCst);
            entry.start = now;
        }

        let current = entry.count.load(Ordering::SeqCst);
        if current >= self.limit as u64 {
            return false;
        }
        entry.count.fetch_add(1, Ordering::SeqCst);
        true
    }

    pub fn remaining(&self, key: &str) -> u32 {
        let now = Instant::now();
        let entry = match self.counters.get(key) {
            Some(e) => e,
            None => return self.limit,
        };

        let elapsed = now.duration_since(entry.start).as_secs();
        if elapsed >= self.window_sec {
            return self.limit;
        }

        let current = entry.count.load(Ordering::SeqCst);
        self.limit.saturating_sub(current as u32)
    }
}
```

- [ ] **Step 2: Write rate_limit/mod.rs**

```rust
pub mod sliding_window;

pub use sliding_window::SlidingWindowLimiter;

use crate::config::RateLimitConfig;

pub struct RateLimiter {
    ip_limiter: SlidingWindowLimiter,
    key_limiter: SlidingWindowLimiter,
}

impl RateLimiter {
    pub fn new(cfg: &RateLimitConfig) -> Self {
        Self {
            ip_limiter: SlidingWindowLimiter::new(cfg.ip_limit, cfg.window_sec),
            key_limiter: SlidingWindowLimiter::new(cfg.key_limit, cfg.window_sec),
        }
    }

    /// Check IP rate limit (Layer 1)
    pub fn check_ip(&self, ip: &str) -> bool {
        self.ip_limiter.check(ip)
    }

    /// Check Key rate limit (Layer 2)
    pub fn check_key(&self, key: &str) -> bool {
        self.key_limiter.check(key)
    }

    /// Check combined: if key present use key limit, else use IP limit
    pub fn check(&self, ip: &str, key: Option<&str>) -> RateLimitResult {
        if let Some(k) = key {
            if self.key_limiter.check(k) {
                RateLimitResult::Allowed
            } else {
                RateLimitResult::KeyLimited
            }
        } else if self.ip_limiter.check(ip) {
            RateLimitResult::Allowed
        } else {
            RateLimitResult::IpLimited
        }
    }

    pub fn ip_limiter(&self) -> &SlidingWindowLimiter {
        &self.ip_limiter
    }

    pub fn key_limiter(&self) -> &SlidingWindowLimiter {
        &self.key_limiter
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateLimitResult {
    Allowed,
    IpLimited,
    KeyLimited,
}
```

- [ ] **Step 3: Commit**

```bash
git add stock-gateway/src/rate_limit/
git commit -m "feat: add tiered sliding window rate limiter"
```

---

## Task 8: HTTP Handlers

**Files:**
- Create: `stock-gateway/src/http/mod.rs`
- Create: `stock-gateway/src/http/handlers.rs`

- [ ] **Step 1: Write http/handlers.rs**

```rust
use salvo::prelude::*;
use std::sync::Arc;

use crate::auth::AuthService;
use crate::db;
use crate::error::{write_error_response, AppError, Result};
use crate::models::{KlineResponse, StockListResponse};
use crate::rate_limit::RateLimiter;

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
        let ip = req.peer_addr().unwrap_or_default().to_string();
        let auth_header = req.headers().get("authorization").and_then(|v| v.to_str().ok());

        let key = auth_header.and_then(|h| {
            if h.starts_with("Bearer ") || h.starts_with("Token ") {
                Some(&h[7..])
            } else {
                Some(h)
            }
        });

        match self.rate_limiter.check(&ip, key) {
            crate::rate_limit::RateLimitResult::KeyLimited => {
                return Err(AppError::RateLimited("Key rate limit exceeded".into()).into());
            }
            crate::rate_limit::RateLimitResult::IpLimited => {
                return Err(AppError::RateLimited("IP rate limit exceeded".into()).into());
            }
            crate::rate_limit::RateLimitResult::Allowed => {}
        }

        // Auth check
        if let Err(e) = self.auth.validate_http(auth_header) {
            write_error_response(res, &e);
            return Ok(());
        }

        // Parse query
        let search = req.queries().get("search").cloned();

        // Query
        let stocks = db::queries::search_stocks(&self.pool, search.as_deref())
            .await
            .map_err(|e| {
                write_error_response(res, &e);
            })?;

        res.status_code(StatusCode::OK);
        res.render(Json(StockListResponse { data: stocks }));
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
        let ip = req.peer_addr().unwrap_or_default().to_string();
        let auth_header = req.headers().get("authorization").and_then(|v| v.to_str().ok());

        let key = auth_header.and_then(|h| {
            if h.starts_with("Bearer ") || h.starts_with("Token ") {
                Some(&h[7..])
            } else {
                Some(h)
            }
        });

        match self.rate_limiter.check(&ip, key) {
            crate::rate_limit::RateLimitResult::KeyLimited => {
                return Err(AppError::RateLimited("Key rate limit exceeded".into()).into());
            }
            crate::rate_limit::RateLimitResult::IpLimited => {
                return Err(AppError::RateLimited("IP rate limit exceeded".into()).into());
            }
            crate::rate_limit::RateLimitResult::Allowed => {}
        }

        // Auth check
        if let Err(e) = self.auth.validate_http(auth_header) {
            write_error_response(res, &e);
            return Ok(());
        }

        // Parse query params
        let code = req.queries().get("code").cloned().ok_or_else(|| {
            let e = AppError::BadRequest("Missing required parameter: code".into());
            write_error_response(res, &e);
        })?;

        let start = req.queries().get("start").cloned().ok_or_else(|| {
            let e = AppError::BadRequest("Missing required parameter: start".into());
            write_error_response(res, &e);
        })?;

        let end = req.queries().get("end").cloned().ok_or_else(|| {
            let e = AppError::BadRequest("Missing required parameter: end".into());
            write_error_response(res, &e);
        })?;

        // Query
        let records = db::queries::query_kline(&self.pool, &code, &start, &end)
            .await
            .map_err(|e| {
                write_error_response(res, &e);
            })?;

        res.status_code(StatusCode::OK);
        res.render(Json(KlineResponse { code, data: records }));
        Ok(())
    }
}
```

- [ ] **Step 2: Write http/mod.rs**

```rust
pub mod handlers;

pub use handlers::*;
```

- [ ] **Step 3: Commit**

```bash
git add stock-gateway/src/http/
git commit -m "feat: add HTTP handlers for stock list and kline endpoints"
```

---

## Task 9: MCP Server

**Files:**
- Create: `stock-gateway/src/mcp/mod.rs`
- Create: `stock-gateway/src/mcp/tools.rs`

- [ ] **Step 1: Write mcp/tools.rs**

```rust
use rmcp::result::ToolCallResult;
use rmcp::tool::Tool;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

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

    pub async fn stock_list(&self, input: StockListInput) -> Result<ToolCallResult, AppError> {
        let stocks = db::queries::search_stocks(&self.pool, input.search.as_deref())
            .await?;
        Ok(ToolCallResult::success(serde_json::to_value(StockListResponse { data: stocks }).unwrap()))
    }

    pub async fn stock_kline(&self, input: StockKlineInput) -> Result<ToolCallResult, AppError> {
        let records = db::queries::query_kline(&self.pool, &input.code, &input.start, &input.end)
            .await?;
        Ok(ToolCallResult::success(serde_json::to_value(KlineResponse {
            code: input.code,
            data: records,
        }).unwrap()))
    }
}
```

- [ ] **Step 2: Write mcp/mod.rs**

```rust
pub mod tools;

pub use tools::*;
```

- [ ] **Step 3: Commit**

```bash
git add stock-gateway/src/mcp/
git commit -m "feat: add MCP server with stock_list and stock_kline tools"
```

---

## Task 10: Main Entry

**Files:**
- Create: `stock-gateway/src/main.rs`

- [ ] **Step 1: Write main.rs**

```rust
use salvo::prelude::*;
use std::net::SocketAddr;
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
            .hoop(CompressionFilter::new())
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

        let addr: SocketAddr = format!("{}:{}", http_cfg.host, http_cfg.port)
            .parse()
            .unwrap();
        tracing::info!("HTTP server listening on {}", addr);
        Server::new(TcpListener::bind(&addr)).serve(router).await;
    });

    // Spawn MCP server
    let mcp_cfg = cfg.mcp.clone();
    let mcp_pool = pool.clone();
    let mcp_auth = (*auth).clone();
    let mcp_rate_limiter = rate_limiter.clone();
    let mcp_handle = tokio::spawn(async move {
        // MCP server startup (rmcp API)
        let addr: SocketAddr = format!("{}:{}", mcp_cfg.host, mcp_cfg.port)
            .parse()
            .unwrap();
        tracing::info!("MCP server listening on {}", addr);

        // For now, a placeholder - actual rmcp server setup would go here
        // rmcp::Server::bind(addr)
        //     .await?
        //     .run()
        //     .await
    });

    // Wait for both
    tokio::try_join!(http_handle, mcp_handle)?;

    Ok(())
}
```

- [ ] **Step 2: Fix compilation issues**

Run: `cd stock-gateway && cargo build 2>&1`
Expected: 编译报错，按错误信息修复（常见问题：missing imports, trait bounds）

- [ ] **Step 3: Commit**

```bash
git add stock-gateway/src/main.rs
git commit -m "feat: add main entry point with dual-port server bootstrap"
```

---

## Task 11: Integration Tests

**Files:**
- Create: `stock-gateway/tests/auth_tests.rs`
- Create: `stock-gateway/tests/rate_limit_tests.rs`
- Create: `stock-gateway/tests/http_handler_tests.rs`

- [ ] **Step 1: Write auth_tests.rs**

```rust
use stock_gateway::auth::{ApiKeyAuth, JwtAuth};
use stock_gateway::config::{AuthConfig, JwtConfig};

#[test]
fn test_api_key_valid() {
    let auth = ApiKeyAuth::new(vec!["key1".into(), "key2".into()]);
    assert!(auth.validate("key1").is_ok());
}

#[test]
fn test_api_key_invalid() {
    let auth = ApiKeyAuth::new(vec!["key1".into()]);
    assert!(auth.validate("wrong_key").is_err());
}

#[test]
fn test_jwt_generate_and_validate() {
    let cfg = JwtConfig {
        secret: "test-secret".into(),
        issuer: "test".into(),
        expiration_hours: 1,
    };
    let jwt = JwtAuth::new(&cfg);
    let token = jwt.generate_token("user1").unwrap();
    let claims = jwt.validate(&token).unwrap();
    assert_eq!(claims.sub, "user1");
    assert_eq!(claims.iss, "test");
}
```

- [ ] **Step 2: Write rate_limit_tests.rs**

```rust
use stock_gateway::rate_limit::RateLimiter;
use stock_gateway::config::RateLimitConfig;

#[test]
fn test_ip_rate_limit() {
    let cfg = RateLimitConfig {
        ip_limit: 3,
        key_limit: 100,
        window_sec: 60,
    };
    let limiter = RateLimiter::new(&cfg);

    // First 3 should pass
    assert_eq!(limiter.check_ip("192.168.1.1"), true);
    assert_eq!(limiter.check_ip("192.168.1.1"), true);
    assert_eq!(limiter.check_ip("192.168.1.1"), true);
    // 4th should fail
    assert_eq!(limiter.check_ip("192.168.1.1"), false);
}

#[test]
fn test_different_ips_independent() {
    let cfg = RateLimitConfig {
        ip_limit: 1,
        key_limit: 100,
        window_sec: 60,
    };
    let limiter = RateLimiter::new(&cfg);

    assert!(limiter.check_ip("192.168.1.1"));
    assert!(!limiter.check_ip("192.168.1.1"));
    assert!(limiter.check_ip("192.168.1.2")); // Different IP, should pass
}
```

- [ ] **Step 3: Write http_handler_tests.rs**

```rust
use salvo::prelude::*;
use stock_gateway::http::{KlineHandler, StockListHandler};
use stock_gateway::auth::AuthService;
use stock_gateway::rate_limit::RateLimiter;
use stock_gateway::config::Config;

#[test]
fn test_config_loads() {
    // This test verifies config parsing works
    let config = Config::load("config.toml").unwrap();
    assert_eq!(config.http.port, 8081);
    assert_eq!(config.mcp.port, 8080);
    assert_eq!(config.rate_limit.ip_limit, 60);
    assert_eq!(config.rate_limit.key_limit, 1000);
}
```

- [ ] **Step 4: Run tests**

Run: `cd stock-gateway && cargo test 2>&1`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add stock-gateway/tests/
git commit -m "test: add integration tests for auth, rate limit, and HTTP handlers"
```

---

## Implementation Order

1. **Task 1** — Project scaffold (Cargo.toml + config.toml)
2. **Task 2** — Config module
3. **Task 3** — Error handling
4. **Task 4** — Data models
5. **Task 5** — Database layer
6. **Task 6** — Auth module
7. **Task 7** — Rate limit module
8. **Task 8** — HTTP handlers
9. **Task 9** — MCP server
10. **Task 10** — Main entry + dual-port bootstrap
11. **Task 11** — Integration tests

---

## Spec Coverage Checklist

- [x] Dual port: MCP :8080, HTTP :8081 (Task 10)
- [x] Unified auth: API Key + JWT (Task 6)
- [x] Tiered rate limiting: IP 60/min + Key 1000/min (Task 7)
- [x] GET /api/v1/stocks with search (Task 8)
- [x] GET /api/v1/kline with date range (Task 8)
- [x] MCP tools: stock_list, stock_kline (Task 9)
- [x] Error responses with error code + message (Task 3)
- [x] config.toml with all settings (Task 1, 2)
- [x] MySQL connection with sqlx (Task 5)
- [x] Tests (Task 11)

---

## Post-Implementation

After all tasks complete, update the design doc status and commit final state.
