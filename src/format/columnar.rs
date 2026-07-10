use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct ColumnarResult {
    pub cols: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub meta: ColumnarMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnarMeta {
    pub n: usize,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows_affected: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_injected: Option<bool>,
}

impl ColumnarResult {
    pub fn empty_command(rows_affected: u64) -> Self {
        Self {
            cols: vec![],
            rows: vec![],
            meta: ColumnarMeta {
                n: 0,
                truncated: false,
                rows_affected: Some(rows_affected),
                limit_injected: None,
            },
        }
    }
}

pub fn truncate_to_bytes(mut result: ColumnarResult, max_bytes: usize) -> ColumnarResult {
    let json = serde_json::to_string(&result).unwrap_or_default();
    if json.len() <= max_bytes {
        return result;
    }

    while result.rows.len() > 1 {
        result.rows.pop();
        result.meta.truncated = true;
        result.meta.n = result.rows.len();
        let json = serde_json::to_string(&result).unwrap_or_default();
        if json.len() <= max_bytes {
            break;
        }
    }
    result
}

pub fn to_json_text(value: &impl Serialize) -> String {
    serde_json::to_string(value).unwrap_or_else(|e| format!(r#"{{"error":"{e}"}}"#))
}
