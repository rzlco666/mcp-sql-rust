//! MySQL integration tests — require a live database.
//!
//! Run with: `MYSQL_DATABASE_URL=mysql://... cargo test --test mysql_integration -- --ignored`

use std::time::Duration;

use mcp_sql_rust::config::WriteMode;
use mcp_sql_rust::db::{describe_table, execute_query, ExecOptions, EnginePool};
use mcp_sql_rust::db::EngineKind;
use mcp_sql_rust::guard::validate_and_prepare;
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
    let mysql = pool.mysql().unwrap();

    let row = sqlx::query("SELECT DATABASE()")
        .fetch_one(mysql)
        .await
        .expect("DATABASE()");
    let schema: String = row.try_get(0).expect("schema name");

    let tables = sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = ? AND table_type = 'BASE TABLE' LIMIT 1",
    )
    .bind(&schema)
    .fetch_all(mysql)
    .await
    .expect("list tables");
    let table: String = tables[0].try_get(0).expect("table name");

    let raw_rows = sqlx::query(
        "SELECT table_schema, table_name, column_name, data_type, is_nullable \
         FROM information_schema.columns \
         WHERE table_schema = ? AND table_name = ?",
    )
    .bind(&schema)
    .bind(&table)
    .fetch_all(mysql)
    .await
    .expect("raw column query");
    assert!(
        !raw_rows.is_empty(),
        "binding bug: information_schema returned 0 rows for {schema}.{table}"
    );

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
async fn mysql_describe_table_fw_users_if_present() {
    let pool = mysql_pool()
        .await
        .expect("set MYSQL_DATABASE_URL to a mysql:// DSN");
    let mysql = pool.mysql().unwrap();

    let schema_row = sqlx::query("SELECT DATABASE()")
        .fetch_one(mysql)
        .await
        .expect("DATABASE()");
    let schema: String = schema_row.try_get(0).expect("schema");

    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.tables \
         WHERE table_schema = ? AND table_name = 'fw_users'",
    )
    .bind(&schema)
    .fetch_one(mysql)
    .await
    .expect("table exists check");

    if exists == 0 {
        return;
    }

    let object = describe_table(&pool, Some(&schema), "fw_users")
        .await
        .expect("describe fw_users");
    let columns = object.columns.expect("columns");
    assert!(!columns.is_empty(), "fw_users should have columns");

    let qualified = describe_table(&pool, None, &format!("{schema}.fw_users"))
        .await
        .expect("describe qualified table");
    assert!(
        !qualified.columns.unwrap().is_empty(),
        "schema.table form should work"
    );
}

#[tokio::test]
#[ignore = "requires MYSQL_DATABASE_URL or mysql:// DATABASE_URL"]
async fn mysql_count_serializes_as_number_not_bool() {
    let pool = mysql_pool()
        .await
        .expect("set MYSQL_DATABASE_URL to a mysql:// DSN");

    for sql in ["SELECT COUNT(*) AS c FROM (SELECT 1 AS x) t", "SELECT 0 AS c"] {
        let result = execute_query(&pool, sql, &[], &exec_opts())
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
    validate_and_prepare("SHOW PROCESSLIST", &[], EngineKind::Mysql, WriteMode::ReadOnly, 100)
        .expect("guard should allow SHOW PROCESSLIST");

    let pool = mysql_pool()
        .await
        .expect("set MYSQL_DATABASE_URL to a mysql:// DSN");

    let result = execute_query(&pool, "SHOW PROCESSLIST", &[], &exec_opts())
        .await
        .expect("execute");
    assert!(result.ok, "SHOW PROCESSLIST should succeed");
    assert!(result.data.is_some());
}

#[tokio::test]
#[ignore = "requires MYSQL_DATABASE_URL or mysql:// DATABASE_URL"]
async fn mysql_parameterized_select_binds_value() {
    let pool = mysql_pool()
        .await
        .expect("set MYSQL_DATABASE_URL to a mysql:// DSN");

    let result = execute_query(&pool, "SELECT ? AS v", &[serde_json::json!(42)], &exec_opts())
        .await
        .expect("execute")
        .data
        .expect("data");
    assert_eq!(result.rows[0][0], serde_json::json!(42));
}
