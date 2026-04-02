/// Connecteur Cloudflare natif — API v4 officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct CloudflareConfig {
    api_token:  String,
    account_id: String,
}

impl CloudflareConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/cloudflare.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!(
            "https://api.cloudflare.com/client/v4/{}",
            path.trim_start_matches('/')
        )
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &CloudflareConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.api_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &CloudflareConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.api_token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn patch_json(cfg: &CloudflareConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .patch(url)
        .header("Authorization", format!("Bearer {}", cfg.api_token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn put_text(cfg: &CloudflareConfig, url: &str, text_body: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .put(url)
        .header("Authorization", format!("Bearer {}", cfg.api_token))
        .header("Content-Type", "text/plain")
        .body(text_body.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn delete_req(cfg: &CloudflareConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bearer {}", cfg.api_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

/// Pour KV get_value — l'endpoint retourne du texte brut, pas du JSON.
async fn get_text(cfg: &CloudflareConfig, url: &str) -> Result<String, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.api_token))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

/// Pour Worker deploy — envoie le script JS en application/javascript (PUT direct).
/// L'API Cloudflare accepte le body raw pour les scripts simples sans bindings.
async fn put_worker_script(
    cfg: &CloudflareConfig,
    url: &str,
    script_content: &str,
) -> Result<Value, String> {
    reqwest::Client::new()
        .put(url)
        .header("Authorization", format!("Bearer {}", cfg.api_token))
        .header("Content-Type", "application/javascript")
        .body(script_content.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Vérifie le champ `success` de la réponse Cloudflare API v4.
fn check_success(resp: &Value) -> Result<(), String> {
    if resp["success"].as_bool() == Some(false) {
        let errors = resp["errors"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|e| {
                        let code = e["code"].as_u64().unwrap_or(0);
                        let msg  = e["message"].as_str().unwrap_or("erreur inconnue");
                        format!("[{code}] {msg}")
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "Erreur inconnue".to_string());
        return Err(format!("Cloudflare API error: {errors}"));
    }
    Ok(())
}

/// Extrait `result` après vérification du succès.
fn extract_result(resp: Value) -> Result<Value, String> {
    check_success(&resp)?;
    Ok(resp["result"].clone())
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Workers ─────────────────────────────────────────────────────────
        json!({
            "name": "cloudflare_list_workers",
            "description": "CLOUDFLARE ☁️ — Liste tous les Workers Scripts du compte. Retourne nom, taille, date de modification de chaque script.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "cloudflare_get_worker",
            "description": "CLOUDFLARE ☁️ — Récupère les métadonnées d'un Worker Script (taille, état, date). Utiliser cloudflare_list_workers pour obtenir le script_name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "script_name": { "type": "string", "description": "Nom du script Worker" }
                },
                "required": ["script_name"]
            }
        }),
        json!({
            "name": "cloudflare_deploy_worker",
            "description": "CLOUDFLARE ☁️ — Déploie (crée ou met à jour) un Worker Script. Envoie le contenu JS/TS du script via multipart/form-data.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "script_name":    { "type": "string", "description": "Nom du script à déployer" },
                    "script_content": { "type": "string", "description": "Contenu du script JavaScript/TypeScript" }
                },
                "required": ["script_name", "script_content"]
            }
        }),
        json!({
            "name": "cloudflare_delete_worker",
            "description": "CLOUDFLARE ☁️ — Supprime définitivement un Worker Script. Action irréversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "script_name": { "type": "string", "description": "Nom du script à supprimer" }
                },
                "required": ["script_name"]
            }
        }),
        json!({
            "name": "cloudflare_list_worker_routes",
            "description": "CLOUDFLARE ☁️ — Liste les routes (URL patterns) associées à un Worker pour une zone DNS. Retourne le pattern, le script associé et l'ID de route.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "zone_id": { "type": "string", "description": "ID de la zone DNS (utiliser cloudflare_list_zones pour l'obtenir)" }
                },
                "required": ["zone_id"]
            }
        }),
        json!({
            "name": "cloudflare_create_worker_route",
            "description": "CLOUDFLARE ☁️ — Crée une route qui associe un pattern d'URL à un Worker Script pour une zone DNS donnée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "zone_id": { "type": "string", "description": "ID de la zone DNS" },
                    "pattern": { "type": "string", "description": "Pattern d'URL ex: exemple.com/api/*" },
                    "script":  { "type": "string", "description": "Nom du Worker à associer (optionnel — laisse vide pour désactiver)" }
                },
                "required": ["zone_id", "pattern"]
            }
        }),

        // ── KV Storage ──────────────────────────────────────────────────────
        json!({
            "name": "cloudflare_list_kv_namespaces",
            "description": "CLOUDFLARE ☁️ — Liste tous les namespaces KV (Workers KV) du compte. Retourne ID, titre et préfixes de chaque namespace.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "cloudflare_create_kv_namespace",
            "description": "CLOUDFLARE ☁️ — Crée un nouveau namespace KV (Workers KV). Retourne l'ID du namespace créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Nom du namespace KV à créer" }
                },
                "required": ["title"]
            }
        }),
        json!({
            "name": "cloudflare_list_kv_keys",
            "description": "CLOUDFLARE ☁️ — Liste les clés dans un namespace KV. Supporte filtrage par préfixe, pagination via cursor.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "namespace_id": { "type": "string", "description": "ID du namespace KV" },
                    "prefix":       { "type": "string", "description": "Filtrer les clés par préfixe (optionnel)" },
                    "limit":        { "type": "integer", "description": "Nombre max de clés (1-1000, défaut 1000)" },
                    "cursor":       { "type": "string", "description": "Curseur de pagination (optionnel)" }
                },
                "required": ["namespace_id"]
            }
        }),
        json!({
            "name": "cloudflare_get_kv_value",
            "description": "CLOUDFLARE ☁️ — Récupère la valeur (texte brut) d'une clé dans un namespace KV.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "namespace_id": { "type": "string", "description": "ID du namespace KV" },
                    "key":          { "type": "string", "description": "Nom de la clé à lire" }
                },
                "required": ["namespace_id", "key"]
            }
        }),
        json!({
            "name": "cloudflare_put_kv_value",
            "description": "CLOUDFLARE ☁️ — Écrit ou met à jour une valeur dans un namespace KV. Supporte un TTL optionnel en secondes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "namespace_id":    { "type": "string", "description": "ID du namespace KV" },
                    "key":             { "type": "string", "description": "Nom de la clé" },
                    "value":           { "type": "string", "description": "Valeur à stocker" },
                    "expiration_ttl":  { "type": "integer", "description": "Durée de vie en secondes (optionnel)" }
                },
                "required": ["namespace_id", "key", "value"]
            }
        }),
        json!({
            "name": "cloudflare_delete_kv_value",
            "description": "CLOUDFLARE ☁️ — Supprime une clé dans un namespace KV.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "namespace_id": { "type": "string", "description": "ID du namespace KV" },
                    "key":          { "type": "string", "description": "Nom de la clé à supprimer" }
                },
                "required": ["namespace_id", "key"]
            }
        }),

        // ── R2 Storage ──────────────────────────────────────────────────────
        json!({
            "name": "cloudflare_list_r2_buckets",
            "description": "CLOUDFLARE ☁️ — Liste tous les buckets R2 du compte. Retourne nom, région et date de création de chaque bucket.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "cloudflare_create_r2_bucket",
            "description": "CLOUDFLARE ☁️ — Crée un nouveau bucket R2. La localisation est optionnelle (ex: WEUR, EEUR, APAC).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":     { "type": "string", "description": "Nom du bucket R2 (unique dans le compte)" },
                    "location": { "type": "string", "description": "Région de stockage : WEUR, EEUR, APAC (optionnel)" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "cloudflare_delete_r2_bucket",
            "description": "CLOUDFLARE ☁️ — Supprime définitivement un bucket R2 vide. Le bucket doit être vide avant suppression.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Nom du bucket R2 à supprimer" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "cloudflare_list_r2_objects",
            "description": "CLOUDFLARE ☁️ — Liste les objets dans un bucket R2. Supporte filtrage par préfixe et délimiteur (pour simuler des dossiers).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "bucket":    { "type": "string", "description": "Nom du bucket R2" },
                    "prefix":    { "type": "string", "description": "Filtrer par préfixe de chemin (optionnel)" },
                    "delimiter": { "type": "string", "description": "Délimiteur de chemin ex: '/' pour simuler des dossiers (optionnel)" },
                    "max_keys":  { "type": "integer", "description": "Nombre max d'objets à retourner (défaut 1000)" }
                },
                "required": ["bucket"]
            }
        }),

        // ── D1 Database ──────────────────────────────────────────────────────
        json!({
            "name": "cloudflare_list_d1_databases",
            "description": "CLOUDFLARE ☁️ — Liste toutes les bases de données D1 (SQLite serverless) du compte. Retourne ID, nom, version et taille de chaque base.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "cloudflare_create_d1_database",
            "description": "CLOUDFLARE ☁️ — Crée une nouvelle base de données D1. Retourne l'UUID de la base créée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Nom de la base D1 à créer" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "cloudflare_query_d1",
            "description": "CLOUDFLARE ☁️ — Exécute une requête SQL sur une base D1. Supporte SELECT, INSERT, UPDATE, DELETE. Les paramètres sont passés en tableau pour éviter les injections SQL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database_id": { "type": "string", "description": "UUID de la base D1 (utiliser cloudflare_list_d1_databases)" },
                    "sql":         { "type": "string", "description": "Requête SQL à exécuter (ex: SELECT * FROM users WHERE id = ?)" },
                    "params":      { "type": "array",  "description": "Paramètres de la requête SQL (optionnel)", "items": {} }
                },
                "required": ["database_id", "sql"]
            }
        }),
        json!({
            "name": "cloudflare_get_d1_database",
            "description": "CLOUDFLARE ☁️ — Récupère les détails d'une base D1 : nom, version, taille, nombre de tables.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database_id": { "type": "string", "description": "UUID de la base D1" }
                },
                "required": ["database_id"]
            }
        }),

        // ── DNS & Zones ──────────────────────────────────────────────────────
        json!({
            "name": "cloudflare_list_zones",
            "description": "CLOUDFLARE ☁️ — Liste toutes les zones DNS du compte (domaines gérés). Retourne ID, nom de domaine, statut et plan de chaque zone.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "cloudflare_list_dns_records",
            "description": "CLOUDFLARE ☁️ — Liste les enregistrements DNS d'une zone. Filtrable par type (A, CNAME, MX…), nom ou contenu.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "zone_id":  { "type": "string", "description": "ID de la zone DNS (utiliser cloudflare_list_zones)" },
                    "type":     { "type": "string", "description": "Type DNS : A, AAAA, CNAME, MX, TXT, NS… (optionnel)" },
                    "name":     { "type": "string", "description": "Filtrer par nom d'hôte (optionnel)" },
                    "content":  { "type": "string", "description": "Filtrer par valeur/contenu (optionnel)" },
                    "per_page": { "type": "integer", "description": "Résultats par page (défaut 100, max 5000)" }
                },
                "required": ["zone_id"]
            }
        }),
        json!({
            "name": "cloudflare_create_dns_record",
            "description": "CLOUDFLARE ☁️ — Crée un enregistrement DNS dans une zone. Supporte tous les types (A, AAAA, CNAME, MX, TXT, etc.). Le proxy Cloudflare est activable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "zone_id": { "type": "string", "description": "ID de la zone DNS" },
                    "type":    { "type": "string", "description": "Type DNS : A, AAAA, CNAME, MX, TXT, NS, SRV…" },
                    "name":    { "type": "string", "description": "Nom de l'hôte ex: www, @ pour root" },
                    "content": { "type": "string", "description": "Valeur de l'enregistrement (IP, domaine, texte…)" },
                    "ttl":     { "type": "integer", "description": "TTL en secondes (1 = auto, min 60)" },
                    "proxied": { "type": "boolean", "description": "Activer le proxy Cloudflare (orange cloud) — défaut false" }
                },
                "required": ["zone_id", "type", "name", "content"]
            }
        }),
        json!({
            "name": "cloudflare_update_dns_record",
            "description": "CLOUDFLARE ☁️ — Met à jour un enregistrement DNS existant (PATCH partiel). Utiliser cloudflare_list_dns_records pour obtenir le record_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "zone_id":   { "type": "string", "description": "ID de la zone DNS" },
                    "record_id": { "type": "string", "description": "ID de l'enregistrement DNS à modifier" },
                    "type":      { "type": "string", "description": "Nouveau type DNS (optionnel)" },
                    "name":      { "type": "string", "description": "Nouveau nom (optionnel)" },
                    "content":   { "type": "string", "description": "Nouvelle valeur (optionnel)" },
                    "ttl":       { "type": "integer", "description": "Nouveau TTL (optionnel)" },
                    "proxied":   { "type": "boolean", "description": "Activer/désactiver proxy Cloudflare (optionnel)" }
                },
                "required": ["zone_id", "record_id"]
            }
        }),
        json!({
            "name": "cloudflare_delete_dns_record",
            "description": "CLOUDFLARE ☁️ — Supprime définitivement un enregistrement DNS. Action irréversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "zone_id":   { "type": "string", "description": "ID de la zone DNS" },
                    "record_id": { "type": "string", "description": "ID de l'enregistrement DNS à supprimer" }
                },
                "required": ["zone_id", "record_id"]
            }
        }),

        // ── Pages ────────────────────────────────────────────────────────────
        json!({
            "name": "cloudflare_list_pages_projects",
            "description": "CLOUDFLARE ☁️ — Liste tous les projets Cloudflare Pages du compte. Retourne nom, domaine de production, branche de production et dernière date de déploiement.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "cloudflare_get_pages_project",
            "description": "CLOUDFLARE ☁️ — Récupère les détails complets d'un projet Cloudflare Pages : configuration de build, domaines, dernier déploiement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_name": { "type": "string", "description": "Nom du projet Pages (utiliser cloudflare_list_pages_projects)" }
                },
                "required": ["project_name"]
            }
        }),
        json!({
            "name": "cloudflare_list_pages_deployments",
            "description": "CLOUDFLARE ☁️ — Liste les déploiements d'un projet Cloudflare Pages. Retourne ID, statut, branche, commit et date de chaque déploiement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_name": { "type": "string", "description": "Nom du projet Pages" }
                },
                "required": ["project_name"]
            }
        }),
        json!({
            "name": "cloudflare_retry_pages_deployment",
            "description": "CLOUDFLARE ☁️ — Rejoue (retry) un déploiement Cloudflare Pages échoué ou à redéployer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_name":  { "type": "string", "description": "Nom du projet Pages" },
                    "deployment_id": { "type": "string", "description": "ID du déploiement à rejouer (utiliser cloudflare_list_pages_deployments)" }
                },
                "required": ["project_name", "deployment_id"]
            }
        }),

        // ── Analytics / Account ──────────────────────────────────────────────
        json!({
            "name": "cloudflare_get_account_details",
            "description": "CLOUDFLARE ☁️ — Récupère les informations du compte Cloudflare : nom, statut, plan, date de création.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
    ]
}

// ─── Dispatch ────────────────────────────────────────────────────────────────

pub async fn handle(tool: &str, args: &Value) -> Result<String, String> {
    let cfg = CloudflareConfig::load()
        .ok_or_else(|| "Cloudflare non configuré — créer ~/.osmozzz/cloudflare.toml avec api_token et account_id".to_string())?;

    match tool {
        // ── Workers ─────────────────────────────────────────────────────────

        "cloudflare_list_workers" => {
            let url  = cfg.api(&format!("accounts/{}/workers/scripts", cfg.account_id));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let scripts = result.as_array().cloned().unwrap_or_default();
            if scripts.is_empty() {
                return Ok("Aucun Worker Script trouvé dans ce compte.".to_string());
            }

            let mut out = format!("{} Worker Script(s) :\n", scripts.len());
            for s in &scripts {
                let id         = s["id"].as_str().unwrap_or("—");
                let created    = s["created_on"].as_str().unwrap_or("—");
                let modified   = s["modified_on"].as_str().unwrap_or("—");
                out.push_str(&format!("• {id}\n  Créé : {created} | Modifié : {modified}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_get_worker" => {
            let script_name = args["script_name"].as_str().ok_or("Paramètre 'script_name' requis")?;
            let url  = cfg.api(&format!("accounts/{}/workers/scripts/{}", cfg.account_id, script_name));
            let resp = get(&cfg, &url).await?;
            check_success(&resp)?;

            let id       = resp["result"]["id"].as_str().unwrap_or(script_name);
            let created  = resp["result"]["created_on"].as_str().unwrap_or("—");
            let modified = resp["result"]["modified_on"].as_str().unwrap_or("—");

            Ok(format!(
                "Worker Script : {id}\nCréé    : {created}\nModifié : {modified}"
            ))
        }

        "cloudflare_deploy_worker" => {
            let script_name    = args["script_name"].as_str().ok_or("Paramètre 'script_name' requis")?;
            let script_content = args["script_content"].as_str().ok_or("Paramètre 'script_content' requis")?;
            let url  = cfg.api(&format!("accounts/{}/workers/scripts/{}", cfg.account_id, script_name));
            let resp = put_worker_script(&cfg, &url, script_content).await?;
            check_success(&resp)?;

            let id       = resp["result"]["id"].as_str().unwrap_or(script_name);
            let modified = resp["result"]["modified_on"].as_str().unwrap_or("—");
            Ok(format!(
                "Worker déployé.\nNom     : {id}\nModifié : {modified}"
            ))
        }

        "cloudflare_delete_worker" => {
            let script_name = args["script_name"].as_str().ok_or("Paramètre 'script_name' requis")?;
            let url  = cfg.api(&format!("accounts/{}/workers/scripts/{}", cfg.account_id, script_name));
            let resp = delete_req(&cfg, &url).await?;
            check_success(&resp)?;
            Ok(format!("Worker Script '{script_name}' supprimé."))
        }

        "cloudflare_list_worker_routes" => {
            let zone_id = args["zone_id"].as_str().ok_or("Paramètre 'zone_id' requis")?;
            let url  = cfg.api(&format!("zones/{zone_id}/workers/routes"));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let routes = result.as_array().cloned().unwrap_or_default();
            if routes.is_empty() {
                return Ok(format!("Aucune route Worker pour la zone {zone_id}."));
            }

            let mut out = format!("{} route(s) :\n", routes.len());
            for r in &routes {
                let id      = r["id"].as_str().unwrap_or("—");
                let pattern = r["pattern"].as_str().unwrap_or("—");
                let script  = r["script"].as_str().unwrap_or("(aucun)");
                out.push_str(&format!("• [{id}] {pattern} → {script}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_create_worker_route" => {
            let zone_id = args["zone_id"].as_str().ok_or("Paramètre 'zone_id' requis")?;
            let pattern = args["pattern"].as_str().ok_or("Paramètre 'pattern' requis")?;

            let mut body = json!({ "pattern": pattern });
            if let Some(script) = args["script"].as_str() {
                body["script"] = json!(script);
            }

            let url  = cfg.api(&format!("zones/{zone_id}/workers/routes"));
            let resp = post_json(&cfg, &url, &body).await?;
            let result = extract_result(resp)?;

            let id = result["id"].as_str().unwrap_or("—");
            Ok(format!(
                "Route Worker créée.\nID      : {id}\nPattern : {pattern}\nZone    : {zone_id}"
            ))
        }

        // ── KV Storage ──────────────────────────────────────────────────────

        "cloudflare_list_kv_namespaces" => {
            let url  = cfg.api(&format!("accounts/{}/storage/kv/namespaces", cfg.account_id));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let ns = result.as_array().cloned().unwrap_or_default();
            if ns.is_empty() {
                return Ok("Aucun namespace KV trouvé dans ce compte.".to_string());
            }

            let mut out = format!("{} namespace(s) KV :\n", ns.len());
            for n in &ns {
                let id    = n["id"].as_str().unwrap_or("—");
                let title = n["title"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {title}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_create_kv_namespace" => {
            let title = args["title"].as_str().ok_or("Paramètre 'title' requis")?;
            let url   = cfg.api(&format!("accounts/{}/storage/kv/namespaces", cfg.account_id));
            let body  = json!({ "title": title });
            let resp  = post_json(&cfg, &url, &body).await?;
            let result = extract_result(resp)?;

            let id = result["id"].as_str().unwrap_or("—");
            Ok(format!(
                "Namespace KV créé.\nID    : {id}\nTitre : {title}"
            ))
        }

        "cloudflare_list_kv_keys" => {
            let namespace_id = args["namespace_id"].as_str().ok_or("Paramètre 'namespace_id' requis")?;
            let limit        = args["limit"].as_u64().unwrap_or(1000).min(1000);

            let mut url = cfg.api(&format!(
                "accounts/{}/storage/kv/namespaces/{}/keys?limit={limit}",
                cfg.account_id, namespace_id
            ));
            if let Some(prefix) = args["prefix"].as_str() {
                url.push_str(&format!("&prefix={}", urlencoding_simple(prefix)));
            }
            if let Some(cursor) = args["cursor"].as_str() {
                url.push_str(&format!("&cursor={cursor}"));
            }

            let resp   = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let keys = result.as_array().cloned().unwrap_or_default();
            if keys.is_empty() {
                return Ok(format!("Aucune clé dans le namespace {namespace_id}."));
            }

            let mut out = format!("{} clé(s) :\n", keys.len());
            for k in &keys {
                let name       = k["name"].as_str().unwrap_or("—");
                let expiration = k["expiration"].as_u64()
                    .map(|e| format!(" (expire: {e})"))
                    .unwrap_or_default();
                out.push_str(&format!("• {name}{expiration}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_get_kv_value" => {
            let namespace_id = args["namespace_id"].as_str().ok_or("Paramètre 'namespace_id' requis")?;
            let key          = args["key"].as_str().ok_or("Paramètre 'key' requis")?;
            let url          = cfg.api(&format!(
                "accounts/{}/storage/kv/namespaces/{}/values/{}",
                cfg.account_id, namespace_id, urlencoding_simple(key)
            ));
            let value = get_text(&cfg, &url).await?;
            Ok(format!("Clé : {key}\nValeur :\n{value}"))
        }

        "cloudflare_put_kv_value" => {
            let namespace_id = args["namespace_id"].as_str().ok_or("Paramètre 'namespace_id' requis")?;
            let key          = args["key"].as_str().ok_or("Paramètre 'key' requis")?;
            let value        = args["value"].as_str().ok_or("Paramètre 'value' requis")?;

            let mut url = cfg.api(&format!(
                "accounts/{}/storage/kv/namespaces/{}/values/{}",
                cfg.account_id, namespace_id, urlencoding_simple(key)
            ));
            if let Some(ttl) = args["expiration_ttl"].as_u64() {
                url.push_str(&format!("?expiration_ttl={ttl}"));
            }

            let resp = put_text(&cfg, &url, value).await?;
            check_success(&resp)?;
            Ok(format!("Clé '{key}' mise à jour dans le namespace {namespace_id}."))
        }

        "cloudflare_delete_kv_value" => {
            let namespace_id = args["namespace_id"].as_str().ok_or("Paramètre 'namespace_id' requis")?;
            let key          = args["key"].as_str().ok_or("Paramètre 'key' requis")?;
            let url          = cfg.api(&format!(
                "accounts/{}/storage/kv/namespaces/{}/values/{}",
                cfg.account_id, namespace_id, urlencoding_simple(key)
            ));
            let resp = delete_req(&cfg, &url).await?;
            check_success(&resp)?;
            Ok(format!("Clé '{key}' supprimée du namespace {namespace_id}."))
        }

        // ── R2 Storage ──────────────────────────────────────────────────────

        "cloudflare_list_r2_buckets" => {
            let url  = cfg.api(&format!("accounts/{}/r2/buckets", cfg.account_id));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let buckets = result["buckets"].as_array().cloned()
                .or_else(|| result.as_array().cloned())
                .unwrap_or_default();
            if buckets.is_empty() {
                return Ok("Aucun bucket R2 trouvé dans ce compte.".to_string());
            }

            let mut out = format!("{} bucket(s) R2 :\n", buckets.len());
            for b in &buckets {
                let name     = b["name"].as_str().unwrap_or("—");
                let location = b["location"].as_str().unwrap_or("—");
                let created  = b["creation_date"].as_str().unwrap_or("—");
                out.push_str(&format!("• {name} — région: {location} | créé: {created}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_create_r2_bucket" => {
            let name = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let url  = cfg.api(&format!("accounts/{}/r2/buckets", cfg.account_id));

            let mut body = json!({ "name": name });
            if let Some(loc) = args["location"].as_str() {
                body["locationHint"] = json!(loc);
            }

            let resp = post_json(&cfg, &url, &body).await?;
            check_success(&resp)?;
            Ok(format!("Bucket R2 '{name}' créé avec succès."))
        }

        "cloudflare_delete_r2_bucket" => {
            let name = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let url  = cfg.api(&format!("accounts/{}/r2/buckets/{}", cfg.account_id, name));
            let resp = delete_req(&cfg, &url).await?;
            check_success(&resp)?;
            Ok(format!("Bucket R2 '{name}' supprimé."))
        }

        "cloudflare_list_r2_objects" => {
            let bucket = args["bucket"].as_str().ok_or("Paramètre 'bucket' requis")?;
            let max_keys = args["max_keys"].as_u64().unwrap_or(1000);

            let mut url = cfg.api(&format!(
                "accounts/{}/r2/buckets/{}/objects?max_keys={max_keys}",
                cfg.account_id, bucket
            ));
            if let Some(prefix) = args["prefix"].as_str() {
                url.push_str(&format!("&prefix={}", urlencoding_simple(prefix)));
            }
            if let Some(delim) = args["delimiter"].as_str() {
                url.push_str(&format!("&delimiter={}", urlencoding_simple(delim)));
            }

            let resp   = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let objects = result["objects"].as_array().cloned()
                .or_else(|| result.as_array().cloned())
                .unwrap_or_default();

            if objects.is_empty() {
                return Ok(format!("Aucun objet dans le bucket '{bucket}'."));
            }

            let mut out = format!("{} objet(s) dans '{bucket}' :\n", objects.len());
            for o in &objects {
                let key  = o["key"].as_str().unwrap_or("—");
                let size = o["size"].as_u64().unwrap_or(0);
                let modified = o["uploaded"].as_str()
                    .or_else(|| o["lastModified"].as_str())
                    .unwrap_or("—");
                out.push_str(&format!("• {key} — {size} octets | modifié: {modified}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── D1 Database ──────────────────────────────────────────────────────

        "cloudflare_list_d1_databases" => {
            let url  = cfg.api(&format!("accounts/{}/d1/database", cfg.account_id));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let dbs = result.as_array().cloned().unwrap_or_default();
            if dbs.is_empty() {
                return Ok("Aucune base D1 trouvée dans ce compte.".to_string());
            }

            let mut out = format!("{} base(s) D1 :\n", dbs.len());
            for db in &dbs {
                let uuid    = db["uuid"].as_str().unwrap_or("—");
                let name    = db["name"].as_str().unwrap_or("—");
                let version = db["version"].as_str().unwrap_or("—");
                let size    = db["file_size"].as_u64().unwrap_or(0);
                out.push_str(&format!("• [{uuid}] {name} — v{version} | {size} octets\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_create_d1_database" => {
            let name = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let url  = cfg.api(&format!("accounts/{}/d1/database", cfg.account_id));
            let body = json!({ "name": name });
            let resp = post_json(&cfg, &url, &body).await?;
            let result = extract_result(resp)?;

            let uuid = result["uuid"].as_str().unwrap_or("—");
            Ok(format!(
                "Base D1 créée.\nNom  : {name}\nUUID : {uuid}"
            ))
        }

        "cloudflare_query_d1" => {
            let database_id = args["database_id"].as_str().ok_or("Paramètre 'database_id' requis")?;
            let sql         = args["sql"].as_str().ok_or("Paramètre 'sql' requis")?;

            let mut body = json!({ "sql": sql });
            if let Some(params) = args["params"].as_array() {
                body["params"] = json!(params);
            }

            let url  = cfg.api(&format!("accounts/{}/d1/database/{}/query", cfg.account_id, database_id));
            let resp = post_json(&cfg, &url, &body).await?;
            let result = extract_result(resp)?;

            // result est un tableau de résultats (une entrée par statement)
            let results_arr = result.as_array().cloned().unwrap_or_default();
            if results_arr.is_empty() {
                return Ok("Requête exécutée. Aucun résultat retourné.".to_string());
            }

            let first = &results_arr[0];
            let rows = first["results"].as_array().cloned().unwrap_or_default();
            let meta = &first["meta"];
            let rows_read    = meta["rows_read"].as_u64().unwrap_or(0);
            let rows_written = meta["rows_written"].as_u64().unwrap_or(0);
            let duration_ms  = meta["duration"].as_f64().unwrap_or(0.0);

            let mut out = format!(
                "Requête exécutée en {duration_ms:.1}ms | lignes lues: {rows_read} | lignes écrites: {rows_written}\n"
            );

            if rows.is_empty() {
                out.push_str("Aucun résultat.\n");
            } else {
                out.push_str(&format!("{} ligne(s) :\n", rows.len()));
                for (i, row) in rows.iter().enumerate().take(50) {
                    out.push_str(&format!("  [{i}] {}\n", serde_json::to_string(row).unwrap_or_default()));
                }
                if rows.len() > 50 {
                    out.push_str(&format!("  … {} lignes supplémentaires.\n", rows.len() - 50));
                }
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_get_d1_database" => {
            let database_id = args["database_id"].as_str().ok_or("Paramètre 'database_id' requis")?;
            let url  = cfg.api(&format!("accounts/{}/d1/database/{}", cfg.account_id, database_id));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let uuid    = result["uuid"].as_str().unwrap_or(database_id);
            let name    = result["name"].as_str().unwrap_or("—");
            let version = result["version"].as_str().unwrap_or("—");
            let size    = result["file_size"].as_u64().unwrap_or(0);
            let created = result["created_at"].as_str().unwrap_or("—");

            Ok(format!(
                "Base D1 : {name}\nUUID    : {uuid}\nVersion : {version}\nTaille  : {size} octets\nCréée   : {created}"
            ))
        }

        // ── DNS & Zones ──────────────────────────────────────────────────────

        "cloudflare_list_zones" => {
            let url  = cfg.api(&format!("zones?account.id={}", cfg.account_id));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let zones = result.as_array().cloned().unwrap_or_default();
            if zones.is_empty() {
                return Ok("Aucune zone DNS trouvée dans ce compte.".to_string());
            }

            let mut out = format!("{} zone(s) DNS :\n", zones.len());
            for z in &zones {
                let id     = z["id"].as_str().unwrap_or("—");
                let name   = z["name"].as_str().unwrap_or("—");
                let status = z["status"].as_str().unwrap_or("—");
                let plan   = z["plan"]["name"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {name} — statut: {status} | plan: {plan}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_list_dns_records" => {
            let zone_id  = args["zone_id"].as_str().ok_or("Paramètre 'zone_id' requis")?;
            let per_page = args["per_page"].as_u64().unwrap_or(100).min(5000);

            let mut url = cfg.api(&format!("zones/{zone_id}/dns_records?per_page={per_page}"));
            if let Some(t) = args["type"].as_str()    { url.push_str(&format!("&type={t}")); }
            if let Some(n) = args["name"].as_str()    { url.push_str(&format!("&name={n}")); }
            if let Some(c) = args["content"].as_str() { url.push_str(&format!("&content={}", urlencoding_simple(c))); }

            let resp   = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let records = result.as_array().cloned().unwrap_or_default();
            if records.is_empty() {
                return Ok(format!("Aucun enregistrement DNS pour la zone {zone_id}."));
            }

            let mut out = format!("{} enregistrement(s) DNS :\n", records.len());
            for r in &records {
                let id      = r["id"].as_str().unwrap_or("—");
                let rtype   = r["type"].as_str().unwrap_or("—");
                let name    = r["name"].as_str().unwrap_or("—");
                let content = r["content"].as_str().unwrap_or("—");
                let ttl     = r["ttl"].as_u64().unwrap_or(1);
                let proxied = r["proxied"].as_bool().unwrap_or(false);
                let proxy_str = if proxied { "☁️ proxied" } else { "DNS only" };
                out.push_str(&format!("• [{id}] {rtype} {name} → {content} (TTL:{ttl}, {proxy_str})\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_create_dns_record" => {
            let zone_id = args["zone_id"].as_str().ok_or("Paramètre 'zone_id' requis")?;
            let rtype   = args["type"].as_str().ok_or("Paramètre 'type' requis")?;
            let name    = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let content = args["content"].as_str().ok_or("Paramètre 'content' requis")?;

            let mut body = json!({
                "type":    rtype,
                "name":    name,
                "content": content
            });
            if let Some(ttl)     = args["ttl"].as_u64()      { body["ttl"]     = json!(ttl); }
            if let Some(proxied) = args["proxied"].as_bool()  { body["proxied"] = json!(proxied); }

            let url  = cfg.api(&format!("zones/{zone_id}/dns_records"));
            let resp = post_json(&cfg, &url, &body).await?;
            let result = extract_result(resp)?;

            let id      = result["id"].as_str().unwrap_or("—");
            let proxied = result["proxied"].as_bool().unwrap_or(false);
            let proxy_str = if proxied { "☁️ proxied" } else { "DNS only" };
            Ok(format!(
                "Enregistrement DNS créé.\nID      : {id}\nType    : {rtype}\nNom     : {name}\nContenu : {content}\nProxy   : {proxy_str}"
            ))
        }

        "cloudflare_update_dns_record" => {
            let zone_id   = args["zone_id"].as_str().ok_or("Paramètre 'zone_id' requis")?;
            let record_id = args["record_id"].as_str().ok_or("Paramètre 'record_id' requis")?;

            let mut body = json!({});
            if let Some(t) = args["type"].as_str()     { body["type"]    = json!(t); }
            if let Some(n) = args["name"].as_str()     { body["name"]    = json!(n); }
            if let Some(c) = args["content"].as_str()  { body["content"] = json!(c); }
            if let Some(ttl) = args["ttl"].as_u64()    { body["ttl"]     = json!(ttl); }
            if let Some(p) = args["proxied"].as_bool() { body["proxied"] = json!(p); }

            if body.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                return Err("Au moins un champ parmi type, name, content, ttl, proxied est requis".to_string());
            }

            let url  = cfg.api(&format!("zones/{zone_id}/dns_records/{record_id}"));
            let resp = patch_json(&cfg, &url, &body).await?;
            let result = extract_result(resp)?;

            let id      = result["id"].as_str().unwrap_or(record_id);
            let rtype   = result["type"].as_str().unwrap_or("—");
            let name    = result["name"].as_str().unwrap_or("—");
            let content = result["content"].as_str().unwrap_or("—");
            Ok(format!("Enregistrement DNS {id} mis à jour : {rtype} {name} → {content}"))
        }

        "cloudflare_delete_dns_record" => {
            let zone_id   = args["zone_id"].as_str().ok_or("Paramètre 'zone_id' requis")?;
            let record_id = args["record_id"].as_str().ok_or("Paramètre 'record_id' requis")?;
            let url  = cfg.api(&format!("zones/{zone_id}/dns_records/{record_id}"));
            let resp = delete_req(&cfg, &url).await?;
            check_success(&resp)?;
            Ok(format!("Enregistrement DNS {record_id} supprimé de la zone {zone_id}."))
        }

        // ── Pages ────────────────────────────────────────────────────────────

        "cloudflare_list_pages_projects" => {
            let url  = cfg.api(&format!("accounts/{}/pages/projects", cfg.account_id));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let projects = result.as_array().cloned().unwrap_or_default();
            if projects.is_empty() {
                return Ok("Aucun projet Cloudflare Pages trouvé.".to_string());
            }

            let mut out = format!("{} projet(s) Pages :\n", projects.len());
            for p in &projects {
                let name        = p["name"].as_str().unwrap_or("—");
                let subdomain   = p["subdomain"].as_str().unwrap_or("—");
                let prod_branch = p["production_branch"].as_str().unwrap_or("—");
                let last_deploy = p["latest_deployment"]["created_on"].as_str().unwrap_or("—");
                out.push_str(&format!(
                    "• {name}\n  Domaine: {subdomain} | Branche: {prod_branch} | Dernier deploy: {last_deploy}\n"
                ));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_get_pages_project" => {
            let project_name = args["project_name"].as_str().ok_or("Paramètre 'project_name' requis")?;
            let url  = cfg.api(&format!("accounts/{}/pages/projects/{}", cfg.account_id, project_name));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let name        = result["name"].as_str().unwrap_or(project_name);
            let subdomain   = result["subdomain"].as_str().unwrap_or("—");
            let prod_branch = result["production_branch"].as_str().unwrap_or("—");
            let created     = result["created_on"].as_str().unwrap_or("—");
            let build_cmd   = result["build_config"]["build_command"].as_str().unwrap_or("—");
            let dest_dir    = result["build_config"]["destination_dir"].as_str().unwrap_or("—");
            let last_status = result["latest_deployment"]["latest_stage"]["status"].as_str().unwrap_or("—");

            Ok(format!(
                "Projet Pages : {name}\nDomaine       : {subdomain}\nBranche prod  : {prod_branch}\nCréé          : {created}\nBuild command : {build_cmd}\nDossier dest  : {dest_dir}\nDernier statut: {last_status}"
            ))
        }

        "cloudflare_list_pages_deployments" => {
            let project_name = args["project_name"].as_str().ok_or("Paramètre 'project_name' requis")?;
            let url  = cfg.api(&format!(
                "accounts/{}/pages/projects/{}/deployments",
                cfg.account_id, project_name
            ));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let deploys = result.as_array().cloned().unwrap_or_default();
            if deploys.is_empty() {
                return Ok(format!("Aucun déploiement pour le projet '{project_name}'."));
            }

            let mut out = format!("{} déploiement(s) pour '{project_name}' :\n", deploys.len());
            for d in deploys.iter().take(20) {
                let id      = d["id"].as_str().unwrap_or("—");
                let env     = d["environment"].as_str().unwrap_or("—");
                let status  = d["latest_stage"]["status"].as_str().unwrap_or("—");
                let branch  = d["deployment_trigger"]["metadata"]["branch"].as_str().unwrap_or("—");
                let commit  = d["deployment_trigger"]["metadata"]["commit_message"].as_str().unwrap_or("—");
                let created = d["created_on"].as_str().unwrap_or("—");
                out.push_str(&format!(
                    "• [{id}] {env} | {status} | {branch} — {commit} | {created}\n"
                ));
            }
            if deploys.len() > 20 {
                out.push_str(&format!("  … {} déploiements supplémentaires.\n", deploys.len() - 20));
            }
            Ok(out.trim_end().to_string())
        }

        "cloudflare_retry_pages_deployment" => {
            let project_name  = args["project_name"].as_str().ok_or("Paramètre 'project_name' requis")?;
            let deployment_id = args["deployment_id"].as_str().ok_or("Paramètre 'deployment_id' requis")?;
            let url  = cfg.api(&format!(
                "accounts/{}/pages/projects/{}/deployments/{}/retry",
                cfg.account_id, project_name, deployment_id
            ));
            let resp = post_json(&cfg, &url, &json!({})).await?;
            let result = extract_result(resp)?;

            let new_id  = result["id"].as_str().unwrap_or("—");
            let created = result["created_on"].as_str().unwrap_or("—");
            Ok(format!(
                "Déploiement relancé.\nNouvel ID : {new_id}\nProjet    : {project_name}\nCréé      : {created}"
            ))
        }

        // ── Account ──────────────────────────────────────────────────────────

        "cloudflare_get_account_details" => {
            let url  = cfg.api(&format!("accounts/{}", cfg.account_id));
            let resp = get(&cfg, &url).await?;
            let result = extract_result(resp)?;

            let id      = result["id"].as_str().unwrap_or("—");
            let name    = result["name"].as_str().unwrap_or("—");
            let status  = result["settings"]["enforce_twofactor"].as_bool()
                .map(|b| if b { "2FA activée" } else { "2FA désactivée" })
                .unwrap_or("—");
            let created = result["created_on"].as_str().unwrap_or("—");

            Ok(format!(
                "Compte Cloudflare\nID      : {id}\nNom     : {name}\nSécurité: {status}\nCréé    : {created}"
            ))
        }

        _ => Err(format!("Tool Cloudflare inconnu : {tool}")),
    }
}

// ─── Utilitaire d'encodage URL minimal ───────────────────────────────────────

/// Encode les caractères spéciaux les plus courants pour les query strings.
/// Utilise une implémentation légère sans dépendance externe.
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
