/// Service watcher autonome — indépendant du MCP et de l'IA.
///
/// Lance la surveillance de ~/Desktop et ~/Documents en arrière-plan.
/// Indexe silencieusement chaque fichier créé ou modifié dans LanceDB.
/// Le serveur MCP (et toute autre app future) lit la même base sans conflit.
///
/// Lancement : osmozzz daemon
/// Arrêt propre : Ctrl+C
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use osmozzz_core::{Embedder, OsmozzError};
use osmozzz_embedder::Vault;
use osmozzz_harvester::{start_watcher, WatchEvent};

use crate::config::Config;

pub async fn run(cfg: Config) -> Result<()> {
    std::fs::create_dir_all(&cfg.data_dir)
        .context("Impossible de créer ~/.osmozzz")?;

    eprintln!("[OSMOzzz Daemon] Initialisation du vault...");

    let vault = Arc::new(
        Vault::open(
            &cfg.model_path,
            &cfg.tokenizer_path,
            cfg.db_path.to_str().unwrap_or(".osmozzz/vault"),
        )
        .await
        .context("Impossible d'ouvrir le vault")?,
    );

    eprintln!("[OSMOzzz Daemon] Vault prêt.");

    let watch_paths = default_watch_paths();
    if watch_paths.is_empty() {
        eprintln!("[OSMOzzz Daemon] Aucun dossier à surveiller (Desktop/Documents introuvables).");
        return Ok(());
    }

    eprintln!(
        "[OSMOzzz Daemon] Surveillance de {} dossier(s) :",
        watch_paths.len()
    );
    for p in &watch_paths {
        eprintln!("[OSMOzzz Daemon]   → {}", p.display());
    }
    eprintln!("[OSMOzzz Daemon] En écoute... (Ctrl+C pour arrêter)");

    let mut rx = start_watcher(watch_paths);

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Some(WatchEvent::Upsert(docs)) => {
                        for doc in &docs {
                            match vault.upsert(doc).await {
                                Ok(()) => {
                                    eprintln!(
                                        "[OSMOzzz Daemon] Indexé : {}",
                                        doc.url
                                    );
                                }
                                Err(OsmozzError::Storage(e)) if e.contains("duplicate") => {
                                    // Déjà présent en base, pas grave
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[OSMOzzz Daemon] Erreur upsert {}: {}",
                                        doc.url, e
                                    );
                                }
                            }
                        }
                    }
                    None => {
                        // Canal watcher fermé → arrêt
                        eprintln!("[OSMOzzz Daemon] Watcher arrêté.");
                        break;
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                eprintln!("\n[OSMOzzz Daemon] Arrêt propre (Ctrl+C).");
                break;
            }
        }
    }

    Ok(())
}

fn default_watch_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs_next::home_dir() {
        let desktop = home.join("Desktop");
        let documents = home.join("Documents");
        if desktop.exists() {
            paths.push(desktop);
        }
        if documents.exists() {
            paths.push(documents);
        }
    }
    paths
}
