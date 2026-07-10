use anyhow::Result;
use sqlx::{MySql, MySqlPool, PgPool, Pool, Postgres};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Postgres,
    Mysql,
}

#[derive(Debug, Clone)]
pub enum EnginePool {
    Postgres(PgPool),
    Mysql(MySqlPool),
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
        }
        Ok(())
    }

    pub fn engine(&self) -> EngineKind {
        match self {
            Self::Postgres(_) => EngineKind::Postgres,
            Self::Mysql(_) => EngineKind::Mysql,
        }
    }

    pub fn postgres(&self) -> Result<&PgPool> {
        match self {
            Self::Postgres(p) => Ok(p),
            Self::Mysql(_) => anyhow::bail!("expected postgres pool"),
        }
    }

    pub fn mysql(&self) -> Result<&MySqlPool> {
        match self {
            Self::Mysql(p) => Ok(p),
            Self::Postgres(_) => anyhow::bail!("expected mysql pool"),
        }
    }
}

pub type PgPoolType = Pool<Postgres>;
pub type MyPoolType = Pool<MySql>;
