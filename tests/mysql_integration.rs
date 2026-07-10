//! MySQL integration tests — require a live database.
//!
//! Run with: `MYSQL_DATABASE_URL=mysql://... cargo test --test mysql_integration -- --ignored`

use std::time::Duration;

use mcp_sql_rust::config::WriteMode;
use mcp_sql_rust::db::{describe_table, execute_query, ExecOptions, EnginePool};
use mcp_sql_rust::guard::validate_and_prepare;
use mcp_sql_rust::db::EngineKind;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::Row;

async fn mysql_pool() -> Option<EnginePool> {
    let url = std::env::var("MYSQL_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()?;
    if !url.to_lowercase().starts_with("mysql://") {
        return None;
    }
    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .ok()?;
    Some(EnginePool::Mysql(pool))
}

fn exec_opts() -> ExecOptions {
    ExecOptions {
        write_mode: WriteMode::ReadOnly,
        max_rows: 100,
        max_bytes: 64 * 1024,
        timeout: Duration::from_secs(10),
        limit_injected: false,
    }
}

#[tokio::test]
#[ignore = "requires MYSQL_DATABASE_URL or mysql:// DATABASE_URL"]
async fn mysql_describe_table_returns_columns() {
    let pool = mysql_pool()
        .await
        .expect("set MYSQL_DATABASE_URL to a mysql:// DSN");

    let row = sqlx::query("SELECT DATABASE()")
        .fetch_one(pool.mysql().unwrap())
        .await
        .expect("DATABASE()");
    let schema: String = row.try_get(0).expect("schema name");

    let tables = sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = ? AND table_type = 'BASE TABLE' LIMIT 1",
    )
    .bind(&schema)
    .fetch_all(pool.mysql().unwrap())
    .await
    .expect("list tables");
    let table: String = tables[0].try_get(0).expect("table name");

    let object = describe_table(&pool, Some(&schema), &table)
        .await
        .expect("describe_table");
    let columns = object.columns.expect("columns field");
    assert!(
        !columns.is_empty(),
        "describe_table should return columns for {schema}.{table}"
    );
}

#[tokio::test]
#[ignore = "requires MYSQL_DATABASE_URL or mysql:// DATABASE_URL"]
async fn mysql_count_serializes_as_number_not_bool() {
    let pool = mysql_pool()
        .await
        .expect("set MYSQL_DATABASE_URL to a mysql:// DSN");

    for sql in ["SELECT COUNT(*) AS c FROM (SELECT 1 AS x) t", "SELECT 0 AS c"] {
        let result = execute_query(&pool, sql, &exec_opts())
            .await
            .expect("execute")
            .data
            .expect("data");
        let value = &result.rows[0][0];
        assert!(
            value.is_number(),
            "expected numeric JSON for `{sql}`, got {value}"
        );
    }
}

#[tokio::test]
#[ignore = "requires MYSQL_DATABASE_URL or mysql:// DATABASE_URL"]
async fn mysql_show_processlist_allowed_and_runs() {
    validate_and_prepare("SHOW PROCESSLIST", EngineKind::Mysql, WriteMode::ReadOnly, 100)
        .expect("guard should allow SHOW PROCESSLIST");

    let pool = mysql_pool()
        .await
        .expect("set MYSQL_DATABASE_URL to a mysql:// DSN");

    let result = execute_query(&pool, "SHOW PROCESSLIST", &exec_opts())
        .await
        .expect("execute");
    assert!(result.ok, "SHOW PROCESSLIST should succeed");
    assert!(result.data.is_some());
}
