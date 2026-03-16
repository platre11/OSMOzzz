use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use osmozzz_core::action::{ActionRequest, ActionStatus};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Événement envoyé via SSE au dashboard lors de tout changement d'état.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEvent {
    /// "new" = nouvelle action | "updated" = statut modifié
    pub kind: String,
    pub action: ActionRequest,
}

/// File d'attente thread-safe des actions en cours.
///
/// Partagée via `Arc` dans l'AppState du serveur HTTP.
/// Le process MCP soumet des actions via `POST /api/actions`.
/// Le dashboard s'abonne via `GET /api/actions/stream` (SSE).
#[derive(Clone)]
pub struct ActionQueue {
    inner: Arc<Mutex<VecDeque<ActionRequest>>>,
    tx: broadcast::Sender<ActionEvent>,
}

impl ActionQueue {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(128);
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
            tx,
        }
    }

    /// Ajoute une action et notifie les abonnés SSE.
    pub fn push(&self, action: ActionRequest) {
        let mut q = self.inner.lock().unwrap();
        if q.len() >= 500 {
            q.pop_front(); // Éviter la croissance illimitée
        }
        let event = ActionEvent { kind: "new".to_string(), action: action.clone() };
        q.push_back(action);
        let _ = self.tx.send(event);
    }

    /// Retourne les actions en attente, en marquant les expirées au passage.
    pub fn pending(&self) -> Vec<ActionRequest> {
        let mut q = self.inner.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        for a in q.iter_mut() {
            if matches!(a.status, ActionStatus::Pending) && now > a.expires_at {
                a.status = ActionStatus::Expired;
                let _ = self.tx.send(ActionEvent {
                    kind: "updated".to_string(),
                    action: a.clone(),
                });
            }
        }
        q.iter()
            .filter(|a| matches!(a.status, ActionStatus::Pending))
            .cloned()
            .collect()
    }

    /// Retourne tout l'historique (les plus récents en premier).
    pub fn all(&self) -> Vec<ActionRequest> {
        self.inner.lock().unwrap().iter().rev().cloned().collect()
    }

    /// Nombre d'actions en attente (pour le badge nav).
    pub fn pending_count(&self) -> usize {
        self.pending().len()
    }

    /// Approuve une action par son ID.
    pub fn approve(&self, id: &str) -> Option<ActionRequest> {
        self.update_status(id, ActionStatus::Approved)
    }

    /// Rejette une action par son ID.
    pub fn reject(&self, id: &str) -> Option<ActionRequest> {
        self.update_status(id, ActionStatus::Rejected)
    }

    /// Stocke le résultat d'exécution après approbation.
    pub fn set_execution_result(&self, id: &str, result: String) -> Option<ActionRequest> {
        let mut q = self.inner.lock().unwrap();
        if let Some(action) = q.iter_mut().find(|a| a.id == id) {
            action.execution_result = Some(result);
            let updated = action.clone();
            let _ = self.tx.send(ActionEvent {
                kind: "updated".to_string(),
                action: updated.clone(),
            });
            return Some(updated);
        }
        None
    }

    /// Abonnement au flux d'événements SSE.
    pub fn subscribe(&self) -> broadcast::Receiver<ActionEvent> {
        self.tx.subscribe()
    }

    fn update_status(&self, id: &str, status: ActionStatus) -> Option<ActionRequest> {
        let mut q = self.inner.lock().unwrap();
        if let Some(action) = q.iter_mut().find(|a| a.id == id) {
            action.status = status;
            let updated = action.clone();
            let _ = self.tx.send(ActionEvent {
                kind: "updated".to_string(),
                action: updated.clone(),
            });
            return Some(updated);
        }
        None
    }
}

impl Default for ActionQueue {
    fn default() -> Self {
        Self::new()
    }
}
