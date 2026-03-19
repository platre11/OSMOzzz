use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Statut d'une action en cours de traitement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// En attente de validation par l'utilisateur dans le dashboard.
    Pending,
    /// Approuvée — prête à être exécutée (Phase 2).
    Approved,
    /// Refusée par l'utilisateur.
    Rejected,
    /// Expirée (aucune réponse dans les 5 minutes).
    Expired,
}

/// Une action demandée par Claude, en attente de validation humaine.
/// Claude appelle un tool `act_*` → OSMOzzz crée un ActionRequest
/// → l'utilisateur valide dans le dashboard → exécution (Phase 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Identifiant unique UUID v4.
    pub id: String,
    /// Nom du tool MCP appelé (ex: "act_send_email").
    pub tool: String,
    /// Paramètres bruts JSON tels que Claude les a fournis.
    pub params: serde_json::Value,
    /// Résumé en langage naturel de ce que l'action va faire.
    /// Affiché dans la modale de validation — jamais de JSON brut ici.
    pub preview: String,
    /// Statut courant de l'action.
    pub status: ActionStatus,
    /// Timestamp Unix (secondes) de création.
    pub created_at: i64,
    /// Timestamp Unix (secondes) d'expiration (created_at + 300s).
    pub expires_at: i64,
    /// Résultat de l'exécution après approbation.
    /// None = pas encore exécuté, Some("ok: ...") = succès, Some("err: ...") = erreur.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_result: Option<String>,
    /// Si true, l'exécution est gérée par le process MCP (proxy subprocess).
    /// Le daemon NE doit PAS appeler executor::execute() — juste changer le statut.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_proxy: Option<bool>,
}

impl ActionRequest {
    pub fn new(
        tool: impl Into<String>,
        params: serde_json::Value,
        preview: impl Into<String>,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            tool: tool.into(),
            params,
            preview: preview.into(),
            status: ActionStatus::Pending,
            created_at: now,
            expires_at: now + 300,
            execution_result: None,
            mcp_proxy: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expires_at
    }
}
