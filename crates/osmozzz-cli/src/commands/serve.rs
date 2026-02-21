use std::sync::Arc;

use anyhow::{Context, Result};
use osmozzz_bridge::BridgeServer;
use osmozzz_embedder::Vault;

use crate::cli::ServeArgs;
use crate::config::Config;

pub async fn run(args: ServeArgs, cfg: Config) -> Result<()> {
    // Ensure data dir exists
    std::fs::create_dir_all(&cfg.data_dir)
        .context("Cannot create ~/.osmozzz data directory")?;

    let vault = Vault::open(&cfg.model_path, &cfg.tokenizer_path, cfg.db_path.to_str().unwrap())
        .await
        .context("Failed to initialize vault")?;

    let vault = Arc::new(vault);

    let socket_path = args
        .socket
        .map(|s| std::path::PathBuf::from(shellexpand::tilde(&s).as_ref()))
        .unwrap_or(cfg.socket_path);

    println!(
        "Starting OSMOzzz bridge at: {}",
        socket_path.display()
    );
    println!("Press Ctrl+C to stop.");

    let server = BridgeServer::new(socket_path, vault);
    server
        .serve()
        .await
        .context("Bridge server error")?;

    Ok(())
}
