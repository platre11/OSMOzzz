use anyhow::{Context, Result};
use osmozzz_core::Embedder;
use osmozzz_embedder::Vault;

use crate::config::Config;

pub async fn run(cfg: Config) -> Result<()> {
    std::fs::create_dir_all(&cfg.data_dir)
        .context("Cannot create ~/.osmozzz data directory")?;

    let vault = Vault::open(&cfg.model_path, &cfg.tokenizer_path, cfg.db_path.to_str().unwrap())
        .await
        .context("Failed to initialize vault")?;

    let count_before = vault.count().await.unwrap_or(0);
    println!(
        "Compacting {} documents ({} fragment files → 1)…",
        count_before,
        count_before
    );

    vault.compact().await.context("Compaction failed")?;

    println!("Done. The vector index is now optimized for fast search.");
    println!("Tip: run 'osmozzz search' again — emails and other recently indexed sources should now appear.");
    Ok(())
}
