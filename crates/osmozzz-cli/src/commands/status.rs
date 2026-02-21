use anyhow::Result;
use osmozzz_core::Embedder;
use osmozzz_embedder::Vault;

use crate::config::Config;

pub async fn run(cfg: Config) -> Result<()> {
    println!("OSMOzzz Status");
    println!("{}", "─".repeat(40));
    println!("Data dir:    {}", cfg.data_dir.display());
    println!("DB path:     {}", cfg.db_path.display());
    println!("Model:       {}", cfg.model_path.display());
    println!(
        "Model ready: {}",
        if cfg.model_path.exists() {
            "yes"
        } else {
            "no  (download: see README)"
        }
    );
    println!("Socket:      {}", cfg.socket_path.display());
    println!(
        "Daemon:      {}",
        if cfg.socket_path.exists() {
            "running"
        } else {
            "stopped"
        }
    );

    let model_ready = cfg.model_path.exists() && cfg.tokenizer_path.exists();
    let db_ready = cfg.db_path.exists();

    if model_ready && db_ready {
        match Vault::open(
            &cfg.model_path,
            &cfg.tokenizer_path,
            cfg.db_path.to_str().unwrap_or(".osmozzz/vault"),
        )
        .await
        {
            Ok(vault) => match vault.count().await {
                Ok(count) => {
                    println!("{}", "─".repeat(40));
                    println!("Indexed docs: {}", count);
                }
                Err(e) => {
                    println!("{}", "─".repeat(40));
                    println!("Indexed docs: (error: {})", e);
                }
            },
            Err(e) => {
                println!("{}", "─".repeat(40));
                println!("Vault:        (cannot open: {})", e);
            }
        }
    } else if !model_ready {
        println!("{}", "─".repeat(40));
        println!("Indexed docs: (model not downloaded yet)");
    } else {
        println!("{}", "─".repeat(40));
        println!("Indexed docs: 0 (not yet initialized)");
    }

    Ok(())
}
