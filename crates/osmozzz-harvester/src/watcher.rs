/// Surveillance de fichiers en temps réel via FSEvents (macOS).
/// Utilise la crate `notify` v6 qui s'appuie sur l'API native du système.
///
/// Correctifs appliqués :
/// - Canal BORNÉ (2048) + try_send → plus d'explosion mémoire au démarrage
/// - Filtre par extension avant tout traitement (PDF + texte connu uniquement)
/// - Nettoyage périodique de last_seen → plus de fuite mémoire
/// - `.osmozzz` exclu explicitement des chemins surveillés
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use osmozzz_core::Document;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::files::harvest_directory;
use crate::files::harvest_file_metadata;
use crate::files::SKIP_DIRS;

const DEBOUNCE_MS: u64 = 500;
/// Pause entre traitements pour ne pas saturer le CPU
const CPU_YIELD_MS: u64 = 10;
/// Grace period au démarrage : absorbe le flood FSEvents historique.
const STARTUP_GRACE_SECS: u64 = 15;

/// Capacité max du canal brut FSEvents.
/// FSEvents envoie des milliers d'événements historiques au démarrage (catchup).
/// Avec try_send, les événements au-delà de cette limite sont silencieusement
/// ignorés → backpressure sans blocage ni explosion mémoire.
const RAW_CHANNEL_CAPACITY: usize = 2048;

/// Intervalle de nettoyage de la map last_seen (évite la fuite mémoire).
const PRUNE_INTERVAL_SECS: u64 = 60;
/// Durée max d'une entrée dans last_seen avant d'être purgée.
const PRUNE_MAX_AGE_SECS: u64 = 300;

/// Événement émis par le watcher vers le consommateur (la commande mcp).
pub enum WatchEvent {
    /// Fichier créé ou modifié → liste de Documents à indexer
    Upsert(Vec<Document>),
}

/// Lance la surveillance des chemins donnés.
/// Retourne un canal de réception d'événements.
/// La tâche tourne indéfiniment en arrière-plan via tokio::spawn.
pub fn start(watch_paths: Vec<PathBuf>) -> mpsc::Receiver<WatchEvent> {
    let (event_tx, event_rx) = mpsc::channel::<WatchEvent>(256);

    // Canal BORNÉ entre le callback synchrone de notify et la tâche async tokio.
    // try_send dans le callback : si plein → événement ignoré (backpressure),
    // jamais de blocage ni de croissance mémoire illimitée.
    let (raw_tx, mut raw_rx) = mpsc::channel::<notify::Result<Event>>(RAW_CHANNEL_CAPACITY);

    tokio::spawn(async move {
        let raw_tx_clone = raw_tx.clone();
        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                // try_send : si le canal est plein, l'événement est jeté.
                // Pas de blocage, pas d'accumulation mémoire illimitée.
                if raw_tx_clone.try_send(res).is_err() {
                    debug!("[Watcher] Canal saturé — événement ignoré (backpressure)");
                }
            },
            notify::Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                warn!("[Watcher] Impossible de créer le watcher: {}", e);
                return;
            }
        };

        for path in &watch_paths {
            if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
                warn!("[Watcher] Impossible de surveiller {}: {}", path.display(), e);
            } else {
                info!("[Watcher] Surveillance active sur: {}", path.display());
            }
        }

        // Debounce map : path → dernière modification vue
        let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();
        let debounce = Duration::from_millis(DEBOUNCE_MS);
        let yield_dur = Duration::from_millis(CPU_YIELD_MS);
        let prune_interval = Duration::from_secs(PRUNE_INTERVAL_SECS);
        let prune_max_age = Duration::from_secs(PRUNE_MAX_AGE_SECS);
        let mut last_prune = Instant::now();
        let startup_grace_end = Instant::now() + Duration::from_secs(STARTUP_GRACE_SECS);
        let mut grace_logged = false;

        loop {
            // Attend le prochain événement (non bloquant côté tokio runtime)
            let result = match raw_rx.recv().await {
                Some(r) => r,
                None => {
                    // Canal fermé → on s'arrête
                    break;
                }
            };

            let event = match result {
                Ok(e) => e,
                Err(e) => {
                    warn!("[Watcher] Erreur d'événement: {}", e);
                    continue;
                }
            };

            // Grace period : draine le flood FSEvents historique sans traiter.
            if Instant::now() < startup_grace_end {
                continue;
            }
            if !grace_logged {
                eprintln!("[OSMOzzz Daemon] Prêt — surveillance active (créations, copies, déplacements).");
                grace_logged = true;
            }

            // On traite créations, modifications ET renommages/déplacements.
            // Sur macOS/Finder :
            //   dupliquer un fichier  → Create(_)
            //   déplacer un fichier   → Modify(ModifyKind::Name(_))
            //   modifier un fichier   → Modify(_)
            let is_relevant = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_)
            );
            if !is_relevant {
                continue;
            }

            let now = Instant::now();

            // Nettoyage périodique de last_seen.
            // Sans ça, la map grandit indéfiniment pour chaque fichier vu.
            if now.duration_since(last_prune) >= prune_interval {
                last_seen.retain(|_, t| now.duration_since(*t) < prune_max_age);
                last_prune = now;
                debug!(
                    "[Watcher] last_seen nettoyé — {} entrées restantes",
                    last_seen.len()
                );
            }

            for path in event.paths {
                if path_is_noisy(&path) {
                    continue;
                }
                if is_hidden(&path) {
                    continue;
                }

                // Debounce
                if let Some(last) = last_seen.get(&path) {
                    if now.duration_since(*last) < debounce {
                        debug!("[Watcher] Debounce skip: {}", path.display());
                        continue;
                    }
                }
                last_seen.insert(path.clone(), now);

                let docs = if path.is_dir() {
                    // Dossier créé → indexer son nom/chemin comme métadonnée
                    harvest_directory(&path)
                } else if path.is_file() {
                    // Watcher : métadonnées uniquement, jamais de lecture de contenu.
                    // Pour indexer le contenu complet → osmozzz index <chemin>
                    harvest_file_metadata(&path)
                } else {
                    continue;
                };

                if !docs.is_empty() {
                    info!(
                        "[Watcher] {} doc(s) depuis: {}",
                        docs.len(),
                        path.display()
                    );
                    if event_tx.send(WatchEvent::Upsert(docs)).await.is_err() {
                        // Le récepteur a été fermé → on s'arrête
                        return;
                    }
                }

                // Petite pause pour ne pas saturer le CPU
                tokio::time::sleep(yield_dur).await;
            }
        }

        eprintln!("[OSMOzzz Watcher] Arrêté.");
        // Garde le watcher en vie jusqu'à la fin de la tâche
        drop(watcher);
    });

    event_rx
}

fn path_is_noisy(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| SKIP_DIRS.contains(&s))
            .unwrap_or(false)
    })
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}
