use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;

use crate::db::EngineKind;
use crate::db::EnginePool;

pub const DEFAULT_MAX_ROWS: u32 = 100;
pub const DEFAULT_MAX_BYTES: usize = 64 * 1024;
pub const DEFAULT_POOL_SIZE: u32 = 10;
pub const DEFAULT_QUERY_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_BATCH_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WriteMode {
    ReadOnly,
    AllowWrites,
    AllowDdl,
}

impl WriteMode {
    pub fn allows_dml(self) -> bool {
        matches!(self, Self::AllowWrites | Self::AllowDdl)
    }

    pub fn allows_ddl(self) -> bool {
        matches!(self, Self::AllowDdl)
    }
}

#[derive(Debug, Parser)]
#[command(name = "mcp-sql-rust", version, about = "Token-efficient MCP server for MySQL, PostgreSQL, and SQLite")]
pub struct Cli {
    /// Database connection URL (overrides .env discovery).
    #[arg(long)]
    pub url: Option<String>,

    /// Environment variable holding the connection URL.
    #[arg(long)]
    pub url_env: Option<String>,

    /// Path to mcp-sql-rust.toml for multi-source configuration.
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Allow INSERT/UPDATE/DELETE.
    #[arg(long, conflicts_with = "allow_ddl")]
    pub allow_writes: bool,

    /// Allow DDL (DROP/ALTER/TRUNCATE/CREATE). Implies --allow-writes.
    #[arg(long)]
    pub allow_ddl: bool,

    /// Expose additional schema tools (list_schemas, describe_table, etc.).
    #[arg(long)]
    pub full_tools: bool,

    /// Streamable HTTP bind address (e.g. 127.0.0.1:8080).
    #[arg(long)]
    pub http: Option<String>,

    /// Max rows returned per query.
    #[arg(long, default_value_t = DEFAULT_MAX_ROWS)]
    pub max_rows: u32,

    /// Max response bytes per query result.
    #[arg(long, default_value_t = DEFAULT_MAX_BYTES)]
    pub max_bytes: usize,

    /// Connection pool size per source.
    #[arg(long, default_value_t = DEFAULT_POOL_SIZE)]
    pub pool_size: u32,

    /// Query timeout in seconds.
    #[arg(long, default_value_t = DEFAULT_QUERY_TIMEOUT_SECS)]
    pub query_timeout: u64,

    /// Max concurrent queries in a batch.
    #[arg(long, default_value_t = DEFAULT_BATCH_CONCURRENCY)]
    pub batch_concurrency: usize,

    /// Stop batch on first error.
    #[arg(long)]
    pub fail_fast: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TomlConfig {
    #[serde(default)]
    pub default_source: Option<String>,
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceConfig {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub url_env: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedSource {
    pub name: String,
    pub url: String,
    pub engine: EngineKind,
    pub pool: EnginePool,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub write_mode: WriteMode,
    pub full_tools: bool,
    pub max_rows: u32,
    pub max_bytes: usize,
    pub query_timeout: Duration,
    pub batch_concurrency: usize,
    pub fail_fast: bool,
    pub default_source: String,
    pub sources: HashMap<String, ResolvedSource>,
    pub searched_paths: Vec<PathBuf>,
}

impl AppConfig {
    pub fn source(&self, name: Option<&str>) -> Result<&ResolvedSource> {
        let key = name.unwrap_or(&self.default_source);
        self.sources
            .get(key)
            .with_context(|| format!("unknown source '{key}'"))
    }

    pub async fn connect_all(&self) -> Result<()> {
        for source in self.sources.values() {
            source.pool.ping().await?;
        }
        Ok(())
    }
}

pub async fn load_config(cli: &Cli) -> Result<AppConfig> {
    let searched_paths = discover_dotenv();
    let write_mode = if cli.allow_ddl {
        WriteMode::AllowDdl
    } else if cli.allow_writes {
        WriteMode::AllowWrites
    } else {
        WriteMode::ReadOnly
    };

    let pool_size = cli.pool_size;
    let mut sources = HashMap::new();

    if let Some(path) = &cli.config {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let toml_cfg: TomlConfig =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        for src in toml_cfg.sources {
            let url = resolve_url(src.url.as_deref(), src.url_env.as_deref())?;
            let engine = detect_engine(src.engine.as_deref(), &url)?;
            let pool = connect_pool(&url, engine, pool_size, write_mode).await?;
            sources.insert(
                src.name.clone(),
                ResolvedSource {
                    name: src.name.clone(),
                    url,
                    engine,
                    pool,
                },
            );
        }
        let default_source = toml_cfg
            .default_source
            .or_else(|| sources.keys().next().cloned())
            .context("no sources configured in TOML")?;
        return Ok(AppConfig {
            write_mode,
            full_tools: cli.full_tools,
            max_rows: cli.max_rows,
            max_bytes: cli.max_bytes,
            query_timeout: Duration::from_secs(cli.query_timeout),
            batch_concurrency: cli.batch_concurrency,
            fail_fast: cli.fail_fast,
            default_source,
            sources,
            searched_paths,
        });
    }

    let url = if let Some(url) = &cli.url {
        url.clone()
    } else if let Some(env_key) = &cli.url_env {
        std::env::var(env_key)
            .with_context(|| format!("environment variable '{env_key}' is not set"))?
    } else {
        resolve_url_from_env()?
    };

    let engine = detect_engine(None, &url)?;
    let pool = connect_pool(&url, engine, pool_size, write_mode).await?;
    sources.insert(
        "default".into(),
        ResolvedSource {
            name: "default".into(),
            url,
            engine,
            pool,
        },
    );

    Ok(AppConfig {
        write_mode,
        full_tools: cli.full_tools,
        max_rows: cli.max_rows,
        max_bytes: cli.max_bytes,
        query_timeout: Duration::from_secs(cli.query_timeout),
        batch_concurrency: cli.batch_concurrency,
        fail_fast: cli.fail_fast,
        default_source: "default".into(),
        sources,
        searched_paths,
    })
}

fn discover_dotenv() -> Vec<PathBuf> {
    let mut searched = Vec::new();
    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let env_path = d.join(".env");
        searched.push(env_path.clone());
        if env_path.exists() {
            let _ = dotenvy::from_path(&env_path);
            break;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    searched
}

fn resolve_url_from_env() -> Result<String> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.is_empty() {
            return Ok(url);
        }
    }

    if let Ok(url) = std::env::var("POSTGRES_URL") {
        if !url.is_empty() {
            return Ok(url);
        }
    }

    if let Ok(url) = std::env::var("MYSQL_URL") {
        if !url.is_empty() {
            return Ok(url);
        }
    }

    if let Ok(url) = std::env::var("SQLITE_URL") {
        if !url.is_empty() {
            return Ok(url);
        }
    }

    if let Some(url) = build_mysql_url_from_parts() {
        return Ok(url);
    }

    if let Some(url) = build_postgres_url_from_parts() {
        return Ok(url);
    }

    bail!(
        "no database credentials found. Set DATABASE_URL in .env, use --url, or provide mcp-sql-rust.toml"
    )
}

fn resolve_url(url: Option<&str>, url_env: Option<&str>) -> Result<String> {
    if let Some(raw) = url {
        return Ok(expand_env_placeholders(raw)?);
    }
    if let Some(key) = url_env {
        return std::env::var(key).with_context(|| format!("environment variable '{key}' is not set"));
    }
    resolve_url_from_env()
}

fn expand_env_placeholders(raw: &str) -> Result<String> {
    if raw.starts_with("${") && raw.ends_with('}') {
        let key = &raw[2..raw.len() - 1];
        return std::env::var(key).with_context(|| format!("environment variable '{key}' is not set"));
    }
    Ok(raw.to_string())
}

fn build_postgres_url_from_parts() -> Option<String> {
    let host = std::env::var("POSTGRES_HOST")
        .or_else(|_| std::env::var("PGHOST"))
        .ok()?;
    let user = std::env::var("POSTGRES_USER")
        .or_else(|_| std::env::var("PGUSER"))
        .ok()?;
    let password = std::env::var("POSTGRES_PASSWORD")
        .or_else(|_| std::env::var("PGPASSWORD"))
        .ok()
        .unwrap_or_default();
    let db = std::env::var("POSTGRES_DB")
        .or_else(|_| std::env::var("PGDATABASE"))
        .ok()?;
    let port = std::env::var("POSTGRES_PORT")
        .or_else(|_| std::env::var("PGPORT"))
        .unwrap_or_else(|_| "5432".into());
    Some(format!(
        "postgresql://{user}:{password}@{host}:{port}/{db}"
    ))
}

fn build_mysql_url_from_parts() -> Option<String> {
    let host = std::env::var("MYSQL_HOST").ok()?;
    let user = std::env::var("MYSQL_USER").ok()?;
    let password = std::env::var("MYSQL_PASSWORD").ok().unwrap_or_default();
    let db = std::env::var("MYSQL_DATABASE")
        .or_else(|_| std::env::var("MYSQL_DB"))
        .ok()?;
    let port = std::env::var("MYSQL_PORT").unwrap_or_else(|_| "3306".into());
    Some(format!("mysql://{user}:{password}@{host}:{port}/{db}"))
}

pub fn detect_engine(hint: Option<&str>, url: &str) -> Result<EngineKind> {
    if let Some(h) = hint {
        return match h.to_lowercase().as_str() {
            "postgres" | "postgresql" | "pg" => Ok(EngineKind::Postgres),
            "mysql" | "mariadb" => Ok(EngineKind::Mysql),
            "sqlite" => Ok(EngineKind::Sqlite),
            other => bail!("unknown engine hint '{other}'"),
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

fn sqlite_connect_url(url: &str, write_mode: WriteMode) -> String {
    if write_mode.allows_dml() {
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

async fn connect_pool(
    url: &str,
    engine: EngineKind,
    pool_size: u32,
    write_mode: WriteMode,
) -> Result<EnginePool> {
    match engine {
        EngineKind::Postgres => {
            let mut options = PgPoolOptions::new().max_connections(pool_size);
            if !write_mode.allows_dml() {
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
            let mut options = MySqlPoolOptions::new().max_connections(pool_size);
            if !write_mode.allows_dml() {
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
            let connect_url = sqlite_connect_url(url, write_mode);
            let options = SqlitePoolOptions::new().max_connections(pool_size);
            let pool = options.connect(&connect_url).await?;
            Ok(EnginePool::Sqlite(pool))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_mode_tiers() {
        assert!(!WriteMode::ReadOnly.allows_dml());
        assert!(WriteMode::AllowWrites.allows_dml());
        assert!(!WriteMode::AllowWrites.allows_ddl());
        assert!(WriteMode::AllowDdl.allows_ddl());
    }

    #[test]
    fn detect_engine_from_url() {
        assert_eq!(
            detect_engine(None, "postgresql://localhost/db").unwrap(),
            EngineKind::Postgres
        );
        assert_eq!(
            detect_engine(None, "mysql://localhost/db").unwrap(),
            EngineKind::Mysql
        );
        assert_eq!(
            detect_engine(None, "sqlite::memory:").unwrap(),
            EngineKind::Sqlite
        );
        assert_eq!(
            detect_engine(None, "sqlite://./data.db").unwrap(),
            EngineKind::Sqlite
        );
    }
}
