use std::sync::Arc;

use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::AppConfig;
use crate::db::{describe_table, list_indexes, list_schemas, list_tables};
use crate::tools::core::{json_result, tool_error};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SourceParams {
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DescribeTableParams {
    pub table: String,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTablesParams {
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListIndexesParams {
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

pub async fn handle_list_sources(config: &Arc<AppConfig>) -> Result<CallToolResult, rmcp::ErrorData> {
    let names: Vec<&str> = config.sources.keys().map(String::as_str).collect();
    let payload = serde_json::json!({
        "default": config.default_source,
        "sources": names,
    });
    Ok(CallToolResult::success(vec![ContentBlock::text(
        payload.to_string(),
    )]))
}

pub async fn handle_list_schemas(
    config: &Arc<AppConfig>,
    params: SourceParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let source = config
        .source(params.source.as_deref())
        .map_err(|e| tool_error(e.to_string()))?;
    let objects = list_schemas(&source.pool, None)
        .await
        .map_err(|e| tool_error(e.to_string()))?;
    json_result(&objects)
}

pub async fn handle_list_tables(
    config: &Arc<AppConfig>,
    params: ListTablesParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let source = config
        .source(params.source.as_deref())
        .map_err(|e| tool_error(e.to_string()))?;
    let objects = list_tables(
        &source.pool,
        params.schema.as_deref(),
        params.keyword.as_deref(),
    )
    .await
    .map_err(|e| tool_error(e.to_string()))?;
    json_result(&objects)
}

pub async fn handle_describe_table(
    config: &Arc<AppConfig>,
    params: DescribeTableParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let source = config
        .source(params.source.as_deref())
        .map_err(|e| tool_error(e.to_string()))?;
    let object = describe_table(
        &source.pool,
        params.schema.as_deref(),
        &params.table,
    )
    .await
    .map_err(|e| tool_error(e.to_string()))?;
    json_result(&object)
}

pub async fn handle_list_indexes(
    config: &Arc<AppConfig>,
    params: ListIndexesParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let source = config
        .source(params.source.as_deref())
        .map_err(|e| tool_error(e.to_string()))?;
    let objects = list_indexes(
        &source.pool,
        params.schema.as_deref(),
        params.table.as_deref(),
        params.keyword.as_deref(),
    )
    .await
    .map_err(|e| tool_error(e.to_string()))?;
    json_result(&objects)
}
