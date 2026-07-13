use std::sync::Arc;

use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, WriteMode};
use crate::db::{execute_query, ExecOptions};
use crate::guard::validate_and_prepare;
use crate::tools::core::{exec_options, json_result, tool_error};

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaMutateAction {
    CreateTable,
    DropTable,
    AddColumn,
    AlterColumn,
    DropColumn,
    TruncateTable,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ColumnDef {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(default)]
    pub nullable: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SchemaMutateParams {
    pub action: SchemaMutateAction,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default)]
    pub ddl: Option<String>,
    #[serde(default)]
    pub column: Option<ColumnDef>,
    #[serde(default)]
    pub confirm: Option<bool>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
struct SchemaMutateResult {
    ok: bool,
    action: SchemaMutateAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rows_affected: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn handle_schema_mutate(
    config: &Arc<AppConfig>,
    params: SchemaMutateParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    if !config.write_mode.allows_ddl() {
        return Err(tool_error(
            "schema_mutate requires --allow-ddl on the server",
        ));
    }

    let source = config
        .source(params.source.as_deref())
        .map_err(|e| tool_error(e.to_string()))?;
    let pool = source
        .pool()
        .await
        .map_err(|e| tool_error(e.to_string()))?;

    let sql = match params.action {
        SchemaMutateAction::CreateTable => {
            let ddl = params
                .ddl
                .as_deref()
                .ok_or_else(|| tool_error("create_table requires ddl"))?;
            ddl.to_string()
        }
        SchemaMutateAction::DropTable => {
            require_confirm(&params)?;
            let table = require_table(&params)?;
            let qualified = qualify_table(params.schema.as_deref(), table);
            format!("DROP TABLE {qualified}")
        }
        SchemaMutateAction::TruncateTable => {
            require_confirm(&params)?;
            let table = require_table(&params)?;
            let qualified = qualify_table(params.schema.as_deref(), table);
            format!("TRUNCATE TABLE {qualified}")
        }
        SchemaMutateAction::AddColumn => {
            let table = require_table(&params)?;
            let column = params
                .column
                .as_ref()
                .ok_or_else(|| tool_error("add_column requires column"))?;
            let qualified = qualify_table(params.schema.as_deref(), table);
            let null_sql = match column.nullable {
                Some(true) | None => "",
                Some(false) => " NOT NULL",
            };
            format!(
                "ALTER TABLE {qualified} ADD COLUMN {} {}{}",
                quote_ident(&column.name),
                column.data_type,
                null_sql
            )
        }
        SchemaMutateAction::AlterColumn => {
            let table = require_table(&params)?;
            let column = params
                .column
                .as_ref()
                .ok_or_else(|| tool_error("alter_column requires column"))?;
            let qualified = qualify_table(params.schema.as_deref(), table);
            format!(
                "ALTER TABLE {qualified} MODIFY COLUMN {} {}",
                quote_ident(&column.name),
                column.data_type
            )
        }
        SchemaMutateAction::DropColumn => {
            require_confirm(&params)?;
            let table = require_table(&params)?;
            let column = params
                .column
                .as_ref()
                .ok_or_else(|| tool_error("drop_column requires column"))?;
            let qualified = qualify_table(params.schema.as_deref(), table);
            format!(
                "ALTER TABLE {qualified} DROP COLUMN {}",
                quote_ident(&column.name)
            )
        }
    };

    let engine = pool.engine();
    validate_and_prepare(&sql, &[], engine, WriteMode::AllowDdl, config.max_rows)
        .map_err(|e| tool_error(e.to_string()))?;

    let mut opts: ExecOptions = exec_options(config);
    opts.write_mode = WriteMode::AllowDdl;

    let result = execute_query(&pool, &sql, &[], &opts)
        .await
        .map_err(|e| tool_error(e.to_string()))?;

    if !result.ok {
        return json_result(&SchemaMutateResult {
            ok: false,
            action: params.action,
            table: params.table,
            rows_affected: None,
            error: result.error,
        });
    }

    let rows_affected = result
        .data
        .as_ref()
        .and_then(|d| d.meta.rows_affected);

    json_result(&SchemaMutateResult {
        ok: true,
        action: params.action,
        table: params.table,
        rows_affected,
        error: None,
    })
}

fn require_confirm(params: &SchemaMutateParams) -> Result<(), rmcp::ErrorData> {
    if params.confirm != Some(true) {
        return Err(tool_error(
            "destructive action requires confirm: true",
        ));
    }
    Ok(())
}

fn require_table(params: &SchemaMutateParams) -> Result<&str, rmcp::ErrorData> {
    params
        .table
        .as_deref()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| tool_error("table is required"))
}

fn qualify_table(schema: Option<&str>, table: &str) -> String {
    if let Some(schema) = schema.filter(|s| !s.is_empty()) {
        format!("{}.{}", quote_ident(schema), quote_ident(table))
    } else {
        quote_ident(table)
    }
}

fn quote_ident(name: &str) -> String {
    let trimmed = name.trim().trim_matches('`');
    format!("`{}`", trimmed.replace('`', "``"))
}
