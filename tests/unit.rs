#[cfg(test)]
mod guard_tests {
    use mcp_sql_rust::config::WriteMode;
    use mcp_sql_rust::db::EngineKind;
    use mcp_sql_rust::guard::validate_and_prepare;
    use serde_json::json;

    #[test]
    fn drop_blocked_without_db() {
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
    fn select_gets_limit() {
        let p =
            validate_and_prepare("SELECT 1", &[], EngineKind::Postgres, WriteMode::ReadOnly, 100)
                .unwrap();
        assert!(p.sql.contains("LIMIT 100"));
    }

    #[test]
    fn batch_strings_are_single_statement_only() {
        let err = validate_and_prepare(
            "SELECT 1; SELECT 2",
            &[],
            EngineKind::Postgres,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap_err();
        assert!(err.to_string().contains("multiple statements"));
    }

    #[test]
    fn show_processlist_allowed_on_mysql() {
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
    fn clamps_user_limit_to_max_rows() {
        let p = validate_and_prepare(
            "SELECT 1 LIMIT 200",
            &[],
            EngineKind::Mysql,
            WriteMode::ReadOnly,
            50,
        )
        .unwrap();
        assert!(p.sql.contains("LIMIT 50"));
        assert!(p.limit_clamped);
        assert!(!p.limit_injected);
    }

    #[test]
    fn param_count_mismatch_rejected() {
        let err = validate_and_prepare(
            "SELECT * FROM t WHERE id = ?",
            &[],
            EngineKind::Mysql,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap_err();
        assert!(err.to_string().contains("mismatch"));
    }

    #[test]
    fn unexpected_params_rejected() {
        let err = validate_and_prepare(
            "SELECT 1",
            &[json!(1)],
            EngineKind::Mysql,
            WriteMode::ReadOnly,
            100,
        )
        .unwrap_err();
        assert!(err.to_string().contains("unexpected params"));
    }
}

#[cfg(test)]
mod execute_params_tests {
    use mcp_sql_rust::tools::core::{BatchQueryItem, ExecuteSqlParams};

    #[test]
    fn batch_deserializes_legacy_strings() {
        let raw = r#"{"queries":["SELECT 1","SELECT 2"]}"#;
        let p: ExecuteSqlParams = serde_json::from_str(raw).unwrap();
        let items = p.queries.unwrap();
        assert_eq!(items.len(), 2);
        match &items[0] {
            BatchQueryItem::Legacy(s) => assert_eq!(s, "SELECT 1"),
            BatchQueryItem::Parameterized { .. } => panic!("expected legacy"),
        }
    }

    #[test]
    fn batch_deserializes_parameterized_objects() {
        let raw = r#"{"queries":[{"sql":"SELECT ? AS v","params":[42]}]}"#;
        let p: ExecuteSqlParams = serde_json::from_str(raw).unwrap();
        let items = p.queries.unwrap();
        match &items[0] {
            BatchQueryItem::Parameterized { sql, params } => {
                assert_eq!(sql, "SELECT ? AS v");
                assert_eq!(params.as_ref().unwrap()[0], serde_json::json!(42));
            }
            BatchQueryItem::Legacy(_) => panic!("expected parameterized"),
        }
    }
}

#[cfg(test)]
mod format_tests {
    use mcp_sql_rust::format::{truncate_to_bytes, ColumnarMeta, ColumnarResult};
    use serde_json::json;

    #[test]
    fn truncates_large_payload() {
        let rows: Vec<Vec<serde_json::Value>> = (0..1000)
            .map(|i| vec![json!(i), json!("x".repeat(200))])
            .collect();
        let result = ColumnarResult {
            cols: vec!["id".into(), "data".into()],
            rows,
            meta: ColumnarMeta {
                n: 1000,
                truncated: false,
                rows_affected: None,
                limit_injected: None,
                limit_clamped: None,
            },
        };
        let trimmed = truncate_to_bytes(result, 4096);
        assert!(trimmed.meta.truncated);
        assert!(trimmed.rows.len() < 1000);
    }
}

#[cfg(test)]
mod config_tests {
    use mcp_sql_rust::config::{detect_engine, WriteMode};

    #[test]
    fn write_mode_tiers() {
        assert!(!WriteMode::ReadOnly.allows_dml());
        assert!(WriteMode::AllowWrites.allows_dml());
        assert!(WriteMode::AllowDdl.allows_ddl());
    }

    #[test]
    fn detects_postgres_url() {
        assert_eq!(
            detect_engine(None, "postgresql://localhost/db").unwrap(),
            mcp_sql_rust::db::EngineKind::Postgres
        );
    }
}
