# Stock Gateway - MCP + HTTP Unified Gateway Design

**Date**: 2026-04-25
**Project**: 基于 rmcp + salvo.rs 的 MCP/HTTP 统一网关
**Data Source**: MySQL (tushare pro 同步数据)

---

## 1. Overview

统一网关同时暴露 MCP 协议和 HTTP REST API，共用同一套鉴权和分层限流体系，提供股票列表查询和 K 线查询服务。

## 2. Architecture

```
                    ┌──────────────────────────────────┐
                    │      stock-gateway (single binary)│
                    │                                  │
TCP Client ────────▶│  :8080 MCP (rmcp)               │
(AI Client)         │  :8081 HTTP REST (salvo)          │
                    │                                  │
                    │  ┌────────────┐  ┌─────────────┐ │
                    │  │ auth       │  │ rate_limit  │ │
                    │  │ (unified)  │  │ (tiered)    │ │
                    │  └────────────┘  └─────────────┘ │
                    │                                  │
                    │  ┌────────────┐  ┌─────────────┐ │
                    │  │ stock_list│  │ kline_query│ │
                    │  └────────────┘  └─────────────┘ │
                    └──────────┬───────────────────────┘
                               │
                    ┌──────────▼───────┐
                    │     MySQL         │
                    │  stock + cn_*    │
                    └──────────────────┘
```

### Ports
- **MCP**: `:8080` (独立端口)
- **HTTP**: `:8081` (独立端口)

---

## 3. Authentication

### Auth Matrix

| Protocol | Auth Mode | Header Format |
|-----------|-----------|---------------|
| MCP | API Key only | `Authorization: Bearer <key>` in MCP metadata |
| HTTP | API Key or JWT | `Authorization: Bearer <token>` or `Authorization: Token <key>` |

### Verification Flow
1. Extract `Authorization` header from request
2. If `Bearer xxx` → attempt JWT parse (HTTP only); if fails, treat `xxx` as API Key
3. If `Token xxx` or plain `xxx` → validate as API Key
4. Key stored in config (`apikeys` list) or `apikeys` table

---

## 4. Rate Limiting (Tiered)

### Layer 1: IP Rate Limit (Anonymous)
- **60 requests / minute / IP**
- Applied when no API Key is present

### Layer 2: Key Rate Limit (Authenticated)
- **1000 requests / minute / Key**
- Applied per authenticated key, across both MCP and HTTP

### Algorithm
- Sliding window counter
- In-memory storage (DashMap)
- Configurable thresholds via `config.toml`

---

## 5. API Design

### 5.1 HTTP REST Endpoints

#### GET /api/v1/stocks
Search stock list.

**Query Parameters**:
- `search` (optional): fuzzy match on stock name or code

**Response**:
```json
{
  "data": [
    {"code": "000001", "name": "平安银行", "se": "sz", "type": "bank"},
    {"code": "600000", "name": "浦发银行", "se": "sh", "type": "bank"}
  ]
}
```

#### GET /api/v1/kline
Query daily K-line data.

**Query Parameters**:
- `code` (required): stock code (e.g., `000001`)
- `start` (required): start date `YYYY-MM-DD`
- `end` (required): end date `YYYY-MM-DD` (max 365 days from start)

**Response**:
```json
{
  "code": "000001",
  "data": [
    {
      "date": "2024-01-02",
      "open": 10.5,
      "high": 11.0,
      "low": 10.2,
      "close": 10.8,
      "volume": 123456,
      "turnover": 234567.8,
      "turnover_rate": 0.56,
      "shake_rate": 1.23,
      "jlrl": 0.45,
      "zljlrl": 0.32,
      "change_rate": 2.34,
      "change_amount": 0.25
    }
  ]
}
```

---

### 5.2 MCP Tools

Two tools exposed via MCP protocol:

| Tool Name | Description | Parameters |
|-----------|-------------|------------|
| `stock_list` | Get stock list with optional search | `search?: string` |
| `stock_kline` | Get daily K-line data | `code: string, start: string, end: string` |

---

## 6. Data Model

### MySQL Tables (Existing)

#### `stock`
```sql
CREATE TABLE stock (
  id int NOT NULL AUTO_INCREMENT,
  code varchar(20) NOT NULL,
  name varchar(100) DEFAULT NULL,
  se varchar(10) DEFAULT NULL,
  type varchar(10) DEFAULT NULL,
  updated_at datetime DEFAULT NULL,
  created_at datetime DEFAULT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY stock_code_un (code)
);
```

#### `{code}` (K-line table per stock)
```sql
CREATE TABLE {code} (
  id int NOT NULL AUTO_INCREMENT,
  date varchar(20) NOT NULL,
  open float DEFAULT NULL,
  high float DEFAULT NULL,
  low float DEFAULT NULL,
  close float DEFAULT NULL,
  volume float DEFAULT NULL,
  turnover float DEFAULT NULL,
  turnover_rate float DEFAULT NULL,
  shake_rate float DEFAULT NULL,
  jlrl float DEFAULT NULL,
  zljlrl float DEFAULT NULL,
  change_rate float DEFAULT NULL,
  change_amount float DEFAULT NULL,
  created_at datetime DEFAULT NULL,
  PRIMARY KEY (id),
  KEY ix_{code}_date (date)
);
```

---

## 7. Project Structure

```
stock-gateway/
├── Cargo.toml
├── config.toml              # Configuration file
├── src/
│   ├── main.rs              # Entry point, dual-port bootstrap
│   ├── config.rs           # Config loading
│   ├── auth/                # Auth module
│   │   ├── mod.rs
│   │   ├── api_key.rs      # API Key validation
│   │   └── jwt.rs          # JWT validation (HTTP only)
│   ├── rate_limit/          # Rate limit module
│   │   ├── mod.rs
│   │   └── sliding_window.rs
│   ├── mcp/                 # MCP Server (rmcp)
│   │   ├── mod.rs
│   │   └── tools.rs        # MCP tool definitions
│   ├── http/                # HTTP REST (salvo)
│   │   ├── mod.rs
│   │   └── handlers.rs     # HTTP route handlers
│   ├── db/                  # Database layer
│   │   ├── mod.rs
│   │   └── queries.rs       # SQL queries
│   └── models/              # Data models
│       ├── mod.rs
│       └── stock.rs
└── tests/                   # Integration tests
```

---

## 8. Configuration (config.toml)

```toml
[mcp]
host = "0.0.0.0"
port = 8080

[http]
host = "0.0.0.0"
port = 8081

[rate_limit]
ip_limit = 60           # requests per minute per IP
key_limit = 1000         # requests per minute per API key
window_sec = 60          # sliding window size in seconds

[auth]
# API keys list (can also be stored in DB)
apikeys = [
  "sk-test-key-001",
  "sk-test-key-002"
]

[jwt]
secret = "your-jwt-secret"  # required for HTTP JWT auth
issuer = "stock-gateway"

[database]
host = "127.0.0.1"
port = 3306
user = "root"
password = "password"
database = "cn_stocks"
```

---

## 9. Error Handling

All error responses follow a consistent format:

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "Rate limit exceeded. Try again in 30 seconds."
  }
}
```

| HTTP Status | Error Code | Description |
|-------------|------------|-------------|
| 401 | `UNAUTHORIZED` | Missing or invalid API key / JWT |
| 403 | `FORBIDDEN` | Valid key but insufficient permissions |
| 429 | `RATE_LIMITED` | Rate limit exceeded |
| 400 | `BAD_REQUEST` | Invalid parameters |
| 404 | `NOT_FOUND` | Stock code not found |
| 500 | `INTERNAL_ERROR` | Server error |

---

## 10. Implementation Notes

- **rmcp** for MCP protocol server
- **salvo** for HTTP REST server
- **sqlx** for async MySQL queries
- **DashMap** for in-memory rate limit counters
- **jsonwebtoken** for JWT (HTTP only)
- No external Redis; all rate limit state in-memory
