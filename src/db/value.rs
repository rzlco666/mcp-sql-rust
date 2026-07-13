use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde_json::Value;
use sqlx::{mysql::MySqlRow, postgres::PgRow, sqlite::SqliteRow, Column, Row, TypeInfo, ValueRef};

pub fn bigdecimal_to_json(v: &BigDecimal) -> Value {
    if v.fractional_digit_count() == 0 {
        if let Some(i) = v.to_string().parse::<i64>().ok() {
            if (-(1i64 << 53)..=(1i64 << 53)).contains(&i) {
                return Value::from(i);
            }
        }
    }
    if let Ok(f) = v.to_string().parse::<f64>() {
        if f.is_finite() {
            return Value::from(f);
        }
    }
    Value::String(v.to_string())
}

fn bytes_to_hex(v: &[u8]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}

fn mysql_type_is_bool(type_name: &str) -> bool {
    type_name.eq_ignore_ascii_case("BOOL") || type_name.eq_ignore_ascii_case("BOOLEAN")
}

pub fn decode_mysql_cell(row: &MySqlRow, index: usize) -> Result<Value, sqlx::Error> {
    let raw = row.try_get_raw(index)?;
    if raw.is_null() {
        return Ok(Value::Null);
    }

    if let Ok(v) = row.try_get::<i64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<u64, _>(index) {
        if v <= (1u64 << 53) {
            return Ok(Value::from(v));
        }
        return Ok(Value::String(v.to_string()));
    }
    if let Ok(v) = row.try_get::<i32, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<u32, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<f32, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<BigDecimal, _>(index) {
        return Ok(bigdecimal_to_json(&v));
    }
    if let Ok(v) = row.try_get::<NaiveDate, _>(index) {
        return Ok(Value::String(v.to_string()));
    }
    if let Ok(v) = row.try_get::<NaiveTime, _>(index) {
        return Ok(Value::String(v.to_string()));
    }
    if let Ok(v) = row.try_get::<NaiveDateTime, _>(index) {
        return Ok(Value::String(v.to_string()));
    }
    if let Ok(v) = row.try_get::<DateTime<Utc>, _>(index) {
        return Ok(Value::String(v.to_rfc3339()));
    }
    if let Ok(v) = row.try_get::<Vec<u8>, _>(index) {
        return Ok(Value::String(bytes_to_hex(&v)));
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

pub fn decode_pg_cell(row: &PgRow, index: usize) -> Result<Value, sqlx::Error> {
    let raw = row.try_get_raw(index)?;
    if raw.is_null() {
        return Ok(Value::Null);
    }

    if let Ok(v) = row.try_get::<i64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<i32, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(index) {
        return Ok(Value::from(v));
    }
    if let Ok(v) = row.try_get::<BigDecimal, _>(index) {
        return Ok(bigdecimal_to_json(&v));
    }
    if let Ok(v) = row.try_get::<bool, _>(index) {
        return Ok(Value::Bool(v));
    }
    if let Ok(v) = row.try_get::<Value, _>(index) {
        return Ok(v);
    }
    if let Ok(v) = row.try_get::<String, _>(index) {
        return Ok(Value::String(v));
    }
    Ok(Value::Null)
}

pub fn decode_sqlite_cell(row: &SqliteRow, index: usize) -> Result<Value, sqlx::Error> {
    let raw = row.try_get_raw(index)?;
    if raw.is_null() {
        return Ok(Value::Null);
    }

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
    fn bigdecimal_integer_fits_as_number() {
        let v: BigDecimal = "184".parse().unwrap();
        assert_eq!(bigdecimal_to_json(&v), Value::from(184));
    }

    #[test]
    fn bigdecimal_decimal_as_float() {
        let v: BigDecimal = "107744.5".parse().unwrap();
        let json = bigdecimal_to_json(&v);
        assert!(json.is_number());
    }

    #[test]
    fn mysql_type_is_bool_recognizes_boolean_types() {
        assert!(mysql_type_is_bool("BOOL"));
        assert!(mysql_type_is_bool("BOOLEAN"));
        assert!(!mysql_type_is_bool("BIGINT"));
        assert!(!mysql_type_is_bool("TINYINT"));
    }
}
