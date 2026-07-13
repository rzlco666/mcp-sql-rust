use anyhow::Result;
use clap::Parser;
use mcp_sql_rust::config::{load_config, Cli};
use mcp_sql_rust::server::{run_http, run_stdio};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    let config = load_config(&cli).await?;
    if cli.eager_connect {
        config.connect_all().await?;
    }

    if let Some(addr) = &cli.http {
        run_http(config, addr).await
    } else {
        run_stdio(config).await
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_env("MCP_SQL_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
}
