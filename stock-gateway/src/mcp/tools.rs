use std::sync::Arc;

use rmcp::{
    model::{self, CallToolResult, ErrorData as McpError, ServerCapabilities, ServerInfo, Implementation, ProtocolVersion},
    tool, tool_handler,
    handler::server::wrapper::Parameters,
    RoleServer, ServerHandler,
    service::RequestContext,
    tool_router,
    schemars,
};

use crate::auth::AuthService;
use crate::db;
use crate::models::{KlineResponse, StockListResponse};

pub struct StockMcpService {
    pool: sqlx::MySqlPool,
    auth: Arc<AuthService>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StockListParams {
    pub search: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StockKlineParams {
    pub code: String,
    pub start: String,
    pub end: String,
}

#[tool_router]
impl StockMcpService {
    pub fn new(pool: sqlx::MySqlPool, auth: Arc<AuthService>) -> Self {
        Self { pool, auth }
    }

    #[tool(description = "Search and list stocks by optional search term")]
    async fn stock_list(
        &self,
        Parameters(params): Parameters<StockListParams>,
    ) -> Result<CallToolResult, McpError> {
        let stocks = db::queries::search_stocks(&self.pool, params.search.as_deref())
            .await
            .map_err(|e| McpError::internal_error("database error", Some(serde_json::json!({ "error": e.to_string() }))))?;

        let response = StockListResponse { data: stocks };
        let json = serde_json::to_value(response).unwrap();
        Ok(CallToolResult::structured(json))
    }

    #[tool(description = "Query kline (candlestick) data for a specific stock")]
    async fn stock_kline(
        &self,
        Parameters(params): Parameters<StockKlineParams>,
    ) -> Result<CallToolResult, McpError> {
        let records = db::queries::query_kline(&self.pool, &params.code, &params.start, &params.end)
            .await
            .map_err(|e| McpError::internal_error("database error", Some(serde_json::json!({ "error": e.to_string() }))))?;

        let response = KlineResponse {
            code: params.code,
            data: records,
        };
        let json = serde_json::to_value(response).unwrap();
        Ok(CallToolResult::structured(json))
    }
}

#[tool_handler]
impl ServerHandler for StockMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions("Stock Gateway MCP server. Tools: stock_list (search stocks), stock_kline (query kline data).".to_string())
    }

    async fn initialize(
        &self,
        _request: model::InitializeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<model::InitializeResult, McpError> {
        Ok(self.get_info())
    }
}
