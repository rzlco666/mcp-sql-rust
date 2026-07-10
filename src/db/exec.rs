use std::time::Duration;

use futures::stream::{self, StreamExt};
use serde::Serialize;
use serde_json::Value;
use sqlx::{mysql::MySqlRow, postgres::PgRow, Column, Row};

use crate::config::WriteMode;
use crate::db::{EngineKind, EnginePool};
use crate::format::{truncate_to_bytes, ColumnarMeta, ColumnarResult};
use crate::guard::{validate_and_prepare, GuardError};

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error(transparent)]
    Guard(#[from] GuardError),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ColumnarResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchResult {
    pub results: Vec<QueryResult>,
}

#[derive(Debug, Clone)]
pub struct ExecOptions {
    pub write_mode: WriteMode,
    pub max_rows: u32,
    pub max_bytes: usize,
    pub timeout: Duration,
    pub limit_injected: bool,
}

pub async fn execute_query(
    pool: &EnginePool,
    sql: &str,
    opts: &ExecOptions,
) -> Result<QueryResult, ExecError> {
    let engine = pool.engine();
    let prepared = validate_and_prepare(sql, engine, opts.write_mode, opts.max_rows)?;

    if prepared.class.requires_writes_for_explain() && !opts.write_mode.allows_dml() {
        return Ok(QueryResult {
            ok: false,
            data: None,
            error: Some(
                "EXPLAIN ANALYZE executes the query and requires --allow-writes".into(),
            ),
        });
    }

    let result = tokio::time::timeout(opts.timeout, run_sql(pool, &prepared.sql, engine))
        .await
        .map_err(|_| ExecError::Other("query timeout".into()))??;

    let mut columnar = result;
    if prepared.limit_injected {
        columnar.meta.limit_injected = Some(true);
    }
    let columnar = truncate_to_bytes(columnar, opts.max_bytes);

    Ok(QueryResult {
        ok: true,
        data: Some(columnar),
        error: None,
    })
}

pub async fn execute_batch(
    pool: &EnginePool,
    queries: Vec<String>,
    opts: &ExecOptions,
    concurrency: usize,
    fail_fast: bool,
) -> BatchResult {
    let results = stream::iter(queries.into_iter())
        .map(|sql| {
            let pool = pool.clone();
            let opts = opts.clone();
            async move {
                match execute_query(&pool, &sql, &opts).await {
                    Ok(r) => r,
                    Err(e) => QueryResult {
                        ok: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

    if fail_fast {
        if let Some(failed) = results.iter().find(|r| !r.ok) {
            return BatchResult {
                results: vec![failed.clone()],
            };
        }
    }

    BatchResult { results }
}

async fn run_sql(
    pool: &EnginePool,
    sql: &str,
    engine: EngineKind,
) -> Result<ColumnarResult, ExecError> {
    match engine {
        EngineKind::Postgres => {
            let pool = pool.postgres().map_err(|e| ExecError::Other(e.to_string()))?;
            if is_select_like(sql) {
                let rows = sqlx::query(sql).fetch_all(pool).await?;
                pg_rows_to_columnar(&rows)
            } else {
                let result = sqlx::query(sql).execute(pool).await?;
                Ok(ColumnarResult::empty_command(result.rows_affected()))
            }
        }
        EngineKind::Mysql => {
            let pool = pool.mysql().map_err(|e| ExecError::Other(e.to_string()))?;
            if is_select_like(sql) {
                let rows = sqlx::query(sql).fetch_all(pool).await?;
                mysql_rows_to_columnar(&rows)
            } else {
                let result = sqlx::query(sql).execute(pool).await?;
                Ok(ColumnarResult::empty_command(result.rows_affected()))
            }
        }
    }
}

fn is_select_like(sql: &str) -> bool {
    let upper = sql.trim().to_uppercase();
    upper.starts_with("SELECT")
        || upper.starts_with("WITH")
        || upper.starts_with("SHOW")
        || upper.starts_with("EXPLAIN")
        || upper.starts_with("DESCRIBE")
}

fn pg_rows_to_columnar(rows: &[PgRow]) -> Result<ColumnarResult, ExecError> {
    if rows.is_empty() {
        return Ok(empty_columnar());
    }

    let cols: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    let mut out_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let mut values = Vec::with_capacity(cols.len());
        for (i, _) in row.columns().iter().enumerate() {
            values.push(pg_value(row, i)?);
        }
        out_rows.push(values);
    }

    let n = out_rows.len();
    Ok(ColumnarResult {
        cols,
        rows: out_rows,
        meta: ColumnarMeta {
            n,
            truncated: false,
            rows_affected: None,
            limit_injected: None,
        },
    })
}

fn mysql_rows_to_columnar(rows: &[MySqlRow]) -> Result<ColumnarResult, ExecError> {
    if rows.is_empty() {
        return Ok(empty_columnar());
    }

    let cols: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    let mut out_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let mut values = Vec::with_capacity(cols.len());
        for (i, _) in row.columns().iter().enumerate() {
            values.push(mysql_value(row, i)?);
        }
        out_rows.push(values);
    }

    let n = out_rows.len();
    Ok(ColumnarResult {
        cols,
        rows: out_rows,
        meta: ColumnarMeta {
            n,
            truncated: false,
            rows_affected: None,
            limit_injected: None,
        },
    })
}

fn empty_columnar() -> ColumnarResult {
    ColumnarResult {
        cols: vec![],
        rows: vec![],
        meta: ColumnarMeta {
            n: 0,
            truncated: false,
            rows_affected: None,
            limit_injected: None,
        },
    }
}

fn pg_value(row: &PgRow, index: usize) -> Result<Value, ExecError> {
    if let Ok(v) = row.try_get::<bool, _>(index) {
        return Ok(Value::Bool(v));
    }
    if let Ok(v) = row.try_get::<i64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<Value, _>(index) {
        return Ok(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return Ok(Value::String(v));
    }
    Ok(Value::Null)
}

fn mysql_value(row: &MySqlRow, index: usize) -> Result<Value, ExecError> {
    if let Ok(v) = row.try_get::<bool, _>(index) {
        return Ok(Value::Bool(v));
    }
    if let Ok(v) = row.try_get::<i64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return Ok(Value::String(v));
    }
    Ok(Value::Null)
}
