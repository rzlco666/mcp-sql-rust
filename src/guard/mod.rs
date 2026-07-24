mod classify;
mod params;

use serde_json::Value;
use sqlparser::ast::{
    Expr, LimitClause, Offset, OffsetRows, Query, SetExpr, Statement, TableFactor,
    Value as SqlValue,
};
use sqlparser::dialect::{Dialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;

use crate::config::WriteMode;
use crate::db::EngineKind;

pub use classify::{SqlClass, StmtClass};
pub use params::{
    count_question_mark_placeholders, placeholder_count, rewrite_placeholders_for_postgres,
    validate_param_count,
};

#[derive(Debug, Clone, Default)]
pub struct PrepareOptions {
    pub page_offset: usize,
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct PreparedSql {
    pub sql: String,
    pub params: Vec<Value>,
    pub limit_injected: bool,
    pub limit_clamped: bool,
    pub class: StmtClass,
    pub server_pagination: bool,
    pub page_size: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    #[error("SQL guard: {0}")]
    Denied(String),
}

pub fn validate_and_prepare(
    sql: &str,
    query_params: &[Value],
    engine: EngineKind,
    write_mode: WriteMode,
    default_limit: u32,
) -> Result<PreparedSql, GuardError> {
    validate_and_prepare_with_options(
        sql,
        query_params,
        engine,
        write_mode,
        default_limit,
        PrepareOptions::default(),
    )
}

pub fn validate_and_prepare_with_options(
    sql: &str,
    query_params: &[Value],
    engine: EngineKind,
    write_mode: WriteMode,
    default_limit: u32,
    prepare_opts: PrepareOptions,
) -> Result<PreparedSql, GuardError> {
    validate_param_count(sql, query_params, engine)?;

    let normalized = normalize_sql_for_engine(sql, engine);
    let dialect: Box<dyn Dialect> = match engine {
        EngineKind::Postgres => Box::new(PostgreSqlDialect {}),
        EngineKind::Mysql => Box::new(MySqlDialect {}),
        EngineKind::Sqlite => Box::new(SQLiteDialect {}),
    };

    let statements = Parser::parse_sql(dialect.as_ref(), &normalized)
        .map_err(|e| GuardError::Denied(format!("parse error: {e}")))?;

    if statements.is_empty() {
        return Err(GuardError::Denied("empty query".into()));
    }
    if statements.len() > 1 {
        return Err(GuardError::Denied(
            "multiple statements are not allowed in a single query string".into(),
        ));
    }

    let stmt = statements.into_iter().next().expect("checked len");
    let class = classify::classify(&stmt)?;
    check_query_complexity(&stmt)?;

    if class.requires_writes_for_explain() && !write_mode.allows_dml() {
        return Err(GuardError::Denied(
            "EXPLAIN ANALYZE executes the query and requires --allow-writes".into(),
        ));
    }

    if let Statement::Explain {
        statement: inner, ..
    } = &stmt
    {
        let inner_class = classify::classify(inner)?;
        enforce_write_mode(&inner_class, write_mode)?;
    }

    enforce_write_mode(&class, write_mode)?;

    let wants_pagination = prepare_opts.page_offset > 0 || prepare_opts.page_size.is_some();
    let page_size = prepare_opts.page_size.unwrap_or(default_limit as usize) as u32;

    let needs_limit = class.is_select_like()
        && !wants_pagination
        && matches!(&stmt, Statement::Query(q) if q.limit_clause.is_none() && q.fetch.is_none());

    let trimmed = strip_trailing_semicolons(&normalized);
    let mut stmt = stmt;
    let limit_clamped = if wants_pagination {
        false
    } else {
        clamp_query_limit(&mut stmt, default_limit)
    };

    let (final_sql, server_pagination) = if wants_pagination {
        if let Statement::Query(q) = &mut stmt {
            if !class.is_select_like() {
                return Err(GuardError::Denied(
                    "page_offset/page_size only supported on SELECT queries".into(),
                ));
            }
            let offset = prepare_opts.page_offset as u64;
            let fetch_limit = offset + page_size as u64 + 1;
            set_query_offset_limit(q, offset, fetch_limit);
            (stmt.to_string(), true)
        } else {
            return Err(GuardError::Denied(
                "page_offset/page_size only supported on SELECT queries".into(),
            ));
        }
    } else if needs_limit {
        (format!("{trimmed} LIMIT {default_limit}"), false)
    } else if limit_clamped {
        (stmt.to_string(), false)
    } else {
        (trimmed.to_string(), false)
    };

    validate_param_count(&final_sql, query_params, engine)?;

    Ok(PreparedSql {
        sql: final_sql,
        params: query_params.to_vec(),
        limit_injected: needs_limit,
        limit_clamped,
        class,
        server_pagination,
        page_size,
    })
}

fn normalize_sql_for_engine(sql: &str, engine: EngineKind) -> String {
    match engine {
        EngineKind::Postgres if count_question_mark_placeholders(sql) > 0 => {
            rewrite_placeholders_for_postgres(sql)
        }
        _ => sql.to_string(),
    }
}

fn enforce_write_mode(class: &StmtClass, mode: WriteMode) -> Result<(), GuardError> {
    match class.sql_class() {
        SqlClass::Read => Ok(()),
        SqlClass::Dml if mode.allows_dml() => Ok(()),
        SqlClass::Ddl if mode.allows_ddl() => Ok(()),
        SqlClass::Txn => Err(GuardError::Denied(
            "transaction control statements are not allowed".into(),
        )),
        SqlClass::Dml => Err(GuardError::Denied(
            "DML blocked in read-only mode; restart with --allow-writes".into(),
        )),
        SqlClass::Ddl => Err(GuardError::Denied(
            "DDL blocked; restart with --allow-ddl".into(),
        )),
        SqlClass::Other => Err(GuardError::Denied(format!(
            "statement type not allowed: {} (blocked for safety)",
            class.label()
        ))),
    }
}

/// Soft limits to stop agent-generated Cartesian explosions before they hit the DB.
const MAX_JOINS: u32 = 8;
const MAX_SUBQUERY_DEPTH: u32 = 5;

fn check_query_complexity(stmt: &Statement) -> Result<(), GuardError> {
    let query = match stmt {
        Statement::Query(q) => q.as_ref(),
        Statement::Explain { statement, .. } => {
            if let Statement::Query(q) = statement.as_ref() {
                q.as_ref()
            } else {
                return Ok(());
            }
        }
        _ => return Ok(()),
    };
    let (joins, depth) = complexity_of_set_expr(query.body.as_ref(), 0);
    if joins > MAX_JOINS {
        return Err(GuardError::Denied(format!(
            "query too complex: {joins} joins (max {MAX_JOINS})"
        )));
    }
    if depth > MAX_SUBQUERY_DEPTH {
        return Err(GuardError::Denied(format!(
            "query too complex: subquery depth {depth} (max {MAX_SUBQUERY_DEPTH})"
        )));
    }
    Ok(())
}

fn complexity_of_set_expr(body: &SetExpr, depth: u32) -> (u32, u32) {
    match body {
        SetExpr::Select(select) => {
            let mut joins = 0u32;
            let mut max_depth = depth;
            for twj in &select.from {
                joins = joins.saturating_add(twj.joins.len() as u32);
                let (j, d) = complexity_of_table_factor(&twj.relation, depth);
                joins = joins.saturating_add(j);
                max_depth = max_depth.max(d);
                for join in &twj.joins {
                    let (j2, d2) = complexity_of_table_factor(&join.relation, depth);
                    joins = joins.saturating_add(j2);
                    max_depth = max_depth.max(d2);
                }
            }
            if let Some(selection) = &select.selection {
                let (j, d) = complexity_of_expr(selection, depth);
                joins = joins.saturating_add(j);
                max_depth = max_depth.max(d);
            }
            (joins, max_depth)
        }
        SetExpr::Query(q) => complexity_of_set_expr(q.body.as_ref(), depth.saturating_add(1)),
        SetExpr::SetOperation { left, right, .. } => {
            let (jl, dl) = complexity_of_set_expr(left.as_ref(), depth);
            let (jr, dr) = complexity_of_set_expr(right.as_ref(), depth);
            (jl.saturating_add(jr), dl.max(dr))
        }
        _ => (0, depth),
    }
}

fn complexity_of_table_factor(factor: &TableFactor, depth: u32) -> (u32, u32) {
    match factor {
        TableFactor::Derived { subquery, .. } => {
            complexity_of_set_expr(subquery.body.as_ref(), depth.saturating_add(1))
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            let mut joins = table_with_joins.joins.len() as u32;
            let (j, d) = complexity_of_table_factor(&table_with_joins.relation, depth);
            joins = joins.saturating_add(j);
            let mut max_depth = d;
            for join in &table_with_joins.joins {
                let (j2, d2) = complexity_of_table_factor(&join.relation, depth);
                joins = joins.saturating_add(j2);
                max_depth = max_depth.max(d2);
            }
            (joins, max_depth)
        }
        _ => (0, depth),
    }
}

fn complexity_of_expr(expr: &Expr, depth: u32) -> (u32, u32) {
    match expr {
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } => {
            complexity_of_set_expr(q.body.as_ref(), depth.saturating_add(1))
        }
        Expr::Exists { subquery, .. } => {
            complexity_of_set_expr(subquery.body.as_ref(), depth.saturating_add(1))
        }
        Expr::BinaryOp { left, right, .. } => {
            let (jl, dl) = complexity_of_expr(left, depth);
            let (jr, dr) = complexity_of_expr(right, depth);
            (jl.saturating_add(jr), dl.max(dr))
        }
        Expr::UnaryOp { expr, .. } => complexity_of_expr(expr, depth),
        Expr::Nested(inner) => complexity_of_expr(inner, depth),
        _ => (0, depth),
    }
}

fn strip_trailing_semicolons(sql: &str) -> &str {
    let mut s = sql.trim();
    while let Some(rest) = s.strip_suffix(';') {
        s = rest.trim_end();
    }
    s
}

fn expr_as_limit(expr: &Expr) -> Option<u64> {
    match expr {
        Expr::Value(v) => match &v.value {
            SqlValue::Number(n, _) => n.parse().ok(),
            _ => None,
        },
        _ => None,
    }
}

fn query_limit_value(q: &Query) -> Option<u64> {
    match &q.limit_clause {
        Some(LimitClause::LimitOffset {
            limit: Some(expr), ..
        }) => expr_as_limit(expr),
        Some(LimitClause::OffsetCommaLimit { limit, .. }) => expr_as_limit(limit),
        _ => None,
    }
}

fn set_query_limit(q: &mut Query, max_rows: u32) {
    let limit_expr = Expr::value(SqlValue::Number(max_rows.to_string(), false));
    match &mut q.limit_clause {
        Some(LimitClause::LimitOffset { limit, .. }) => {
            *limit = Some(limit_expr);
        }
        Some(LimitClause::OffsetCommaLimit { limit, .. }) => {
            *limit = limit_expr;
        }
        None => {
            q.limit_clause = Some(LimitClause::LimitOffset {
                limit: Some(limit_expr),
                offset: None,
                limit_by: vec![],
            });
        }
    }
}

fn set_query_offset_limit(q: &mut Query, offset: u64, limit: u64) {
    let limit_expr = Expr::value(SqlValue::Number(limit.to_string(), false));
    let offset_expr = Offset {
        value: Expr::value(SqlValue::Number(offset.to_string(), false)),
        rows: OffsetRows::None,
    };
    q.limit_clause = Some(LimitClause::LimitOffset {
        limit: Some(limit_expr),
        offset: Some(offset_expr),
        limit_by: vec![],
    });
}

/// Returns true when an explicit LIMIT was reduced to `max_rows`.
fn clamp_query_limit(stmt: &mut Statement, max_rows: u32) -> bool {
    let Statement::Query(q) = stmt else {
        return false;
    };
    let Some(current) = query_limit_value(q) else {
        return false;
    };
    if current <= max_rows as u64 {
        return false;
    }
    set_query_limit(q, max_rows);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_drop_in_readonly() {
        let err = validate_and_prepare(
            "DROP TABLE users",
            &[],
            EngineKind::Postgres,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap_err();
        assert!(err.to_string().contains("DDL blocked"));
    }

    #[test]
    fn allows_select_with_limit_injection() {
        let p = validate_and_prepare(
            "SELECT 1",
            &[],
            EngineKind::Postgres,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap();
        assert!(p.limit_injected);
        assert!(p.sql.ends_with("LIMIT 100"));
    }

    #[test]
    fn allows_insert_with_writes() {
        validate_and_prepare(
            "INSERT INTO t VALUES (1)",
            &[],
            EngineKind::Postgres,
            WriteMode::AllowWrites,
            100,
        )
        .unwrap();
    }

    #[test]
    fn allows_show_processlist_mysql_readonly() {
        validate_and_prepare(
            "SHOW PROCESSLIST",
            &[],
            EngineKind::Mysql,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap();
    }

    #[test]
    fn blocks_batch_multi_statement_in_single_string() {
        let err = validate_and_prepare(
            "SELECT 1; DROP TABLE users",
            &[],
            EngineKind::Mysql,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap_err();
        assert!(err.to_string().contains("multiple statements"));
    }

    #[test]
    fn clamps_explicit_limit_above_max_rows() {
        let p = validate_and_prepare(
            "SELECT * FROM users LIMIT 200",
            &[],
            EngineKind::Postgres,
            WriteMode::ReadOnly,
            50,
        )
        .unwrap();
        assert!(!p.limit_injected);
        assert!(p.limit_clamped);
        assert!(p.sql.contains("LIMIT 50"));
        assert!(!p.sql.contains("LIMIT 200"));
    }

    #[test]
    fn allows_parameterized_select_mysql() {
        let p = validate_and_prepare(
            "SELECT * FROM t WHERE id = ?",
            &[Value::from(1)],
            EngineKind::Mysql,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap();
        assert_eq!(p.params.len(), 1);
        assert!(p.limit_injected);
    }

    #[test]
    fn allows_parameterized_select_postgres() {
        validate_and_prepare(
            "SELECT * FROM t WHERE id = ?",
            &[Value::from(1)],
            EngineKind::Postgres,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap();
    }

    #[test]
    fn server_pagination_injects_offset_limit() {
        let p = validate_and_prepare_with_options(
            "SELECT id FROM pages ORDER BY id",
            &[],
            EngineKind::Sqlite,
            WriteMode::ReadOnly,
            100,
            PrepareOptions {
                page_offset: 50,
                page_size: Some(25),
            },
        )
        .unwrap();
        assert!(p.server_pagination);
        assert!(p.sql.contains("OFFSET 50"));
        assert!(p.sql.contains("LIMIT 76"));
    }

    #[test]
    fn parameterized_limit_clamp_preserves_placeholder() {
        let p = validate_and_prepare(
            "SELECT * FROM t WHERE id = ? LIMIT 200",
            &[Value::from(1)],
            EngineKind::Postgres,
            WriteMode::ReadOnly,
            50,
        )
        .unwrap();
        assert!(p.limit_clamped);
        assert_eq!(placeholder_count(&p.sql, EngineKind::Postgres), 1);
    }

    #[test]
    fn blocks_copy_explicitly() {
        let err = validate_and_prepare(
            "COPY users FROM STDIN",
            &[],
            EngineKind::Postgres,
            WriteMode::AllowDdl,
            100,
        )
        .unwrap_err();
        assert!(err.to_string().contains("COPY"));
    }

    #[test]
    fn blocks_attach_database_sqlite() {
        let err = validate_and_prepare(
            "ATTACH DATABASE 'other.db' AS other",
            &[],
            EngineKind::Sqlite,
            WriteMode::AllowDdl,
            100,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("ATTACH") || err.to_string().contains("not allowed"),
            "{err}"
        );
    }

    #[test]
    fn blocks_excessive_joins() {
        let sql = "SELECT * FROM a \
            JOIN b ON a.id = b.id \
            JOIN c ON b.id = c.id \
            JOIN d ON c.id = d.id \
            JOIN e ON d.id = e.id \
            JOIN f ON e.id = f.id \
            JOIN g ON f.id = g.id \
            JOIN h ON g.id = h.id \
            JOIN i ON h.id = i.id \
            JOIN j ON i.id = j.id";
        let err = validate_and_prepare(sql, &[], EngineKind::Postgres, WriteMode::ReadOnly, 100)
            .unwrap_err();
        assert!(err.to_string().contains("joins"), "{err}");
    }
}
