/// Connecteur n8n — REST API v1.
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct N8nConfig {
    api_url: String,
    api_key: String,
}

impl N8nConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/n8n.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!(
            "{}/api/v1/{}",
            self.api_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &N8nConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("X-N8N-API-KEY", &cfg.api_key)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &N8nConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("X-N8N-API-KEY", &cfg.api_key)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn patch_json(cfg: &N8nConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .patch(url)
        .header("X-N8N-API-KEY", &cfg.api_key)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn delete(cfg: &N8nConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .delete(url)
        .header("X-N8N-API-KEY", &cfg.api_key)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn put_json(cfg: &N8nConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .put(url)
        .header("X-N8N-API-KEY", &cfg.api_key)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

// ─── Formatters ───────────────────────────────────────────────────────────────

fn format_workflow(w: &Value) -> String {
    let id      = w["id"].as_str().or_else(|| w["id"].as_i64().map(|_| "")).unwrap_or("—");
    let id      = if id.is_empty() {
        w["id"].to_string()
    } else {
        id.to_string()
    };
    let name    = w["name"].as_str().unwrap_or("—");
    let active  = w["active"].as_bool().unwrap_or(false);
    let updated = w["updatedAt"].as_str().unwrap_or("—");

    let tags: Vec<&str> = w["tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t["name"].as_str())
                .collect()
        })
        .unwrap_or_default();

    let active_str = if active { "actif" } else { "inactif" };
    let tags_str   = if tags.is_empty() {
        "—".to_string()
    } else {
        tags.join(", ")
    };

    format!(
        "• [{id}] {name}\n  Statut : {active_str} | Modifié : {updated}\n  Tags   : {tags_str}"
    )
}

fn format_credential(c: &Value) -> String {
    let id         = &c["id"];
    let name       = c["name"].as_str().unwrap_or("—");
    let cred_type  = c["type"].as_str().unwrap_or("—");
    let created    = c["createdAt"].as_str().unwrap_or("—");
    format!("• [{id}] {name}\n  Type : {cred_type} | Créé : {created}")
}

fn format_variable(v: &Value) -> String {
    let id    = &v["id"];
    let key   = v["key"].as_str().unwrap_or("—");
    let value = v["value"].as_str().unwrap_or("—");
    format!("• [{id}] {key} = {value}")
}

fn format_execution(e: &Value) -> String {
    let id          = e["id"].as_str().or_else(|| e["id"].as_i64().map(|_| "")).unwrap_or("—");
    let id          = if id.is_empty() { e["id"].to_string() } else { id.to_string() };
    let workflow_id = e["workflowId"].as_str().unwrap_or("—");
    let status      = e["status"].as_str().unwrap_or("—");
    let started     = e["startedAt"].as_str().unwrap_or("—");
    let stopped     = e["stoppedAt"].as_str().unwrap_or("—");
    let mode        = e["mode"].as_str().unwrap_or("—");

    format!(
        "• [{id}] workflow:{workflow_id}\n  Statut : {status} | Mode : {mode}\n  Début  : {started} | Fin : {stopped}"
    )
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "n8n_list_workflows",
            "description": "N8N ⚙️ — Liste tous les workflows avec leur id, nom, statut actif/inactif et tags. Retourne jusqu'à 50 workflows. Utiliser n8n_get_workflow pour le détail complet.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "n8n_get_workflow",
            "description": "N8N ⚙️ — Récupère le détail complet d'un workflow : nœuds, connexions, paramètres, déclencheurs. Utiliser n8n_list_workflows pour obtenir l'id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID du workflow" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_activate_workflow",
            "description": "N8N ⚙️ — Active un workflow (le met en production). Une fois actif, il se déclenche automatiquement selon son déclencheur (webhook, cron, etc.).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID du workflow à activer" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_deactivate_workflow",
            "description": "N8N ⚙️ — Désactive un workflow (le met en pause). Il ne se déclenchera plus automatiquement jusqu'à sa réactivation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID du workflow à désactiver" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_execute_workflow",
            "description": "N8N ⚙️ — Déclenche l'exécution manuelle d'un workflow. Retourne l'ID d'exécution pour suivre son statut avec n8n_get_execution.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID du workflow à exécuter" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_delete_workflow",
            "description": "N8N ⚙️ — Supprime définitivement un workflow. Action irréversible. Utiliser n8n_deactivate_workflow pour désactiver temporairement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID du workflow à supprimer" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_list_executions",
            "description": "N8N ⚙️ — Liste les exécutions d'un workflow avec statut (success/error/running), dates de début et fin. Retourne les 20 dernières exécutions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workflow_id": { "type": "string", "description": "ID du workflow dont on veut les exécutions" }
                },
                "required": ["workflow_id"]
            }
        }),
        json!({
            "name": "n8n_get_execution",
            "description": "N8N ⚙️ — Récupère le détail complet d'une exécution : données d'entrée/sortie de chaque nœud, erreurs éventuelles, durée. Utiliser n8n_list_executions pour obtenir l'id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID de l'exécution" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_stop_execution",
            "description": "N8N ⚙️ — Arrête une exécution en cours. Utile si un workflow est bloqué ou tourne trop longtemps.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID de l'exécution à arrêter" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_list_tags",
            "description": "N8N ⚙️ — Liste tous les tags disponibles dans l'instance n8n, avec leur id et nom. Utile pour filtrer les workflows par catégorie.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "n8n_get_workflow_runs",
            "description": "N8N ⚙️ — Récupère les 10 dernières exécutions en erreur d'un workflow spécifique. Pratique pour diagnostiquer rapidement les problèmes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workflow_id": { "type": "string", "description": "ID du workflow à inspecter" }
                },
                "required": ["workflow_id"]
            }
        }),
        json!({
            "name": "n8n_trigger_webhook",
            "description": "N8N ⚙️ — Déclenche un workflow via son URL de webhook directement. Supporte GET et POST avec un corps JSON optionnel. Utiliser pour les workflows déclenchés par webhook.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "webhook_url": { "type": "string", "description": "URL complète du webhook n8n (ex: http://localhost:5678/webhook/abc123)" },
                    "method":      {
                        "type": "string",
                        "enum": ["GET", "POST"],
                        "default": "POST",
                        "description": "Méthode HTTP à utiliser (défaut: POST)"
                    },
                    "body": {
                        "type": "object",
                        "description": "Corps JSON à envoyer (optionnel, uniquement pour POST)"
                    }
                },
                "required": ["webhook_url"]
            }
        }),
        json!({
            "name": "n8n_create_workflow",
            "description": "N8N ⚙️ — Crée un nouveau workflow vide ou avec des nœuds. Retourne l'ID du workflow créé. Utiliser n8n_update_workflow pour modifier ensuite.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":   { "type": "string", "description": "Nom du workflow" },
                    "nodes":  { "type": "array",  "description": "Tableau JSON de nœuds n8n (optionnel, défaut: [])" },
                    "active": { "type": "boolean","description": "Activer immédiatement le workflow (défaut: false)" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "n8n_update_workflow",
            "description": "N8N ⚙️ — Met à jour un workflow existant (nom, nœuds, statut actif). Récupère d'abord le workflow via n8n_get_workflow puis fusionne les modifications avant PUT.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id":     { "type": "string",  "description": "ID du workflow à modifier" },
                    "name":   { "type": "string",  "description": "Nouveau nom (optionnel)" },
                    "nodes":  { "type": "array",   "description": "Nouveau tableau de nœuds JSON (optionnel)" },
                    "active": { "type": "boolean", "description": "Changer le statut actif/inactif (optionnel)" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_list_credentials",
            "description": "N8N ⚙️ — Liste tous les credentials configurés (noms et types uniquement, jamais les valeurs). Utile pour savoir quels credentials sont disponibles.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "n8n_create_credential",
            "description": "N8N ⚙️ — Crée un nouveau credential dans n8n. Le type doit correspondre au type n8n (ex: 'githubApi', 'slackApi'). Le champ data contient les paires clé/valeur du credential.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Nom du credential" },
                    "type": { "type": "string", "description": "Type n8n du credential (ex: githubApi, slackApi, httpBasicAuth)" },
                    "data": { "type": "object", "description": "Objet JSON avec les champs du credential (ex: {\"accessToken\": \"xxx\"})" }
                },
                "required": ["name", "type", "data"]
            }
        }),
        json!({
            "name": "n8n_delete_credential",
            "description": "N8N ⚙️ — Supprime définitivement un credential. Action irréversible. Utiliser n8n_list_credentials pour obtenir l'id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID du credential à supprimer" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_list_variables",
            "description": "N8N ⚙️ — Liste toutes les variables d'environnement configurées dans l'instance n8n avec leur clé et valeur.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "n8n_create_variable",
            "description": "N8N ⚙️ — Crée une nouvelle variable d'environnement dans n8n. Les variables sont accessibles dans les workflows via $vars.KEY.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key":   { "type": "string", "description": "Clé de la variable (ex: API_BASE_URL)" },
                    "value": { "type": "string", "description": "Valeur de la variable" }
                },
                "required": ["key", "value"]
            }
        }),
        json!({
            "name": "n8n_delete_variable",
            "description": "N8N ⚙️ — Supprime une variable d'environnement. Utiliser n8n_list_variables pour obtenir l'id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID de la variable à supprimer" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "n8n_create_tag",
            "description": "N8N ⚙️ — Crée un nouveau tag pour organiser les workflows. Retourne l'ID du tag créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Nom du tag à créer" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "n8n_delete_tag",
            "description": "N8N ⚙️ — Supprime un tag. Les workflows qui utilisaient ce tag ne sont pas supprimés. Utiliser n8n_list_tags pour obtenir l'id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID du tag à supprimer" }
                },
                "required": ["id"]
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = N8nConfig::load()
        .ok_or_else(|| "n8n non configuré — créer ~/.osmozzz/n8n.toml avec api_url et api_key".to_string())?;

    match name {
        "n8n_list_workflows" => {
            let url  = cfg.api("/workflows?limit=50");
            let resp = get(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let workflows = resp["data"].as_array().cloned().unwrap_or_default();

            if workflows.is_empty() {
                return Ok("Aucun workflow trouvé.".to_string());
            }

            let mut out = format!("{} workflow(s) :\n\n", workflows.len());
            for w in &workflows {
                out.push_str(&format_workflow(w));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "n8n_get_workflow" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/workflows/{id}"));
            let resp = get(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let name    = resp["name"].as_str().unwrap_or("—");
            let active  = resp["active"].as_bool().unwrap_or(false);
            let created = resp["createdAt"].as_str().unwrap_or("—");
            let updated = resp["updatedAt"].as_str().unwrap_or("—");

            let tags: Vec<&str> = resp["tags"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|t| t["name"].as_str()).collect())
                .unwrap_or_default();
            let tags_str = if tags.is_empty() { "—".to_string() } else { tags.join(", ") };

            let node_count = resp["nodes"].as_array().map(|a| a.len()).unwrap_or(0);
            let active_str = if active { "actif" } else { "inactif" };

            Ok(format!(
                "Workflow [{id}] {name}\nStatut  : {active_str}\nNœuds   : {node_count}\nTags    : {tags_str}\nCréé    : {created}\nModifié : {updated}"
            ))
        }

        "n8n_activate_workflow" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/workflows/{id}"));
            let body = json!({ "active": true });
            let resp = patch_json(&cfg, &url, &body).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let name = resp["name"].as_str().unwrap_or(id);
            Ok(format!("Workflow '{name}' ({id}) activé avec succès."))
        }

        "n8n_deactivate_workflow" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/workflows/{id}"));
            let body = json!({ "active": false });
            let resp = patch_json(&cfg, &url, &body).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let name = resp["name"].as_str().unwrap_or(id);
            Ok(format!("Workflow '{name}' ({id}) désactivé avec succès."))
        }

        "n8n_execute_workflow" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/workflows/{id}/run"));
            let body = json!({});
            let resp = post_json(&cfg, &url, &body).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let exec_id = resp["executionId"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| resp["executionId"].as_i64().map(|n| n.to_string()))
                .unwrap_or_else(|| "—".to_string());

            Ok(format!(
                "Workflow {id} déclenché. Exécution ID : {exec_id}\nUtiliser n8n_get_execution avec cet ID pour suivre l'avancement."
            ))
        }

        "n8n_delete_workflow" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/workflows/{id}"));
            let resp = delete(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            Ok(format!("Workflow {id} supprimé définitivement."))
        }

        "n8n_list_executions" => {
            let workflow_id = args["workflow_id"].as_str().ok_or("Paramètre 'workflow_id' requis")?;
            let url         = cfg.api(&format!("/executions?workflowId={workflow_id}&limit=20"));
            let resp        = get(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let executions = resp["data"].as_array().cloned().unwrap_or_default();

            if executions.is_empty() {
                return Ok(format!("Aucune exécution trouvée pour le workflow {workflow_id}."));
            }

            let mut out = format!("{} exécution(s) pour workflow {workflow_id} :\n\n", executions.len());
            for e in &executions {
                out.push_str(&format_execution(e));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "n8n_get_execution" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/executions/{id}"));
            let resp = get(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let workflow_id = resp["workflowId"].as_str().unwrap_or("—");
            let status      = resp["status"].as_str().unwrap_or("—");
            let started     = resp["startedAt"].as_str().unwrap_or("—");
            let stopped     = resp["stoppedAt"].as_str().unwrap_or("—");
            let mode        = resp["mode"].as_str().unwrap_or("—");

            let mut out = format!(
                "Exécution [{id}]\nWorkflow  : {workflow_id}\nStatut    : {status}\nMode      : {mode}\nDébut     : {started}\nFin       : {stopped}\n"
            );

            // Afficher l'erreur si présente
            if let Some(err) = resp["data"]["resultData"]["error"].as_object() {
                let err_msg  = err.get("message").and_then(|v| v.as_str()).unwrap_or("—");
                let err_node = err.get("node").and_then(|v| v.as_str()).unwrap_or("—");
                out.push_str(&format!("\nErreur sur nœud '{err_node}' : {err_msg}\n"));
            }

            Ok(out.trim_end().to_string())
        }

        "n8n_stop_execution" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/executions/{id}/stop"));
            let resp = post_json(&cfg, &url, &json!({})).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            Ok(format!("Exécution {id} arrêtée."))
        }

        "n8n_list_tags" => {
            let url  = cfg.api("/tags");
            let resp = get(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let tags = resp["data"].as_array().cloned().unwrap_or_default();

            if tags.is_empty() {
                return Ok("Aucun tag trouvé.".to_string());
            }

            let mut out = format!("{} tag(s) :\n\n", tags.len());
            for t in &tags {
                let tag_id   = &t["id"];
                let tag_name = t["name"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{tag_id}] {tag_name}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "n8n_get_workflow_runs" => {
            let workflow_id = args["workflow_id"].as_str().ok_or("Paramètre 'workflow_id' requis")?;
            let url         = cfg.api(&format!(
                "/executions?workflowId={workflow_id}&limit=10&status=error"
            ));
            let resp = get(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let executions = resp["data"].as_array().cloned().unwrap_or_default();

            if executions.is_empty() {
                return Ok(format!("Aucune exécution en erreur pour le workflow {workflow_id}. Tout va bien !"));
            }

            let mut out = format!("{} exécution(s) en erreur pour workflow {workflow_id} :\n\n", executions.len());
            for e in &executions {
                out.push_str(&format_execution(e));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "n8n_trigger_webhook" => {
            let webhook_url = args["webhook_url"].as_str().ok_or("Paramètre 'webhook_url' requis")?;
            let method      = args["method"].as_str().unwrap_or("POST");

            let resp = if method.eq_ignore_ascii_case("GET") {
                reqwest::Client::new()
                    .get(webhook_url)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .json::<Value>()
                    .await
                    .unwrap_or(json!({"status": "triggered"}))
            } else {
                let body = if args["body"].is_object() { args["body"].clone() } else { json!({}) };
                reqwest::Client::new()
                    .post(webhook_url)
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .json::<Value>()
                    .await
                    .unwrap_or(json!({"status": "triggered"}))
            };

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            Ok(format!(
                "Webhook déclenché ({method} {webhook_url})\nRéponse : {}",
                serde_json::to_string_pretty(&resp).unwrap_or_else(|_| resp.to_string())
            ))
        }

        "n8n_create_workflow" => {
            let name_arg = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let nodes    = args["nodes"].as_array().cloned().unwrap_or_default();
            let active   = args["active"].as_bool().unwrap_or(false);
            let url      = cfg.api("/workflows");
            let body     = json!({
                "name":        name_arg,
                "nodes":       nodes,
                "connections": {},
                "settings":    {},
                "active":      active
            });
            let resp = post_json(&cfg, &url, &body).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let new_id   = &resp["id"];
            let new_name = resp["name"].as_str().unwrap_or(name_arg);
            Ok(format!("Workflow '{new_name}' créé avec succès. ID : {new_id}"))
        }

        "n8n_update_workflow" => {
            let id = args["id"].as_str().ok_or("Paramètre 'id' requis")?;

            // Fetch existing workflow first
            let get_url  = cfg.api(&format!("/workflows/{id}"));
            let existing = get(&cfg, &get_url).await?;

            if let Some(msg) = existing["message"].as_str() {
                return Err(msg.to_string());
            }

            // Merge changes onto the existing workflow
            let mut merged = existing.clone();
            if let Some(new_name) = args["name"].as_str() {
                merged["name"] = json!(new_name);
            }
            if let Some(new_nodes) = args["nodes"].as_array() {
                merged["nodes"] = json!(new_nodes);
            }
            if let Some(new_active) = args["active"].as_bool() {
                merged["active"] = json!(new_active);
            }

            let put_url = cfg.api(&format!("/workflows/{id}"));
            let resp    = put_json(&cfg, &put_url, &merged).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let updated_name = resp["name"].as_str().unwrap_or(id);
            Ok(format!("Workflow '{updated_name}' ({id}) mis à jour avec succès."))
        }

        "n8n_list_credentials" => {
            let url  = cfg.api("/credentials");
            let resp = get(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let credentials = resp["data"].as_array().cloned().unwrap_or_default();

            if credentials.is_empty() {
                return Ok("Aucun credential trouvé.".to_string());
            }

            let mut out = format!("{} credential(s) :\n\n", credentials.len());
            for c in &credentials {
                out.push_str(&format_credential(c));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "n8n_create_credential" => {
            let cred_name = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let cred_type = args["type"].as_str().ok_or("Paramètre 'type' requis")?;
            let data      = args["data"].as_object().ok_or("Paramètre 'data' requis (objet JSON)")?;
            let url       = cfg.api("/credentials");
            let body      = json!({
                "name": cred_name,
                "type": cred_type,
                "data": data
            });
            let resp = post_json(&cfg, &url, &body).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let new_id = &resp["id"];
            Ok(format!("Credential '{cred_name}' (type: {cred_type}) créé avec succès. ID : {new_id}"))
        }

        "n8n_delete_credential" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/credentials/{id}"));
            let resp = delete(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            Ok(format!("Credential {id} supprimé définitivement."))
        }

        "n8n_list_variables" => {
            let url  = cfg.api("/variables");
            let resp = get(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let variables = resp["data"].as_array().cloned().unwrap_or_default();

            if variables.is_empty() {
                return Ok("Aucune variable trouvée.".to_string());
            }

            let mut out = format!("{} variable(s) :\n\n", variables.len());
            for v in &variables {
                out.push_str(&format_variable(v));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "n8n_create_variable" => {
            let key   = args["key"].as_str().ok_or("Paramètre 'key' requis")?;
            let value = args["value"].as_str().ok_or("Paramètre 'value' requis")?;
            let url   = cfg.api("/variables");
            let body  = json!({ "key": key, "value": value });
            let resp  = post_json(&cfg, &url, &body).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let new_id = &resp["id"];
            Ok(format!("Variable '{key}' créée avec succès. ID : {new_id}"))
        }

        "n8n_delete_variable" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/variables/{id}"));
            let resp = delete(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            Ok(format!("Variable {id} supprimée définitivement."))
        }

        "n8n_create_tag" => {
            let tag_name = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let url      = cfg.api("/tags");
            let body     = json!({ "name": tag_name });
            let resp     = post_json(&cfg, &url, &body).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            let new_id = &resp["id"];
            Ok(format!("Tag '{tag_name}' créé avec succès. ID : {new_id}"))
        }

        "n8n_delete_tag" => {
            let id   = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url  = cfg.api(&format!("/tags/{id}"));
            let resp = delete(&cfg, &url).await?;

            if let Some(msg) = resp["message"].as_str() {
                return Err(msg.to_string());
            }

            Ok(format!("Tag {id} supprimé."))
        }

        _ => Err(format!("Tool n8n inconnu : {name}")),
    }
}
