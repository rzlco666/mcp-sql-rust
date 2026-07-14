use std::sync::Arc;

use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, WriteMode};
use crate::db::{execute_query, EngineKind, ExecOptions};
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

/// Alias-tool params (action implied by tool name).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateTableParams {
    pub ddl: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DropTableParams {
    pub table: String,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub confirm: Option<bool>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TruncateTableParams {
    pub table: String,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub confirm: Option<bool>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddColumnParams {
    pub table: String,
    pub column: ColumnDef,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AlterColumnParams {
    pub table: String,
    pub column: ColumnDef,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DropColumnParams {
    pub table: String,
    pub column: ColumnDef,
    #[serde(default)]
    pub schema: Option<String>,
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

pub async fn handle_create_table(
    config: &Arc<AppConfig>,
    params: CreateTableParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    handle_schema_mutate(
        config,
        SchemaMutateParams {
            action: SchemaMutateAction::CreateTable,
            schema: None,
            table: None,
            ddl: Some(params.ddl),
            column: None,
            confirm: None,
            source: params.source,
        },
    )
    .await
}

pub async fn handle_drop_table(
    config: &Arc<AppConfig>,
    params: DropTableParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    handle_schema_mutate(
        config,
        SchemaMutateParams {
            action: SchemaMutateAction::DropTable,
            schema: params.schema,
            table: Some(params.table),
            ddl: None,
            column: None,
            confirm: params.confirm,
            source: params.source,
        },
    )
    .await
}

pub async fn handle_truncate_table(
    config: &Arc<AppConfig>,
    params: TruncateTableParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    handle_schema_mutate(
        config,
        SchemaMutateParams {
            action: SchemaMutateAction::TruncateTable,
            schema: params.schema,
            table: Some(params.table),
            ddl: None,
            column: None,
            confirm: params.confirm,
            source: params.source,
        },
    )
    .await
}

pub async fn handle_add_column(
    config: &Arc<AppConfig>,
    params: AddColumnParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    handle_schema_mutate(
        config,
        SchemaMutateParams {
            action: SchemaMutateAction::AddColumn,
            schema: params.schema,
            table: Some(params.table),
            ddl: None,
            column: Some(params.column),
            confirm: None,
            source: params.source,
        },
    )
    .await
}

pub async fn handle_alter_column(
    config: &Arc<AppConfig>,
    params: AlterColumnParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    handle_schema_mutate(
        config,
        SchemaMutateParams {
            action: SchemaMutateAction::AlterColumn,
            schema: params.schema,
            table: Some(params.table),
            ddl: None,
            column: Some(params.column),
            confirm: None,
            source: params.source,
        },
    )
    .await
}

pub async fn handle_drop_column(
    config: &Arc<AppConfig>,
    params: DropColumnParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    handle_schema_mutate(
        config,
        SchemaMutateParams {
            action: SchemaMutateAction::DropColumn,
            schema: params.schema,
            table: Some(params.table),
            ddl: None,
            column: Some(params.column),
            confirm: params.confirm,
            source: params.source,
        },
    )
    .await
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
    let engine = pool.engine();

    let sql = build_ddl_sql(&params, engine)?;

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

fn build_ddl_sql(
    params: &SchemaMutateParams,
    engine: EngineKind,
) -> Result<String, rmcp::ErrorData> {
    match params.action {
        SchemaMutateAction::CreateTable => {
            let ddl = params
                .ddl
                .as_deref()
                .ok_or_else(|| tool_error("create_table requires ddl"))?;
            Ok(ddl.to_string())
        }
        SchemaMutateAction::DropTable => {
            require_confirm(params)?;
            let table = require_table(params)?;
            let qualified = qualify_table(params.schema.as_deref(), table, engine);
            Ok(format!("DROP TABLE {qualified}"))
        }
        SchemaMutateAction::TruncateTable => {
            require_confirm(params)?;
            let table = require_table(params)?;
            let qualified = qualify_table(params.schema.as_deref(), table, engine);
            Ok(format!("TRUNCATE TABLE {qualified}"))
        }
        SchemaMutateAction::AddColumn => {
            let table = require_table(params)?;
            let column = params
                .column
                .as_ref()
                .ok_or_else(|| tool_error("add_column requires column"))?;
            let qualified = qualify_table(params.schema.as_deref(), table, engine);
            let null_sql = match column.nullable {
                Some(true) | None => "",
                Some(false) => " NOT NULL",
            };
            Ok(format!(
                "ALTER TABLE {qualified} ADD COLUMN {} {}{}",
                quote_ident(&column.name, engine),
                column.data_type,
                null_sql
            ))
        }
        SchemaMutateAction::AlterColumn => {
            let table = require_table(params)?;
            let column = params
                .column
                .as_ref()
                .ok_or_else(|| tool_error("alter_column requires column"))?;
            let qualified = qualify_table(params.schema.as_deref(), table, engine);
            let col = quote_ident(&column.name, engine);
            match engine {
                EngineKind::Mysql => Ok(format!(
                    "ALTER TABLE {qualified} MODIFY COLUMN {col} {}",
                    column.data_type
                )),
                EngineKind::Postgres => {
                    if column.nullable.is_some() {
                        return Err(tool_error(
                            "alter_column on Postgres supports type only; omit nullable and run SET/DROP NOT NULL via execute_sql",
                        ));
                    }
                    Ok(format!(
                        "ALTER TABLE {qualified} ALTER COLUMN {col} TYPE {}",
                        column.data_type
                    ))
                }
                EngineKind::Sqlite => Err(tool_error(
                    "alter_column unsupported on SQLite; recreate the table",
                )),
            }
        }
        SchemaMutateAction::DropColumn => {
            require_confirm(params)?;
            let table = require_table(params)?;
            let column = params
                .column
                .as_ref()
                .ok_or_else(|| tool_error("drop_column requires column"))?;
            let qualified = qualify_table(params.schema.as_deref(), table, engine);
            Ok(format!(
                "ALTER TABLE {qualified} DROP COLUMN {}",
                quote_ident(&column.name, engine)
            ))
        }
    }
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

fn qualify_table(schema: Option<&str>, table: &str, engine: EngineKind) -> String {
    if let Some(schema) = schema.filter(|s| !s.is_empty()) {
        format!(
            "{}.{}",
            quote_ident(schema, engine),
            quote_ident(table, engine)
        )
    } else {
        quote_ident(table, engine)
    }
}

fn quote_ident(name: &str, engine: EngineKind) -> String {
    match engine {
        EngineKind::Mysql => {
            let trimmed = name.trim().trim_matches('`');
            format!("`{}`", trimmed.replace('`', "``"))
        }
        EngineKind::Postgres | EngineKind::Sqlite => {
            let trimmed = name.trim().trim_matches('"');
            format!("\"{}\"", trimmed.replace('"', "\"\""))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alter_column_mysql_uses_modify() {
        let params = SchemaMutateParams {
            action: SchemaMutateAction::AlterColumn,
            schema: None,
            table: Some("t".into()),
            ddl: None,
            column: Some(ColumnDef {
                name: "c".into(),
                data_type: "VARCHAR(64)".into(),
                nullable: None,
            }),
            confirm: None,
            source: None,
        };
        let sql = build_ddl_sql(&params, EngineKind::Mysql).unwrap();
        assert_eq!(sql, "ALTER TABLE `t` MODIFY COLUMN `c` VARCHAR(64)");
    }

    #[test]
    fn alter_column_postgres_uses_type() {
        let params = SchemaMutateParams {
            action: SchemaMutateAction::AlterColumn,
            schema: Some("public".into()),
            table: Some("t".into()),
            ddl: None,
            column: Some(ColumnDef {
                name: "c".into(),
                data_type: "text".into(),
                nullable: None,
            }),
            confirm: None,
            source: None,
        };
        let sql = build_ddl_sql(&params, EngineKind::Postgres).unwrap();
        assert_eq!(
            sql,
            "ALTER TABLE \"public\".\"t\" ALTER COLUMN \"c\" TYPE text"
        );
    }

    #[test]
    fn alter_column_sqlite_errors() {
        let params = SchemaMutateParams {
            action: SchemaMutateAction::AlterColumn,
            schema: None,
            table: Some("t".into()),
            ddl: None,
            column: Some(ColumnDef {
                name: "c".into(),
                data_type: "TEXT".into(),
                nullable: None,
            }),
            confirm: None,
            source: None,
        };
        assert!(build_ddl_sql(&params, EngineKind::Sqlite).is_err());
    }
}
