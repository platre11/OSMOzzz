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
use chrono::{NaiveDate, Utc};
use osmozzz_core::{Embedder, Harvester, OsmozzError};
use osmozzz_embedder::Vault;
use osmozzz_harvester::{start_watcher, GmailConfig, GmailHarvester, WatchEvent};

use crate::config::Config;

const GMAIL_SYNC_INTERVAL_SECS: u64 = 2 * 60; // 2 minutes

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

// ─── Checkpoint ──────────────────────────────────────────────────────────────

fn checkpoint_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".osmozzz/gmail_sync_checkpoint")
}

fn read_checkpoint() -> Option<NaiveDate> {
    let content = std::fs::read_to_string(checkpoint_path()).ok()?;
    NaiveDate::parse_from_str(content.trim(), "%Y-%m-%d").ok()
}

fn write_checkpoint(date: NaiveDate) {
    let _ = std::fs::write(checkpoint_path(), date.format("%Y-%m-%d").to_string());
}

// ─── Sync Gmail ──────────────────────────────────────────────────────────────

async fn sync_gmail(vault: &Arc<Vault>) {
    let config = match GmailConfig::load() {
        Some(c) => c,
        None => return, // Gmail non configuré, on ignore silencieusement
    };

    // Lecture du checkpoint pour l'indexation incrémentale
    let since = read_checkpoint();

    match since {
        Some(d) => eprintln!("[OSMOzzz Daemon] Gmail sync incrémentale depuis {}...", d),
        None    => eprintln!("[OSMOzzz Daemon] Gmail sync initiale (premier démarrage, jusqu'à 5000 emails)..."),
    }

    let mut harvester = GmailHarvester::new(config);
    if let Some(since_date) = since {
        harvester = harvester.with_since(since_date);
    }

    let docs = match harvester.harvest().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[OSMOzzz Daemon] Gmail sync erreur: {}", e);
            return;
        }
    };

    // Met à jour le checkpoint même si aucun email nouveau (évite de re-scanner indéfiniment)
    let today = Utc::now().date_naive();

    if docs.is_empty() {
        eprintln!("[OSMOzzz Daemon] Gmail sync: aucun nouvel email.");
        write_checkpoint(today);
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

    // Checkpoint = aujourd'hui : la prochaine sync ne vérifiera que les emails d'aujourd'hui
    write_checkpoint(today);
}

fn default_watch_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs_next::home_dir() {
        for folder in &["Desktop", "Documents", "Downloads"] {
            let p = home.join(folder);
            if p.exists() {
                paths.push(p);
            }
        }
    }
    paths
}
