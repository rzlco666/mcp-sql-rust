mod classify;

use sqlparser::ast::Statement;
use sqlparser::dialect::{Dialect, MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;

use crate::config::WriteMode;
use crate::db::EngineKind;

pub use classify::{SqlClass, StmtClass};

#[derive(Debug, Clone)]
pub struct PreparedSql {
    pub sql: String,
    pub limit_injected: bool,
    pub class: StmtClass,
}

#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    #[error("SQL guard: {0}")]
    Denied(String),
}

pub fn validate_and_prepare(
    sql: &str,
    engine: EngineKind,
    write_mode: WriteMode,
    default_limit: u32,
) -> Result<PreparedSql, GuardError> {
    let dialect: Box<dyn Dialect> = match engine {
        EngineKind::Postgres => Box::new(PostgreSqlDialect {}),
        EngineKind::Mysql => Box::new(MySqlDialect {}),
    };

    let statements = Parser::parse_sql(dialect.as_ref(), sql)
        .map_err(|e| GuardError::Denied(format!("parse error: {e}")))?;

    if statements.is_empty() {
        return Err(GuardError::Denied("empty query".into()));
    }
    if statements.len() > 1 {
        return Err(GuardError::Denied(
            "multiple statements are not allowed in a single query string".into(),
        ));
    }

    let stmt = &statements[0];
    let class = classify::classify(stmt)?;

    if class.requires_writes_for_explain() && !write_mode.allows_dml() {
        return Err(GuardError::Denied(
            "EXPLAIN ANALYZE executes the query and requires --allow-writes".into(),
        ));
    }

    if let sqlparser::ast::Statement::Explain { statement: inner, .. } = stmt {
        let inner_class = classify::classify(inner)?;
        enforce_write_mode(&inner_class, write_mode)?;
    }

    enforce_write_mode(&class, write_mode)?;

    let needs_limit = class.is_select_like()
        && matches!(stmt, Statement::Query(q) if q.limit_clause.is_none() && q.fetch.is_none());

    let trimmed = strip_trailing_semicolons(sql);
    let final_sql = if needs_limit {
        format!("{trimmed} LIMIT {default_limit}")
    } else {
        trimmed.to_string()
    };

    Ok(PreparedSql {
        sql: final_sql,
        limit_injected: needs_limit,
        class,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_drop_in_readonly() {
        let err = validate_and_prepare(
            "DROP TABLE users",
            EngineKind::Postgres,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap_err();
        assert!(err.to_string().contains("DDL blocked"));
    }

    #[test]
    fn allows_select_with_limit_injection() {
        let p = validate_and_prepare("SELECT 1", EngineKind::Postgres, WriteMode::ReadOnly, 100)
            .unwrap();
        assert!(p.limit_injected);
        assert!(p.sql.ends_with("LIMIT 100"));
    }

    #[test]
    fn allows_insert_with_writes() {
        validate_and_prepare(
            "INSERT INTO t VALUES (1)",
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
            EngineKind::Mysql,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap_err();
        assert!(err.to_string().contains("multiple statements"));
    }
}
