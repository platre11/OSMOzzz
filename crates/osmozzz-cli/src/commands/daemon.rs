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
use osmozzz_core::{Embedder, Harvester, OsmozzError};
use osmozzz_embedder::Vault;
use osmozzz_harvester::{start_watcher, GmailConfig, GmailHarvester, WatchEvent};

use crate::config::Config;

const GMAIL_SYNC_INTERVAL_SECS: u64 = 15 * 60; // 15 minutes

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
    // Gmail auto-sync : lance une première sync immédiate, puis toutes les 15 min
    let gmail_vault = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(GMAIL_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            sync_gmail(&gmail_vault).await;
        }
    });

    eprintln!("[OSMOzzz Daemon] En écoute... (Ctrl+C pour arrêter)");
    eprintln!("[OSMOzzz Daemon] Gmail sync automatique toutes les {} min.", GMAIL_SYNC_INTERVAL_SECS / 60);

    // Première sync Gmail immédiate au démarrage
    sync_gmail(&vault).await;

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

async fn sync_gmail(vault: &Arc<Vault>) {
    let config = match GmailConfig::load() {
        Some(c) => c,
        None => return, // Gmail non configuré, on ignore silencieusement
    };

    eprintln!("[OSMOzzz Daemon] Gmail sync démarrage...");

    let harvester = GmailHarvester::new(config);
    let docs = match harvester.harvest().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[OSMOzzz Daemon] Gmail sync erreur: {}", e);
            return;
        }
    };

    if docs.is_empty() {
        eprintln!("[OSMOzzz Daemon] Gmail sync: aucun nouvel email.");
        return;
    }

    let mut indexed = 0;
    for doc in &docs {
        if let Ok(true) = vault.exists(&doc.checksum).await { continue; }
        match vault.upsert(doc).await {
            Ok(_) => indexed += 1,
            Err(e) => eprintln!("[OSMOzzz Daemon] Gmail upsert erreur: {}", e),
        }
    }

    if indexed > 0 {
        eprintln!("[OSMOzzz Daemon] Gmail sync: {} nouveaux emails indexés.", indexed);
        // Compact après chaque sync pour maintenir les performances
        if let Err(e) = vault.compact().await {
            eprintln!("[OSMOzzz Daemon] Compact erreur: {}", e);
        }
    }
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
