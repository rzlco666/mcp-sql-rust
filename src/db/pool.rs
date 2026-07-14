use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Result};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{MySql, MySqlPool, PgPool, Pool, Postgres, Sqlite, SqlitePool};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::redact::redact_url;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 2;
/// Fail fast when the DB host/port is unreachable (before sqlx handshake).
pub const TCP_PREFLIGHT_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Postgres,
    Mysql,
    Sqlite,
}

#[derive(Debug, Clone)]
pub enum EnginePool {
    Postgres(PgPool),
    Mysql(MySqlPool),
    Sqlite(SqlitePool),
}

impl EnginePool {
    pub async fn ping(&self) -> Result<()> {
        match self {
            Self::Postgres(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            Self::Mysql(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            Self::Sqlite(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
        }
        Ok(())
    }

    pub fn engine(&self) -> EngineKind {
        match self {
            Self::Postgres(_) => EngineKind::Postgres,
            Self::Mysql(_) => EngineKind::Mysql,
            Self::Sqlite(_) => EngineKind::Sqlite,
        }
    }

    pub fn postgres(&self) -> Result<&PgPool> {
        match self {
            Self::Postgres(p) => Ok(p),
            _ => anyhow::bail!("expected postgres pool"),
        }
    }

    pub fn mysql(&self) -> Result<&MySqlPool> {
        match self {
            Self::Mysql(p) => Ok(p),
            _ => anyhow::bail!("expected mysql pool"),
        }
    }

    pub fn sqlite(&self) -> Result<&SqlitePool> {
        match self {
            Self::Sqlite(p) => Ok(p),
            _ => anyhow::bail!("expected sqlite pool"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LazyEnginePool {
    url: String,
    engine: EngineKind,
    pool_size: u32,
    read_only: bool,
    connect_timeout: Duration,
    inner: Arc<Mutex<Option<EnginePool>>>,
}

impl LazyEnginePool {
    pub fn new(
        url: String,
        engine: EngineKind,
        pool_size: u32,
        read_only: bool,
        connect_timeout: Duration,
    ) -> Self {
        Self {
            url,
            engine,
            pool_size,
            read_only,
            connect_timeout,
            inner: Arc::new(Mutex::new(None)),
        }
    }

    /// Pre-populate an already-connected pool (integration tests).
    pub fn with_pool(
        url: String,
        engine: EngineKind,
        pool: EnginePool,
        read_only: bool,
    ) -> Self {
        Self {
            url,
            engine,
            pool_size: 1,
            read_only,
            connect_timeout: CONNECT_TIMEOUT,
            inner: Arc::new(Mutex::new(Some(pool))),
        }
    }

    pub async fn get(&self) -> Result<EnginePool> {
        let mut guard = self.inner.lock().await;
        if let Some(pool) = guard.as_ref() {
            return Ok(pool.clone());
        }
        let pool = connect_pool(
            &self.url,
            self.engine,
            self.pool_size,
            self.read_only,
            self.connect_timeout,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "cannot connect to {}: {}",
                redact_url(&self.url),
                e
            )
        })?;
        *guard = Some(pool.clone());
        Ok(pool)
    }

    pub async fn ping(&self) -> Result<()> {
        self.get().await?.ping().await
    }
}

pub async fn connect_pool(
    url: &str,
    engine: EngineKind,
    pool_size: u32,
    read_only: bool,
    connect_timeout: Duration,
) -> Result<EnginePool> {
    if !matches!(engine, EngineKind::Sqlite) {
        tcp_preflight(url, engine).await?;
    }

    match engine {
        EngineKind::Postgres => {
            let mut options = PgPoolOptions::new()
                .max_connections(pool_size)
                .acquire_timeout(connect_timeout)
                .idle_timeout(Duration::from_secs(30));
            if read_only {
                options = options.after_connect(|conn, _meta| {
                    Box::pin(async move {
                        sqlx::query("SET default_transaction_read_only = on")
                            .execute(conn)
                            .await?;
                        Ok(())
                    })
                });
            }
            let pool = options.connect(url).await?;
            Ok(EnginePool::Postgres(pool))
        }
        EngineKind::Mysql => {
            let mut options = MySqlPoolOptions::new()
                .max_connections(pool_size)
                .acquire_timeout(connect_timeout);
            if read_only {
                options = options.after_connect(|conn, _meta| {
                    Box::pin(async move {
                        sqlx::query("SET SESSION TRANSACTION READ ONLY")
                            .execute(conn)
                            .await?;
                        Ok(())
                    })
                });
            }
            let pool = options.connect(url).await?;
            Ok(EnginePool::Mysql(pool))
        }
        EngineKind::Sqlite => {
            let connect_url = sqlite_connect_url(url, read_only);
            let options = SqlitePoolOptions::new()
                .max_connections(pool_size)
                .acquire_timeout(connect_timeout);
            let pool = options.connect(&connect_url).await?;
            Ok(EnginePool::Sqlite(pool))
        }
    }
}

async fn tcp_preflight(url: &str, engine: EngineKind) -> Result<()> {
    let Some((host, port)) = parse_host_port(url, engine) else {
        return Ok(());
    };
    let addr = format!("{host}:{port}");
    match tokio::time::timeout(TCP_PREFLIGHT_TIMEOUT, TcpStream::connect(&addr)).await {
        Ok(Ok(_stream)) => Ok(()),
        Ok(Err(e)) => bail!(
            "TCP preflight failed in <{}ms ({addr}): {e}",
            TCP_PREFLIGHT_TIMEOUT.as_millis()
        ),
        Err(_) => bail!(
            "TCP preflight failed in <{}ms ({addr}): timed out",
            TCP_PREFLIGHT_TIMEOUT.as_millis()
        ),
    }
}

/// Extract host/port from mysql:// or postgres(ql):// URLs. Returns None for unparsable forms.
pub fn parse_host_port(url: &str, engine: EngineKind) -> Option<(String, u16)> {
    let lower = url.to_ascii_lowercase();
    let (rest, default_port) = match engine {
        EngineKind::Mysql if lower.starts_with("mysql://") => {
            (url.get("mysql://".len()..)?, 3306u16)
        }
        EngineKind::Postgres if lower.starts_with("postgresql://") => {
            (url.get("postgresql://".len()..)?, 5432u16)
        }
        EngineKind::Postgres if lower.starts_with("postgres://") => {
            (url.get("postgres://".len()..)?, 5432u16)
        }
        _ => return None,
    };

    let after_at = rest.rsplit('@').next().unwrap_or(rest);
    let host_port = after_at.split('/').next().unwrap_or(after_at);
    let host_port = host_port.split('?').next().unwrap_or(host_port);
    // Strip IPv6 brackets if present: [::1]:5432
    if let Some(inner) = host_port.strip_prefix('[') {
        let (host, rest) = inner.split_once(']')?;
        let port = if let Some(p) = rest.strip_prefix(':') {
            p.parse().ok()?
        } else {
            default_port
        };
        return Some((host.to_string(), port));
    }
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() => (h, p.parse().unwrap_or(default_port)),
        _ => (host_port, default_port),
    };
    if host.is_empty() {
        return None;
    }
    Some((host.to_string(), port))
}

fn sqlite_connect_url(url: &str, read_only: bool) -> String {
    if !read_only {
        return url.to_string();
    }
    let lower = url.to_lowercase();
    if lower.contains("mode=ro") || lower.contains("mode=readonly") {
        return url.to_string();
    }
    if url.contains('?') {
        format!("{url}&mode=ro")
    } else {
        format!("{url}?mode=ro")
    }
}

pub type PgPoolType = Pool<Postgres>;
pub type MyPoolType = Pool<MySql>;
pub type SqlitePoolType = Pool<Sqlite>;

pub fn detect_engine_from_url(hint: Option<&str>, url: &str) -> anyhow::Result<EngineKind> {
    if let Some(h) = hint {
        return match h.to_lowercase().as_str() {
            "postgres" | "postgresql" | "pg" => Ok(EngineKind::Postgres),
            "mysql" | "mariadb" => Ok(EngineKind::Mysql),
            "sqlite" => Ok(EngineKind::Sqlite),
            other => anyhow::bail!("unknown engine hint '{other}'"),
        };
    }
    let lower = url.to_lowercase();
    if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
        Ok(EngineKind::Postgres)
    } else if lower.starts_with("mysql://") {
        Ok(EngineKind::Mysql)
    } else if lower.starts_with("sqlite:") || lower.starts_with("sqlite://") {
        Ok(EngineKind::Sqlite)
    } else {
        bail!("cannot detect engine from URL scheme; use postgresql://, mysql://, or sqlite:")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mysql_host_port() {
        assert_eq!(
            parse_host_port("mysql://u:p@127.0.0.1:3307/db", EngineKind::Mysql),
            Some(("127.0.0.1".into(), 3307))
        );
        assert_eq!(
            parse_host_port("mysql://u:p@localhost/db", EngineKind::Mysql),
            Some(("localhost".into(), 3306))
        );
    }

    #[test]
    fn parse_postgres_host_port() {
        assert_eq!(
            parse_host_port(
                "postgresql://u:p@db.example.com:5433/app",
                EngineKind::Postgres
            ),
            Some(("db.example.com".into(), 5433))
        );
    }

    #[tokio::test]
    async fn tcp_preflight_fails_fast_on_closed_port() {
        // Port 1 is typically closed / unprivileged refuse
        let err = tcp_preflight("mysql://u:p@127.0.0.1:1/db", EngineKind::Mysql)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("TCP preflight failed"),
            "unexpected error: {msg}"
        );
    }
}
