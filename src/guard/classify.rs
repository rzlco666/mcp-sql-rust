use sqlparser::ast::Statement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlClass {
    Read,
    Dml,
    Ddl,
    Txn,
    Other,
}

#[derive(Debug, Clone)]
pub struct StmtClass {
    class: SqlClass,
    label: String,
    explain_analyze: bool,
}

impl StmtClass {
    pub fn sql_class(&self) -> SqlClass {
        self.class
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn is_select_like(&self) -> bool {
        matches!(self.class, SqlClass::Read) && self.label == "SELECT"
    }

    pub fn requires_writes_for_explain(&self) -> bool {
        self.explain_analyze
    }
}

pub fn classify(stmt: &Statement) -> Result<StmtClass, crate::guard::GuardError> {
    match stmt {
        Statement::Query(_) => Ok(StmtClass {
            class: SqlClass::Read,
            label: "SELECT".into(),
            explain_analyze: false,
        }),

        Statement::Explain {
            analyze, statement, ..
        } => {
            let inner = classify(statement)?;
            Ok(StmtClass {
                class: SqlClass::Read,
                label: if *analyze {
                    "EXPLAIN ANALYZE".into()
                } else {
                    "EXPLAIN".into()
                },
                explain_analyze: *analyze || inner.requires_writes_for_explain(),
            })
        }

        Statement::ExplainTable { .. } => Ok(StmtClass {
            class: SqlClass::Read,
            label: "DESCRIBE".into(),
            explain_analyze: false,
        }),

        Statement::ShowTables { .. }
        | Statement::ShowColumns { .. }
        | Statement::ShowVariable { .. }
        | Statement::ShowVariables { .. }
        | Statement::ShowSchemas { .. }
        | Statement::ShowDatabases { .. }
        | Statement::ShowCatalogs { .. }
        | Statement::ShowCreate { .. }
        | Statement::ShowViews { .. }
        | Statement::ShowFunctions { .. }
        | Statement::ShowStatus { .. }
        | Statement::ShowCollation { .. } => Ok(StmtClass {
            class: SqlClass::Read,
            label: "SHOW".into(),
            explain_analyze: false,
        }),

        Statement::Insert(_) => Ok(dml("INSERT")),
        Statement::Update { .. } => Ok(dml("UPDATE")),
        Statement::Delete(_) => Ok(dml("DELETE")),
        Statement::Merge { .. } => Ok(dml("MERGE")),

        Statement::Truncate { .. } => Ok(ddl("TRUNCATE")),
        Statement::Drop { .. } => Ok(ddl("DROP")),
        Statement::AlterTable { .. } => Ok(ddl("ALTER TABLE")),
        Statement::AlterIndex { .. } => Ok(ddl("ALTER INDEX")),
        Statement::CreateTable(_) => Ok(ddl("CREATE TABLE")),
        Statement::CreateView { .. } => Ok(ddl("CREATE VIEW")),
        Statement::CreateIndex(_) => Ok(ddl("CREATE INDEX")),
        Statement::CreateSchema { .. } => Ok(ddl("CREATE SCHEMA")),
        Statement::CreateDatabase { .. } => Ok(ddl("CREATE DATABASE")),
        Statement::CreateFunction(_) => Ok(ddl("CREATE FUNCTION")),
        Statement::Grant { .. } => Ok(ddl("GRANT")),
        Statement::Revoke { .. } => Ok(ddl("REVOKE")),

        Statement::StartTransaction { .. }
        | Statement::Commit { .. }
        | Statement::Rollback { .. }
        | Statement::Savepoint { .. } => Ok(StmtClass {
            class: SqlClass::Txn,
            label: "TRANSACTION".into(),
            explain_analyze: false,
        }),

        other => {
            let dbg = format!("{other:?}");
            let head = dbg
                .split(['(', ' ', '{'])
                .next()
                .unwrap_or("statement")
                .to_string();
            Ok(StmtClass {
                class: SqlClass::Other,
                label: head,
                explain_analyze: false,
            })
        }
    }
}

fn dml(label: &str) -> StmtClass {
    StmtClass {
        class: SqlClass::Dml,
        label: label.into(),
        explain_analyze: false,
    }
}

fn ddl(label: &str) -> StmtClass {
    StmtClass {
        class: SqlClass::Ddl,
        label: label.into(),
        explain_analyze: false,
    }
}
