//! SQLite integration tests — in-memory, no external database required.

use std::time::Duration;

use mcp_sql_rust::config::WriteMode;
use mcp_sql_rust::db::{analyze_query, describe_table, execute_query, list_tables, ExecOptions, EnginePool};
use mcp_sql_rust::db::EngineKind;
use mcp_sql_rust::guard::validate_and_prepare;
use serde_json::{json, Value};
use sqlx::sqlite::SqlitePoolOptions;

async fn sqlite_pool() -> EnginePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect("sqlite::memory:")
        .await
        .expect("sqlite memory pool");
    EnginePool::Sqlite(pool)
}

fn exec_opts(write_mode: WriteMode) -> ExecOptions {
    ExecOptions {
        write_mode,
        max_rows: 100,
        max_bytes: 64 * 1024,
        timeout: Duration::from_secs(10),
        limit_injected: false,
    }
}

#[tokio::test]
async fn sqlite_crud_and_parameterized_select() {
    let pool = sqlite_pool().await;
    let ddl_opts = exec_opts(WriteMode::AllowDdl);
    let write_opts = exec_opts(WriteMode::AllowWrites);
    let read_opts = exec_opts(WriteMode::ReadOnly);

    let created = execute_query(
        &pool,
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        &[],
        &ddl_opts,
    )
    .await
    .expect("create table");
    assert!(created.ok, "{:?}", created.error);

    let inserted = execute_query(
        &pool,
        "INSERT INTO users (id, name) VALUES (?, ?)",
        &[json!(1), json!("alice")],
        &write_opts,
    )
    .await
    .expect("insert");
    assert!(inserted.ok, "{:?}", inserted.error);

    let selected = execute_query(
        &pool,
        "SELECT id, name FROM users WHERE id = ?",
        &[json!(1)],
        &read_opts,
    )
    .await
    .expect("select");
    assert!(selected.ok, "{:?}", selected.error);
    let data = selected.data.expect("columnar data");
    assert_eq!(data.cols, vec!["id", "name"]);
    assert_eq!(data.rows.len(), 1);
    assert_eq!(data.rows[0][1], Value::String("alice".into()));
}

#[tokio::test]
async fn sqlite_list_tables_and_describe_table() {
    let pool = sqlite_pool().await;
    let ddl_opts = exec_opts(WriteMode::AllowDdl);

    execute_query(
        &pool,
        "CREATE TABLE items (id INTEGER PRIMARY KEY, sku TEXT)",
        &[],
        &ddl_opts,
    )
    .await
    .expect("create items");

    let tables = list_tables(&pool, Some("main"), None)
        .await
        .expect("list tables");
    assert!(tables.iter().any(|t| t.name == "items"));

    let described = describe_table(&pool, Some("main"), "items")
        .await
        .expect("describe items");
    let columns = described.columns.expect("columns");
    assert!(columns.iter().any(|c| c.name == "sku"));
    assert!(described.indexes.is_some());
}

#[tokio::test]
async fn sqlite_guard_blocks_ddl_in_read_only() {
    let err = validate_and_prepare(
        "CREATE TABLE blocked (id INTEGER)",
        &[],
        EngineKind::Sqlite,
        WriteMode::ReadOnly,
        100,
    )
    .unwrap_err();
    assert!(err.to_string().contains("DDL blocked"));
}

#[tokio::test]
async fn sqlite_analyze_query_performance() {
    let pool = sqlite_pool().await;
    let ddl_opts = exec_opts(WriteMode::AllowDdl);

    execute_query(
        &pool,
        "CREATE TABLE metrics (id INTEGER PRIMARY KEY, value REAL)",
        &[],
        &ddl_opts,
    )
    .await
    .expect("create metrics");

    let summary = analyze_query(
        &pool,
        "SELECT * FROM metrics WHERE id = 1",
        WriteMode::ReadOnly,
        Duration::from_secs(5),
    )
    .await
    .expect("explain");
    assert_eq!(summary.engine, "sqlite");
    assert!(!summary.nodes.is_empty());
}
