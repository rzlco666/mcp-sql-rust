use std::time::{Duration, Instant};

use futures::stream::{self, StreamExt};
use serde::Serialize;
use serde_json::Value;
use sqlx::{mysql::MySqlRow, postgres::PgRow, sqlite::SqliteRow, Column, Row, TypeInfo};

use crate::config::WriteMode;
use crate::db::{EngineKind, EnginePool};
use crate::format::{truncate_to_bytes, ColumnarMeta, ColumnarResult};
use crate::guard::{
    validate_and_prepare, GuardError,
};

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
    params: &[Value],
    opts: &ExecOptions,
) -> Result<QueryResult, ExecError> {
    let engine = pool.engine();
    let prepared = validate_and_prepare(sql, params, engine, opts.write_mode, opts.max_rows)?;
    let started = Instant::now();

    if prepared.class.requires_writes_for_explain() && !opts.write_mode.allows_dml() {
        tracing::warn!(
            duration_ms = started.elapsed().as_millis() as u64,
            engine = ?engine,
            error = "explain_analyze_requires_writes",
            "query failed"
        );
        return Ok(QueryResult {
            ok: false,
            data: None,
            error: Some(
                "EXPLAIN ANALYZE executes the query and requires --allow-writes".into(),
            ),
        });
    }

    let result = match tokio::time::timeout(
        opts.timeout,
        run_sql(pool, &prepared.sql, &prepared.params, engine),
    )
    .await
    {
        Ok(Ok(columnar)) => {
            let mut columnar = columnar;
            if prepared.limit_injected {
                columnar.meta.limit_injected = Some(true);
            }
            if prepared.limit_clamped {
                columnar.meta.limit_clamped = Some(true);
            }
            let columnar = truncate_to_bytes(columnar, opts.max_bytes);
            tracing::info!(
                duration_ms = started.elapsed().as_millis() as u64,
                rows = columnar.meta.n,
                engine = ?engine,
                limit_injected = prepared.limit_injected,
                limit_clamped = prepared.limit_clamped,
                truncated = columnar.meta.truncated,
                param_count = prepared.params.len(),
                "query executed"
            );
            Ok(QueryResult {
                ok: true,
                data: Some(columnar),
                error: None,
            })
        }
        Ok(Err(e)) => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                engine = ?engine,
                error = %e,
                "query failed"
            );
            Err(e)
        }
        Err(_) => {
            tracing::warn!(
                duration_ms = started.elapsed().as_millis() as u64,
                engine = ?engine,
                error = "query_timeout",
                "query failed"
            );
            Err(ExecError::Other("query timeout".into()))
        }
    };

    result
}

pub async fn execute_batch(
    pool: &EnginePool,
    queries: Vec<(String, Vec<Value>)>,
    opts: &ExecOptions,
    concurrency: usize,
    fail_fast: bool,
) -> BatchResult {
    let results = stream::iter(queries.into_iter())
        .map(|(sql, params)| {
            let pool = pool.clone();
            let opts = opts.clone();
            async move {
                match execute_query(&pool, &sql, &params, &opts).await {
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
    params: &[Value],
    engine: EngineKind,
) -> Result<ColumnarResult, ExecError> {
    match engine {
        EngineKind::Postgres => {
            let pool = pool.postgres().map_err(|e| ExecError::Other(e.to_string()))?;
            if is_select_like(sql) {
                let rows = bind_pg_params(sql, params)?.fetch_all(pool).await?;
                pg_rows_to_columnar(&rows)
            } else {
                let result = bind_pg_params(sql, params)?.execute(pool).await?;
                Ok(ColumnarResult::empty_command(result.rows_affected()))
            }
        }
        EngineKind::Mysql => {
            let pool = pool.mysql().map_err(|e| ExecError::Other(e.to_string()))?;
            if is_select_like(sql) {
                let rows = bind_mysql_params(sql, params)?.fetch_all(pool).await?;
                mysql_rows_to_columnar(&rows)
            } else {
                let result = bind_mysql_params(sql, params)?.execute(pool).await?;
                Ok(ColumnarResult::empty_command(result.rows_affected()))
            }
        }
        EngineKind::Sqlite => {
            let pool = pool.sqlite().map_err(|e| ExecError::Other(e.to_string()))?;
            if is_select_like(sql) {
                let rows = bind_sqlite_params(sql, params)?.fetch_all(pool).await?;
                sqlite_rows_to_columnar(&rows)
            } else {
                let result = bind_sqlite_params(sql, params)?.execute(pool).await?;
                Ok(ColumnarResult::empty_command(result.rows_affected()))
            }
        }
    }
}

fn bind_pg_params<'q>(
    sql: &'q str,
    params: &[Value],
) -> Result<sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>, ExecError> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = match p {
            Value::Null => q.bind(None::<String>),
            Value::Bool(b) => q.bind(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    q.bind(i)
                } else if let Some(f) = n.as_f64() {
                    q.bind(f)
                } else if let Some(u) = n.as_u64() {
                    q.bind(i64::try_from(u).map_err(|_| {
                        ExecError::Other(format!("parameter value out of range: {u}"))
                    })?)
                } else {
                    return Err(ExecError::Other("invalid numeric parameter".into()));
                }
            }
            Value::String(s) => q.bind(s.clone()),
            Value::Array(_) | Value::Object(_) => {
                return Err(ExecError::Other(
                    "array and object parameters are not supported".into(),
                ));
            }
        };
    }
    Ok(q)
}

fn bind_mysql_params<'q>(
    sql: &'q str,
    params: &[Value],
) -> Result<sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>, ExecError> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = match p {
            Value::Null => q.bind(None::<String>),
            Value::Bool(b) => q.bind(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    q.bind(i)
                } else if let Some(f) = n.as_f64() {
                    q.bind(f)
                } else if let Some(u) = n.as_u64() {
                    q.bind(i64::try_from(u).map_err(|_| {
                        ExecError::Other(format!("parameter value out of range: {u}"))
                    })?)
                } else {
                    return Err(ExecError::Other("invalid numeric parameter".into()));
                }
            }
            Value::String(s) => q.bind(s.clone()),
            Value::Array(_) | Value::Object(_) => {
                return Err(ExecError::Other(
                    "array and object parameters are not supported".into(),
                ));
            }
        };
    }
    Ok(q)
}

fn bind_sqlite_params<'q>(
    sql: &'q str,
    params: &[Value],
) -> Result<sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>, ExecError> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = match p {
            Value::Null => q.bind(None::<String>),
            Value::Bool(b) => q.bind(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    q.bind(i)
                } else if let Some(f) = n.as_f64() {
                    q.bind(f)
                } else if let Some(u) = n.as_u64() {
                    q.bind(i64::try_from(u).map_err(|_| {
                        ExecError::Other(format!("parameter value out of range: {u}"))
                    })?)
                } else {
                    return Err(ExecError::Other("invalid numeric parameter".into()));
                }
            }
            Value::String(s) => q.bind(s.clone()),
            Value::Array(_) | Value::Object(_) => {
                return Err(ExecError::Other(
                    "array and object parameters are not supported".into(),
                ));
            }
        };
    }
    Ok(q)
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
            limit_clamped: None,
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
            limit_clamped: None,
        },
    })
}

fn sqlite_rows_to_columnar(rows: &[SqliteRow]) -> Result<ColumnarResult, ExecError> {
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
            values.push(sqlite_value(row, i)?);
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
            limit_clamped: None,
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
            limit_clamped: None,
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

fn mysql_type_is_bool(type_name: &str) -> bool {
    type_name.eq_ignore_ascii_case("BOOL") || type_name.eq_ignore_ascii_case("BOOLEAN")
}

fn mysql_value(row: &MySqlRow, index: usize) -> Result<Value, ExecError> {
    // Numeric types before bool — sqlx MySQL maps integer 0/1 to bool (e.g. COUNT(*)).
    if let Ok(v) = row.try_get::<i64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<u64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return Ok(Value::from(v));
    }
    if mysql_type_is_bool(row.column(index).type_info().name()) {
        if let Ok(v) = row.try_get::<bool, _>(index) {
            return Ok(Value::Bool(v));
        }
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return Ok(Value::String(v));
    }
    Ok(Value::Null)
}

fn sqlite_value(row: &SqliteRow, index: usize) -> Result<Value, ExecError> {
    if let Ok(v) = row.try_get::<i64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<bool, _>(index) {
        return Ok(Value::Bool(v));
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return Ok(Value::String(v));
    }
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysql_type_is_bool_recognizes_boolean_types() {
        assert!(mysql_type_is_bool("BOOL"));
        assert!(mysql_type_is_bool("BOOLEAN"));
        assert!(!mysql_type_is_bool("BIGINT"));
        assert!(!mysql_type_is_bool("TINYINT"));
    }
}
