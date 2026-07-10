use serde_json::Value;

use crate::db::EngineKind;
use crate::guard::GuardError;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScanState {
    Normal,
    SingleQuote,
    DoubleQuote,
    Backtick,
    LineComment,
    BlockComment,
    DollarQuote,
}

/// Count `?` placeholders outside string literals and comments.
pub fn count_question_mark_placeholders(sql: &str) -> usize {
    let mut count = 0;
    scan_sql(sql, |event| {
        if event == ScanEvent::QuestionMark {
            count += 1;
        }
    });
    count
}

/// Count PostgreSQL `$1`, `$2`, … placeholders outside literals (when no `?` present).
pub fn count_pg_numbered_placeholders(sql: &str) -> usize {
    let mut max_idx = 0usize;
    scan_sql(sql, |event| {
        if let ScanEvent::PgNumbered(n) = event {
            max_idx = max_idx.max(n);
        }
    });
    max_idx
}

pub fn placeholder_count(sql: &str, engine: EngineKind) -> usize {
    let qm = count_question_mark_placeholders(sql);
    if qm > 0 {
        return qm;
    }
    if engine == EngineKind::Postgres {
        return count_pg_numbered_placeholders(sql);
    }
    0
}

pub fn validate_param_count(sql: &str, params: &[Value], engine: EngineKind) -> Result<(), GuardError> {
    let qm = count_question_mark_placeholders(sql);
    let pg_num = count_pg_numbered_placeholders(sql);

    if qm > 0 && pg_num > 0 {
        return Err(GuardError::Denied(
            "mixing ? and $N placeholders is not allowed".into(),
        ));
    }

    let expected = if qm > 0 {
        qm
    } else if engine == EngineKind::Postgres {
        pg_num
    } else if pg_num > 0 {
        return Err(GuardError::Denied(
            "PostgreSQL $N placeholders are only supported on PostgreSQL sources".into(),
        ));
    } else {
        0
    };

    if expected == 0 && !params.is_empty() {
        return Err(GuardError::Denied("unexpected params for SQL without placeholders".into()));
    }
    if params.len() != expected {
        return Err(GuardError::Denied(format!(
            "placeholder count mismatch: expected {expected}, got {}",
            params.len()
        )));
    }
    Ok(())
}

/// Replace `?` with `$1`, `$2`, … outside string literals (PostgreSQL execution path).
pub fn rewrite_placeholders_for_postgres(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len() + 8);
    let mut index = 1usize;
    let bytes = sql.as_bytes();
    let mut i = 0usize;
    let mut state = ScanState::Normal;
    let mut dollar_tag: Option<String> = None;

    while i < bytes.len() {
        let c = bytes[i] as char;
        match state {
            ScanState::Normal if c == '?' => {
                out.push('$');
                out.push_str(&index.to_string());
                index += 1;
            }
            ScanState::Normal => match c {
                '\'' => {
                    out.push(c);
                    state = ScanState::SingleQuote;
                }
                '"' => {
                    out.push(c);
                    state = ScanState::DoubleQuote;
                }
                '`' => {
                    out.push(c);
                    state = ScanState::Backtick;
                }
                '-' if bytes.get(i + 1) == Some(&b'-') => {
                    out.push(c);
                    state = ScanState::LineComment;
                }
                '/' if bytes.get(i + 1) == Some(&b'*') => {
                    out.push(c);
                    state = ScanState::BlockComment;
                }
                '$' => {
                    if let Some((tag, end)) = parse_dollar_tag(bytes, i) {
                        dollar_tag = Some(tag);
                        for b in &bytes[i..end] {
                            out.push(*b as char);
                        }
                        i = end;
                        state = ScanState::DollarQuote;
                        continue;
                    }
                    out.push(c);
                }
                _ => out.push(c),
            },
            ScanState::SingleQuote => {
                out.push(c);
                if c == '\'' {
                    if bytes.get(i + 1) == Some(&b'\'') {
                        out.push('\'');
                        i += 1;
                    } else {
                        state = ScanState::Normal;
                    }
                }
            }
            ScanState::DoubleQuote => {
                out.push(c);
                if c == '"' {
                    if bytes.get(i + 1) == Some(&b'"') {
                        out.push('"');
                        i += 1;
                    } else {
                        state = ScanState::Normal;
                    }
                }
            }
            ScanState::Backtick => {
                out.push(c);
                if c == '`' {
                    state = ScanState::Normal;
                }
            }
            ScanState::LineComment => {
                out.push(c);
                if c == '\n' {
                    state = ScanState::Normal;
                }
            }
            ScanState::BlockComment => {
                out.push(c);
                if c == '*' && bytes.get(i + 1) == Some(&b'/') {
                    out.push('/');
                    i += 1;
                    state = ScanState::Normal;
                }
            }
            ScanState::DollarQuote => {
                out.push(c);
                if c == '$' {
                    let tag = dollar_tag.as_deref().unwrap_or("");
                    let tag_bytes = tag.as_bytes();
                    let end = i + 1 + tag_bytes.len() + 1;
                    if end <= bytes.len()
                        && bytes[i + 1..i + 1 + tag_bytes.len()] == tag_bytes[..][..]
                        && bytes.get(i + 1 + tag_bytes.len()) == Some(&b'$')
                    {
                        for b in &bytes[i + 1..=i + 1 + tag_bytes.len()] {
                            out.push(*b as char);
                        }
                        i = i + 1 + tag_bytes.len();
                        dollar_tag = None;
                        state = ScanState::Normal;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScanEvent {
    QuestionMark,
    PgNumbered(usize),
    Char(char),
}

fn scan_sql<F>(sql: &str, mut on_event: F)
where
    F: FnMut(ScanEvent),
{
    scan_sql_with_output(sql, &mut (), |event| {
        on_event(event);
    });
}

fn scan_sql_with_output<F, W>(sql: &str, _writer: &mut W, mut on_event: F)
where
    F: FnMut(ScanEvent),
{
    let bytes = sql.as_bytes();
    let mut i = 0usize;
    let mut state = ScanState::Normal;
    let mut dollar_tag: Option<String> = None;

    while i < bytes.len() {
        let c = bytes[i] as char;

        match state {
            ScanState::Normal => match c {
                '\'' => {
                    on_event(ScanEvent::Char(c));
                    state = ScanState::SingleQuote;
                }
                '"' => {
                    on_event(ScanEvent::Char(c));
                    state = ScanState::DoubleQuote;
                }
                '`' => {
                    on_event(ScanEvent::Char(c));
                    state = ScanState::Backtick;
                }
                '?' => on_event(ScanEvent::QuestionMark),
                '-' if bytes.get(i + 1) == Some(&b'-') => {
                    on_event(ScanEvent::Char(c));
                    state = ScanState::LineComment;
                }
                '/' if bytes.get(i + 1) == Some(&b'*') => {
                    on_event(ScanEvent::Char(c));
                    state = ScanState::BlockComment;
                }
                '$' => {
                    if let Some((tag, end)) = parse_dollar_tag(bytes, i) {
                        dollar_tag = Some(tag);
                        for b in &bytes[i..end] {
                            on_event(ScanEvent::Char(*b as char));
                        }
                        i = end;
                        state = ScanState::DollarQuote;
                        continue;
                    }
                    if let Some(n) = parse_pg_number(bytes, i) {
                        on_event(ScanEvent::PgNumbered(n));
                        i += 1 + n.to_string().len();
                        continue;
                    }
                    on_event(ScanEvent::Char(c));
                }
                _ => on_event(ScanEvent::Char(c)),
            },
            ScanState::SingleQuote => {
                on_event(ScanEvent::Char(c));
                if c == '\'' {
                    if bytes.get(i + 1) == Some(&b'\'') {
                        on_event(ScanEvent::Char('\''));
                        i += 1;
                    } else {
                        state = ScanState::Normal;
                    }
                }
            }
            ScanState::DoubleQuote => {
                on_event(ScanEvent::Char(c));
                if c == '"' {
                    if bytes.get(i + 1) == Some(&b'"') {
                        on_event(ScanEvent::Char('"'));
                        i += 1;
                    } else {
                        state = ScanState::Normal;
                    }
                }
            }
            ScanState::Backtick => {
                on_event(ScanEvent::Char(c));
                if c == '`' {
                    state = ScanState::Normal;
                }
            }
            ScanState::LineComment => {
                on_event(ScanEvent::Char(c));
                if c == '\n' {
                    state = ScanState::Normal;
                }
            }
            ScanState::BlockComment => {
                on_event(ScanEvent::Char(c));
                if c == '*' && bytes.get(i + 1) == Some(&b'/') {
                    on_event(ScanEvent::Char('/'));
                    i += 1;
                    state = ScanState::Normal;
                }
            }
            ScanState::DollarQuote => {
                on_event(ScanEvent::Char(c));
                if c == '$' {
                    let tag = dollar_tag.as_deref().unwrap_or("");
                    let tag_bytes = tag.as_bytes();
                    let end = i + 1 + tag_bytes.len() + 1;
                    if end <= bytes.len()
                        && bytes[i + 1..i + 1 + tag_bytes.len()] == tag_bytes[..]
                        && bytes.get(i + 1 + tag_bytes.len()) == Some(&b'$')
                    {
                        for b in &bytes[i + 1..=i + 1 + tag_bytes.len()] {
                            on_event(ScanEvent::Char(*b as char));
                        }
                        i = i + 1 + tag_bytes.len();
                        dollar_tag = None;
                        state = ScanState::Normal;
                    }
                }
            }
        }
        i += 1;
    }
}

/// Returns `(tag, end_index_exclusive)` for opening `$tag$` or `$$`.
fn parse_dollar_tag(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    if bytes.get(start) != Some(&b'$') {
        return None;
    }
    let mut j = start + 1;
    while j < bytes.len() && bytes[j] != b'$' {
        let ch = bytes[j] as char;
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return None;
        }
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b'$' {
        return None;
    }
    let tag = String::from_utf8_lossy(&bytes[start + 1..j]).into_owned();
    Some((tag, j + 1))
}

fn parse_pg_number(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'$') {
        return None;
    }
    let mut j = start + 1;
    if j >= bytes.len() || !bytes[j].is_ascii_digit() {
        return None;
    }
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    let num = std::str::from_utf8(&bytes[start + 1..j])
        .ok()?
        .parse()
        .ok()?;
    Some(num)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_question_marks_outside_strings() {
        assert_eq!(count_question_mark_placeholders("SELECT * FROM t WHERE id = ?"), 1);
        assert_eq!(
            count_question_mark_placeholders("SELECT '?' AS q, ? AS p"),
            1
        );
        assert_eq!(count_question_mark_placeholders("SELECT 1"), 0);
    }

    #[test]
    fn rewrites_question_marks_for_postgres() {
        assert_eq!(
            rewrite_placeholders_for_postgres("SELECT * FROM t WHERE id = ? AND x = ?"),
            "SELECT * FROM t WHERE id = $1 AND x = $2"
        );
        assert_eq!(
            rewrite_placeholders_for_postgres("SELECT '?' AS q WHERE id = ?"),
            "SELECT '?' AS q WHERE id = $1"
        );
    }

    #[test]
    fn validates_param_count() {
        validate_param_count("SELECT 1", &[], EngineKind::Mysql).unwrap();
        validate_param_count("SELECT ? AS v", &[Value::from(1)], EngineKind::Mysql).unwrap();
        let err = validate_param_count("SELECT 1", &[Value::from(1)], EngineKind::Mysql).unwrap_err();
        assert!(err.to_string().contains("unexpected params"));
        let err =
            validate_param_count("SELECT ?", &[], EngineKind::Mysql).unwrap_err();
        assert!(err.to_string().contains("mismatch"));
    }

    #[test]
    fn rejects_mixed_placeholder_styles() {
        let err = validate_param_count(
            "SELECT * FROM t WHERE id = $1 AND x = ?",
            &[Value::from(1), Value::from(2)],
            EngineKind::Postgres,
        )
        .unwrap_err();
        assert!(err.to_string().contains("mixing"));
    }

    #[test]
    fn pg_native_numbered_placeholders() {
        assert_eq!(
            count_pg_numbered_placeholders("SELECT * FROM t WHERE id = $1 AND x = $2"),
            2
        );
        validate_param_count(
            "SELECT * FROM t WHERE id = $1",
            &[Value::from(42)],
            EngineKind::Postgres,
        )
        .unwrap();
    }
}
