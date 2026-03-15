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
use osmozzz_harvester::{
    start_watcher, AirtableHarvester, GithubHarvester, GitlabHarvester,
    GmailConfig, GmailHarvester, JiraHarvester, LinearHarvester,
    NotionHarvester, ObsidianHarvester, SlackHarvester,
    TerminalHarvester, TodoistHarvester, TrelloHarvester, WatchEvent,
};
#[cfg(target_os = "macos")]
use osmozzz_harvester::{CalendarHarvester, IMessageHarvester, NotesHarvester, SafariHarvester};

use osmozzz_api;
use osmozzz_embedder::Blacklist;
use osmozzz_p2p::{P2pNode, node::{P2pEvent, DEFAULT_P2P_PORT}};

use crate::config::Config;

const DASHBOARD_PORT: u16 = 7878;
const GMAIL_SYNC_INTERVAL_SECS: u64 = 15 * 60;  // 15 minutes
const IMESSAGE_SYNC_INTERVAL_SECS: u64 = 60;    // 1 minute
const SAFARI_SYNC_INTERVAL_SECS: u64 = 60;      // 1 minute
const NOTES_SYNC_INTERVAL_SECS: u64 = 60;       // 1 minute
const TERMINAL_SYNC_INTERVAL_SECS: u64 = 60;    // 1 minute
const CALENDAR_SYNC_INTERVAL_SECS: u64 = 60;    // 1 minute
// Connecteurs cloud (sync moins fréquente — appels API)
const NOTION_SYNC_INTERVAL_SECS: u64 = 60 * 60;    // 1 heure
const GITHUB_SYNC_INTERVAL_SECS: u64 = 60 * 60;    // 1 heure
const LINEAR_SYNC_INTERVAL_SECS: u64 = 60 * 60;    // 1 heure
const JIRA_SYNC_INTERVAL_SECS: u64 = 60 * 60;      // 1 heure
const SLACK_SYNC_INTERVAL_SECS: u64 = 30 * 60;     // 30 minutes
const TRELLO_SYNC_INTERVAL_SECS: u64 = 60 * 60;    // 1 heure
const TODOIST_SYNC_INTERVAL_SECS: u64 = 15 * 60;   // 15 minutes
const GITLAB_SYNC_INTERVAL_SECS: u64 = 60 * 60;    // 1 heure
const AIRTABLE_SYNC_INTERVAL_SECS: u64 = 60 * 60;  // 1 heure
const OBSIDIAN_SYNC_INTERVAL_SECS: u64 = 5 * 60;   // 5 minutes (local)

/// Copie les modèles ONNX dans ~/.osmozzz/models/ si absents.
/// Cherche dans les ancêtres du binaire (workspace dev) ou à côté du binaire.
fn ensure_models(cfg: &Config) {
    let dest_dir = cfg.data_dir.join("models");
    let model_dest = dest_dir.join("all-MiniLM-L6-v2.onnx");
    if model_dest.exists() {
        return; // déjà en place
    }

    // Cherche un dossier models/ contenant le fichier ONNX
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let source_dir = exe_dir
        .ancestors()
        .find(|p| p.join("models").join("all-MiniLM-L6-v2.onnx").exists())
        .map(|p| p.join("models"));

    let Some(src) = source_dir else {
        return; // pas trouvé — l'erreur sera levée par Vault::open
    };

    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        eprintln!("[OSMOzzz] Impossible de créer ~/.osmozzz/models/: {}", e);
        return;
    }

    for file in ["all-MiniLM-L6-v2.onnx", "tokenizer.json"] {
        let from = src.join(file);
        let to = dest_dir.join(file);
        if from.exists() {
            if let Err(e) = std::fs::copy(&from, &to) {
                eprintln!("[OSMOzzz] Copie {} échouée: {}", file, e);
            }
        }
    }
    eprintln!("[OSMOzzz] Modèles installés dans ~/.osmozzz/models/");
}

pub async fn run(cfg: Config) -> Result<()> {
    std::fs::create_dir_all(&cfg.data_dir)
        .context("Impossible de créer ~/.osmozzz")?;

    ensure_models(&cfg);

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

    // Initialiser le nœud P2P
    let (p2p_event_tx, mut p2p_event_rx) = tokio::sync::mpsc::channel::<P2pEvent>(64);
    let display_name = std::env::var("USER").unwrap_or_else(|_| "OSMOzzz".to_string());
    let p2p_node = match P2pNode::new(&display_name, DEFAULT_P2P_PORT, p2p_event_tx).await {
        Ok(node) => {
            let node_clone = node.clone();
            tokio::spawn(async move {
                if let Err(e) = node_clone.start_server().await {
                    eprintln!("[P2P] Erreur serveur : {}", e);
                }
            });
            // Écoute les événements P2P (connexions, requêtes reçues)
            tokio::spawn(async move {
                while let Some(event) = p2p_event_rx.recv().await {
                    match event {
                        P2pEvent::PeerConnected { display_name, .. } => {
                            eprintln!("[P2P] {} connecté", display_name);
                        }
                        P2pEvent::PeerDisconnected { peer_id } => {
                            eprintln!("[P2P] Peer {} déconnecté", &peer_id[..8.min(peer_id.len())]);
                        }
                        P2pEvent::QueryReceived { peer_name, query, .. } => {
                            eprintln!("[P2P] {} a cherché : \"{}\"", peer_name, query);
                        }
                    }
                }
            });
            eprintln!("[OSMOzzz Daemon] P2P démarré sur le port {}", DEFAULT_P2P_PORT);
            Some(node)
        }
        Err(e) => {
            eprintln!("[OSMOzzz Daemon] P2P désactivé : {}", e);
            None
        }
    };

    // Démarrer le serveur HTTP du dashboard
    let dashboard_vault = Arc::clone(&vault);
    let dashboard_p2p = p2p_node.clone();
    tokio::spawn(async move {
        if let Err(e) = osmozzz_api::start_server(dashboard_vault, dashboard_p2p, DASHBOARD_PORT).await {
            eprintln!("[OSMOzzz Daemon] Dashboard erreur: {}", e);
        }
    });

    // Ouvrir le dashboard dans le navigateur par défaut après 500ms
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let url = format!("http://localhost:{}", DASHBOARD_PORT);
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&url).spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd").args(["/c", "start", &url]).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        eprintln!("[OSMOzzz Daemon] Dashboard ouvert sur {}", url);
    });

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
    // Gmail auto-sync — premier tick retardé d'un intervalle pour éviter la double sync
    let gmail_vault = Arc::clone(&vault);
    tokio::spawn(async move {
        let start = tokio::time::Instant::now()
            + tokio::time::Duration::from_secs(GMAIL_SYNC_INTERVAL_SECS);
        let mut interval = tokio::time::interval_at(
            start,
            tokio::time::Duration::from_secs(GMAIL_SYNC_INTERVAL_SECS),
        );
        loop {
            interval.tick().await;
            sync_gmail(&gmail_vault).await;
        }
    });

    // iMessage auto-sync (macOS uniquement)
    #[cfg(target_os = "macos")]
    {
        let v = Arc::clone(&vault);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(IMESSAGE_SYNC_INTERVAL_SECS)
            );
            loop {
                interval.tick().await;
                let docs = IMessageHarvester::new().harvest().await.unwrap_or_default();
                sync_docs(&v, "iMessage", docs).await;
            }
        });
    }

    // Safari auto-sync (macOS uniquement)
    #[cfg(target_os = "macos")]
    {
        let v = Arc::clone(&vault);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(SAFARI_SYNC_INTERVAL_SECS)
            );
            loop {
                interval.tick().await;
                let docs = SafariHarvester::new().harvest().await.unwrap_or_default();
                sync_docs(&v, "Safari", docs).await;
            }
        });
    }

    // Notes auto-sync (macOS uniquement)
    #[cfg(target_os = "macos")]
    {
        let v = Arc::clone(&vault);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(NOTES_SYNC_INTERVAL_SECS)
            );
            loop {
                interval.tick().await;
                let docs = NotesHarvester::new().harvest().await.unwrap_or_default();
                sync_docs(&v, "Notes", docs).await;
            }
        });
    }

    // Terminal auto-sync (tous OS)
    {
        let v = Arc::clone(&vault);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(TERMINAL_SYNC_INTERVAL_SECS)
            );
            loop {
                interval.tick().await;
                let docs = TerminalHarvester::new().harvest().await.unwrap_or_default();
                sync_docs(&v, "Terminal", docs).await;
            }
        });
    }

    // Calendar auto-sync (macOS uniquement)
    #[cfg(target_os = "macos")]
    {
        let v = Arc::clone(&vault);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(CALENDAR_SYNC_INTERVAL_SECS)
            );
            loop {
                interval.tick().await;
                let docs = CalendarHarvester::new().harvest().await.unwrap_or_default();
                sync_docs(&v, "Calendar", docs).await;
            }
        });
    }

    // Notion auto-sync (1h)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(NOTION_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = NotionHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "Notion", docs).await;
        }
    });

    // GitHub auto-sync (1h)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(GITHUB_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = GithubHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "GitHub", docs).await;
        }
    });

    // Linear auto-sync (1h)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(LINEAR_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = LinearHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "Linear", docs).await;
        }
    });

    // Jira auto-sync (1h)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(JIRA_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = JiraHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "Jira", docs).await;
        }
    });

    // Slack auto-sync (30min)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(SLACK_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = SlackHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "Slack", docs).await;
        }
    });

    // Trello auto-sync (1h)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(TRELLO_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = TrelloHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "Trello", docs).await;
        }
    });

    // Todoist auto-sync (15min)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(TODOIST_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = TodoistHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "Todoist", docs).await;
        }
    });

    // GitLab auto-sync (1h)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(GITLAB_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = GitlabHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "GitLab", docs).await;
        }
    });

    // Airtable auto-sync (1h)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(AIRTABLE_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = AirtableHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "Airtable", docs).await;
        }
    });

    // Obsidian auto-sync (5min — vault local)
    let v = Arc::clone(&vault);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(OBSIDIAN_SYNC_INTERVAL_SECS)
        );
        loop {
            interval.tick().await;
            let docs = ObsidianHarvester::new().harvest().await.unwrap_or_default();
            sync_docs(&v, "Obsidian", docs).await;
        }
    });

    eprintln!("[OSMOzzz Daemon] En écoute... (Ctrl+C pour arrêter)");
    eprintln!("[OSMOzzz Daemon] Syncs automatiques :");
    eprintln!("[OSMOzzz Daemon]   Gmail    : toutes les {} min", GMAIL_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   iMessage : toutes les {} min", IMESSAGE_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Safari   : toutes les {} min", SAFARI_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Notes    : toutes les {} min", NOTES_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Terminal : toutes les {} min", TERMINAL_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Calendar : toutes les {} min", CALENDAR_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Notion   : toutes les {} min", NOTION_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   GitHub   : toutes les {} min", GITHUB_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Linear   : toutes les {} min", LINEAR_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Jira     : toutes les {} min", JIRA_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Slack    : toutes les {} min", SLACK_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Trello   : toutes les {} min", TRELLO_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Todoist  : toutes les {} min", TODOIST_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   GitLab   : toutes les {} min", GITLAB_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Airtable : toutes les {} min", AIRTABLE_SYNC_INTERVAL_SECS / 60);
    eprintln!("[OSMOzzz Daemon]   Obsidian : toutes les {} min", OBSIDIAN_SYNC_INTERVAL_SECS / 60);

    // Syncs initiales échelonnées (évite le pic mémoire au démarrage)
    // Les intervalles tokio gèrent les syncs suivantes automatiquement
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

    // Checkpoint = hier : garantit qu'on ne rate jamais les emails arrivés entre deux syncs.
    // La déduplication par checksum (vault.exists) évite les doublons.
    let today = Utc::now().date_naive();
    let checkpoint_date = today.pred_opt().unwrap_or(today);

    if docs.is_empty() {
        eprintln!("[OSMOzzz Daemon] Gmail sync: aucun nouvel email.");
        write_checkpoint(checkpoint_date);
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

    write_checkpoint(checkpoint_date);
}

// ─── Sync générique pour toutes les sources locales ──────────────────────────

async fn sync_docs(vault: &Arc<Vault>, label: &str, docs: Vec<osmozzz_core::Document>) {
    if docs.is_empty() {
        return;
    }

    let blacklist = Blacklist::load();
    let mut indexed = 0;
    for doc in &docs {
        // Skip banned documents (URL ban or source-level ban)
        let source = doc.source.to_string();
        let title  = doc.title.as_deref().unwrap_or("");
        if blacklist.is_banned(&source, &doc.url, title) { continue; }

        if let Ok(true) = vault.exists(&doc.checksum).await { continue; }
        match vault.upsert(doc).await {
            Ok(_) => indexed += 1,
            Err(e) => eprintln!("[OSMOzzz Daemon] {} upsert erreur: {}", label, e),
        }
    }

    if indexed > 0 {
        eprintln!("[OSMOzzz Daemon] {} sync: {} nouveaux documents indexés.", label, indexed);
        if let Err(e) = vault.compact().await {
            eprintln!("[OSMOzzz Daemon] Compact erreur: {}", e);
        }
    }
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
