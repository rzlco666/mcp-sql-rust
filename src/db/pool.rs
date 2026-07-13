use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use sqlx::{MySql, MySqlPool, PgPool, Pool, Postgres, Sqlite, SqlitePool};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use tokio::sync::Mutex;

use crate::redact::redact_url;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

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
    inner: Arc<Mutex<Option<EnginePool>>>,
}

impl LazyEnginePool {
    pub fn new(
        url: String,
        engine: EngineKind,
        pool_size: u32,
        read_only: bool,
    ) -> Self {
        Self {
            url,
            engine,
            pool_size,
            read_only,
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
) -> Result<EnginePool> {
    match engine {
        EngineKind::Postgres => {
            let mut options = PgPoolOptions::new()
                .max_connections(pool_size)
                .acquire_timeout(CONNECT_TIMEOUT)
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
                .acquire_timeout(CONNECT_TIMEOUT);
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
                .acquire_timeout(CONNECT_TIMEOUT);
            let pool = options.connect(&connect_url).await?;
            Ok(EnginePool::Sqlite(pool))
        }
    }
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
        anyhow::bail!("cannot detect engine from URL scheme; use postgresql://, mysql://, or sqlite:")
    }
}
