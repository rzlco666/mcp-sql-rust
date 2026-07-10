mod classify;
mod params;

use serde_json::Value;
use sqlparser::ast::{
    Expr, LimitClause, Query, Statement, Value as SqlValue,
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

#[derive(Debug, Clone)]
pub struct PreparedSql {
    pub sql: String,
    pub params: Vec<Value>,
    pub limit_injected: bool,
    pub limit_clamped: bool,
    pub class: StmtClass,
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

    if class.requires_writes_for_explain() && !write_mode.allows_dml() {
        return Err(GuardError::Denied(
            "EXPLAIN ANALYZE executes the query and requires --allow-writes".into(),
        ));
    }

    if let Statement::Explain { statement: inner, .. } = &stmt {
        let inner_class = classify::classify(inner)?;
        enforce_write_mode(&inner_class, write_mode)?;
    }

    enforce_write_mode(&class, write_mode)?;

    let needs_limit = class.is_select_like()
        && matches!(&stmt, Statement::Query(q) if q.limit_clause.is_none() && q.fetch.is_none());

    let trimmed = strip_trailing_semicolons(&normalized);
    let mut stmt = stmt;
    let limit_clamped = clamp_query_limit(&mut stmt, default_limit);

    let final_sql = if needs_limit {
        format!("{trimmed} LIMIT {default_limit}")
    } else if limit_clamped {
        stmt.to_string()
    } else {
        trimmed.to_string()
    };

    validate_param_count(&final_sql, query_params, engine)?;

    Ok(PreparedSql {
        sql: final_sql,
        params: query_params.to_vec(),
        limit_injected: needs_limit,
        limit_clamped,
        class,
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
            "statement type not allowed: {}",
            class.label()
        ))),
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
        let p = validate_and_prepare("SELECT 1", &[], EngineKind::Postgres, WriteMode::ReadOnly, 100)
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
}
