mod cli;
mod commands;
mod config;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // En mode MCP et Daemon, logs vers stderr uniquement
    let log_target = matches!(cli.command, Commands::Mcp | Commands::Daemon);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            if cli.verbose { EnvFilter::new("debug") } else { EnvFilter::new("warn") }
        });

    if log_target {
        fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_writer(std::io::stderr)
            .init();
    } else {
        fmt()
            .with_env_filter(filter)
            .with_target(false)
            .init();
    }

    let cfg = config::Config::load()?;

    match cli.command {
        Commands::Index(args) => commands::index::run(args, cfg).await,
        Commands::Search(args) => commands::search::run(args, cfg).await,
        Commands::Serve(args) => commands::serve::run(args, cfg).await,
        Commands::Status => commands::status::run(cfg).await,
        Commands::Mcp => commands::mcp::run(cfg).await,
        Commands::Daemon => commands::daemon::run(cfg).await,
    }
}
