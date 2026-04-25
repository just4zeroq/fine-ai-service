# Stock Gateway

A high-performance stock data gateway service providing HTTP REST API and MCP (Model Context Protocol) tool interface.

## Features

- **HTTP REST API** — Query stock list and candlestick (kline) data
- **MCP Tool Server** — AI-assistant-friendly tools via MCP protocol over HTTP
- **Authentication** — API Key and JWT token support
- **Rate Limiting** — Tiered sliding window limiter (IP-level + key-level)
- **MySQL Backend** — Connection pool with async queries via SQLx

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Stock Gateway                       │
├──────────────────────┬──────────────────────────────┤
│   HTTP API (:8081)   │      MCP Server (:8080)      │
│   GET /api/v1/stocks │   POST /mcp (StreamableHTTP)│
│   GET /api/v1/kline  │   Tools: stock_list          │
│                      │          stock_kline         │
├──────────────────────┴──────────────────────────────┤
│  Auth (API Key / JWT)  │  Rate Limiter (IP + Key)  │
├─────────────────────────────────────────────────────┤
│              Database Layer (SQLx / MySQL)           │
└─────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+
- MySQL database

### Configure

Edit `config.toml`:

```toml
[mcp]
host = "0.0.0.0"
port = 8080

[http]
host = "0.0.0.0"
port = 8081

[rate_limit]
ip_limit = 60       # requests per IP per window
key_limit = 1000     # requests per key per window
window_sec = 60      # sliding window size

[auth]
apikeys = ["sk-your-key-here"]

[jwt]
secret = "your-jwt-secret"
issuer = "stock-gateway"
expiration_hours = 24

[database]
host = "localhost"
port = 3306
user = "fintools"
password = "password"
database = "cn_stocks"
```

### Run

```bash
cargo run
```

### Build

```bash
cargo build --release
```

## HTTP API

All HTTP endpoints require authentication via `X-API-Key` header.

### `GET /api/v1/stocks`

Search stocks.

**Query Parameters:**
- `search` (optional) — filter by stock code or name

**Response:**
```json
{
  "data": [
    { "code": "600000", "name": "浦发银行", "market": "SH", "type": "stock" }
  ]
}
```

### `GET /api/v1/kline`

Query candlestick (kline) data.

**Query Parameters:**
- `code` (required) — stock code, e.g. `600000`
- `start` (required) — start datetime, e.g. `2024-01-01`
- `end` (required) — end datetime, e.g. `2024-12-31`

**Response:**
```json
{
  "code": "600000",
  "data": [
    { "date": "2024-01-02", "open": 10.5, "high": 10.8, "low": 10.4, "close": 10.7, "volume": 1000000 }
  ]
}
```

## MCP Tools

Connect via Streamable HTTP at `http://host:8080/mcp`.

### `stock_list`

Search and list stocks.

```json
{
  "name": "stock_list",
  "arguments": { "search": "浦发" }
}
```

### `stock_kline`

Query kline data for a specific stock.

```json
{
  "name": "stock_kline",
  "arguments": { "code": "600000", "start": "2024-01-01", "end": "2024-12-31" }
}
```

## Testing

```bash
cargo test
```

## Project Structure

```
src/
├── main.rs              # Entry point, spawns HTTP + MCP servers
├── lib.rs               # Library exports
├── config.rs            # Configuration loading
├── error.rs             # Unified error type (AppError)
├── auth/                # API Key + JWT authentication
│   ├── mod.rs
│   ├── jwt.rs
│   └── api_key.rs
├── rate_limit/          # Tiered sliding window rate limiter
│   ├── mod.rs
│   └── sliding_window.rs
├── http/                # Salvo HTTP handlers
│   ├── mod.rs
│   ├── stock_list.rs
│   └── kline.rs
├── mcp/                 # MCP server tools
│   ├── mod.rs
│   └── tools.rs         # StockMcpService (#[tool_router])
├── db/                  # MySQL queries
│   ├── mod.rs
│   └── queries.rs
└── models/              # Data models
    └── mod.rs
```

## License

Apache-2.0
