mod cli;
mod commands;
mod config;
mod proof;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // En mode MCP et Daemon, logs vers stderr uniquement
    let log_target = matches!(cli.command, Commands::Mcp | Commands::Daemon | Commands::Install | Commands::Verify(_));

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            if cli.verbose {
                EnvFilter::new("debug")
            } else {
                // Silencer les libs P2P (iroh, quinn, etc.) qui spamment en WARN
                EnvFilter::new("warn,iroh=error,iroh_net=error,iroh_relay=error,quinn=error,quinn_proto=error,iroh_gossip=error")
            }
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
        Commands::Compact => commands::compact::run(cfg).await,
        Commands::Install => { commands::install::run()?; Ok(()) }
        Commands::Verify(args) => {
            commands::verify::run(&args.sig, &args.source, &args.url, &args.content, args.ts)
        }
    }
}
