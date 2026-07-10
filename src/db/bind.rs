use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum BindError {
    #[error("{0}")]
    Invalid(String),
}

pub fn bind_pg_params<'q>(
    sql: &'q str,
    params: &[Value],
) -> Result<sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>, BindError> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = bind_pg_value(q, p)?;
    }
    Ok(q)
}

pub fn bind_mysql_params<'q>(
    sql: &'q str,
    params: &[Value],
) -> Result<sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>, BindError> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = bind_mysql_value(q, p)?;
    }
    Ok(q)
}

pub fn bind_sqlite_params<'q>(
    sql: &'q str,
    params: &[Value],
) -> Result<sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>, BindError> {
    let mut q = sqlx::query(sql);
    for p in params {
        q = bind_sqlite_value(q, p)?;
    }
    Ok(q)
}

fn bind_pg_value<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    p: &Value,
) -> Result<sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>, BindError> {
    let q = match p {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b),
        Value::Number(n) => bind_number(q, n)?,
        Value::String(s) => q.bind(s.clone()),
        Value::Array(_) | Value::Object(_) => {
            return Err(BindError::Invalid(
                "array and object parameters are not supported".into(),
            ));
        }
    };
    Ok(q)
}

fn bind_mysql_value<'q>(
    q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    p: &Value,
) -> Result<sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>, BindError> {
    let q = match p {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b),
        Value::Number(n) => bind_number(q, n)?,
        Value::String(s) => q.bind(s.clone()),
        Value::Array(_) | Value::Object(_) => {
            return Err(BindError::Invalid(
                "array and object parameters are not supported".into(),
            ));
        }
    };
    Ok(q)
}

fn bind_sqlite_value<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    p: &Value,
) -> Result<sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>, BindError> {
    let q = match p {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b),
        Value::Number(n) => bind_number(q, n)?,
        Value::String(s) => q.bind(s.clone()),
        Value::Array(_) | Value::Object(_) => {
            return Err(BindError::Invalid(
                "array and object parameters are not supported".into(),
            ));
        }
    };
    Ok(q)
}

fn bind_number<'q, DB>(
    q: sqlx::query::Query<'q, DB, <DB as sqlx::Database>::Arguments<'q>>,
    n: &serde_json::Number,
) -> Result<sqlx::query::Query<'q, DB, <DB as sqlx::Database>::Arguments<'q>>, BindError>
where
    DB: sqlx::Database,
    for<'a> i64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    for<'a> f64: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
{
    let q = if let Some(i) = n.as_i64() {
        q.bind(i)
    } else if let Some(f) = n.as_f64() {
        q.bind(f)
    } else if let Some(u) = n.as_u64() {
        q.bind(i64::try_from(u).map_err(|_| {
            BindError::Invalid(format!("parameter value out of range: {u}"))
        })?)
    } else {
        return Err(BindError::Invalid("invalid numeric parameter".into()));
    };
    Ok(q)
}
