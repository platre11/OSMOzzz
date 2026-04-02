/// Connecteur Supabase natif — Management API v1.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct SupabaseConfig {
    access_token: String,
    project_id:   Option<String>,
}

impl SupabaseConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/supabase.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    /// URL de base Management API
    fn mgmt(&self, path: &str) -> String {
        format!("https://api.supabase.com/v1/{}", path.trim_start_matches('/'))
    }

    /// Résout le project_id depuis les args ou la config.
    fn resolve_project<'a>(&'a self, args: &'a Value) -> Result<&'a str, String> {
        if let Some(pid) = args["project_id"].as_str() {
            if !pid.is_empty() {
                return Ok(pid);
            }
        }
        self.project_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "project_id requis — fournir en argument ou configurer dans supabase.toml".to_string())
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &SupabaseConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.access_token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn get_text(cfg: &SupabaseConfig, url: &str) -> Result<String, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.access_token))
        .header("Accept", "text/plain")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &SupabaseConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.access_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_empty(cfg: &SupabaseConfig, url: &str) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.access_token))
        .header("Accept", "application/json")
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 204 {
        return Ok(json!({"success": true}));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

async fn put_json(cfg: &SupabaseConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .put(url)
        .header("Authorization", format!("Bearer {}", cfg.access_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn delete_req(cfg: &SupabaseConfig, url: &str) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bearer {}", cfg.access_token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 204 {
        return Ok(json!({"success": true}));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Projects ────────────────────────────────────────────────────────
        json!({
            "name": "supabase_list_projects",
            "description": "SUPABASE 📋 — Liste tous les projets Supabase de l'organisation. Retourne id, name, region, status et organisation_id.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "supabase_get_project",
            "description": "SUPABASE 📋 — Récupère le détail complet d'un projet Supabase : nom, région, statut, URL, plan. Utiliser supabase_list_projects pour obtenir un project_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),
        json!({
            "name": "supabase_create_project",
            "description": "SUPABASE ➕ — Crée un nouveau projet Supabase dans une organisation. Retourne le projet créé avec son id et son URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_id":      { "type": "string", "description": "ID de l'organisation Supabase" },
                    "name":        { "type": "string", "description": "Nom du projet" },
                    "db_pass":     { "type": "string", "description": "Mot de passe de la base de données (min 16 caractères)" },
                    "region":      { "type": "string", "description": "Région AWS (ex: eu-west-1, us-east-1 — optionnel)" }
                },
                "required": ["org_id", "name", "db_pass"]
            }
        }),
        json!({
            "name": "supabase_pause_project",
            "description": "SUPABASE ⏸ — Met en pause un projet Supabase (arrête les ressources de calcul, les données sont conservées). Retourne le statut mis à jour.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),
        json!({
            "name": "supabase_restore_project",
            "description": "SUPABASE ▶️ — Restaure (réactive) un projet Supabase qui était en pause. Retourne le statut mis à jour.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),

        // ── Organizations ────────────────────────────────────────────────────
        json!({
            "name": "supabase_list_organizations",
            "description": "SUPABASE 🏢 — Liste toutes les organisations Supabase accessibles avec le token. Retourne id, name et plan.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "supabase_get_organization",
            "description": "SUPABASE 🏢 — Récupère le détail d'une organisation Supabase : nom, plan, membres. Utiliser supabase_list_organizations pour obtenir un org_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_id": { "type": "string", "description": "Slug / ID de l'organisation" }
                },
                "required": ["org_id"]
            }
        }),

        // ── Database ─────────────────────────────────────────────────────────
        json!({
            "name": "supabase_list_tables",
            "description": "SUPABASE 🗄 — Liste toutes les tables d'un projet Supabase avec leur schéma, colonnes et clés étrangères.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "schemas":    { "type": "string", "description": "Filtrer par schéma(s) séparés par virgule (ex: public,auth — optionnel)" }
                }
            }
        }),
        json!({
            "name": "supabase_list_extensions",
            "description": "SUPABASE 🔌 — Liste toutes les extensions PostgreSQL disponibles et leur statut d'activation dans un projet Supabase.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),
        json!({
            "name": "supabase_list_migrations",
            "description": "SUPABASE 📜 — Liste l'historique des migrations de schéma appliquées sur un projet Supabase (name, version, exécutées à quelle date).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),
        json!({
            "name": "supabase_apply_migration",
            "description": "SUPABASE 🔄 — Applique une migration SQL sur un projet Supabase (DDL uniquement : CREATE TABLE, ALTER TABLE, etc.). Retourne le résultat de la migration.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "name":       { "type": "string", "description": "Nom de la migration (ex: create_users_table)" },
                    "query":      { "type": "string", "description": "SQL DDL de la migration" }
                },
                "required": ["name", "query"]
            }
        }),
        json!({
            "name": "supabase_execute_sql",
            "description": "SUPABASE 💾 — Exécute une requête SQL arbitraire (SELECT, INSERT, UPDATE, DELETE) sur un projet Supabase et retourne les résultats. ATTENTION : les modifications sont irréversibles.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "query":      { "type": "string", "description": "Requête SQL à exécuter" }
                },
                "required": ["query"]
            }
        }),

        // ── URLs & Keys ──────────────────────────────────────────────────────
        json!({
            "name": "supabase_get_project_url",
            "description": "SUPABASE 🔗 — Retourne l'URL publique d'un projet Supabase (https://{ref}.supabase.co). À utiliser pour construire les appels REST/Realtime/Storage.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),
        json!({
            "name": "supabase_get_publishable_keys",
            "description": "SUPABASE 🔑 — Récupère les clés API publiques d'un projet Supabase : anon key et service role key. À utiliser pour configurer le client supabase-js.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),

        // ── Advisors ─────────────────────────────────────────────────────────
        json!({
            "name": "supabase_get_advisors",
            "description": "SUPABASE 🔍 — Récupère les recommandations de sécurité ou de performance pour un projet Supabase (security advisor ou performance advisor).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id":    { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "advisor_type":  { "type": "string", "description": "Type d'advisor : security ou performance (défaut: security)", "enum": ["security", "performance"] }
                }
            }
        }),

        // ── Edge Functions ───────────────────────────────────────────────────
        json!({
            "name": "supabase_list_edge_functions",
            "description": "SUPABASE ⚡ — Liste toutes les Edge Functions déployées sur un projet Supabase avec leur slug, statut et date de déploiement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),
        json!({
            "name": "supabase_get_edge_function",
            "description": "SUPABASE ⚡ — Récupère le détail d'une Edge Function Supabase : slug, statut, configuration verify_jwt. Utiliser supabase_list_edge_functions pour la liste.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id":     { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "function_slug":  { "type": "string", "description": "Slug de la Edge Function" }
                },
                "required": ["function_slug"]
            }
        }),
        json!({
            "name": "supabase_deploy_edge_function",
            "description": "SUPABASE ⚡ — Déploie ou met à jour une Edge Function Supabase en fournissant son code source (TypeScript/JavaScript). La fonction est disponible immédiatement après déploiement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id":  { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "name":        { "type": "string", "description": "Nom (slug) de la fonction (ex: hello-world)" },
                    "body_code":   { "type": "string", "description": "Code source TypeScript/JavaScript de la fonction" },
                    "verify_jwt":  { "type": "boolean", "description": "Vérifier le JWT Supabase à chaque appel (défaut: true)" }
                },
                "required": ["name", "body_code"]
            }
        }),

        // ── TypeScript types ─────────────────────────────────────────────────
        json!({
            "name": "supabase_generate_typescript_types",
            "description": "SUPABASE 📝 — Génère les types TypeScript correspondant au schéma de base de données d'un projet Supabase. Retourne le fichier .ts prêt à copier dans le projet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "schemas":    { "type": "string", "description": "Schéma(s) à inclure séparés par virgule (ex: public — optionnel)" }
                }
            }
        }),

        // ── Logs ─────────────────────────────────────────────────────────────
        json!({
            "name": "supabase_get_logs",
            "description": "SUPABASE 📊 — Récupère les logs d'un service d'un projet Supabase (api, postgres, edge-functions, storage). Retourne les entrées les plus récentes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "service":    { "type": "string", "description": "Service à inspecter : api | postgres | edge-functions | storage", "enum": ["api", "postgres", "edge-functions", "storage"] },
                    "limit":      { "type": "integer", "description": "Nombre de lignes de logs (défaut: 100, max: 500)", "default": 100, "minimum": 1, "maximum": 500 },
                    "cursor":     { "type": "string", "description": "Curseur de pagination pour les logs suivants (optionnel)" }
                },
                "required": ["service"]
            }
        }),

        // ── Billing ──────────────────────────────────────────────────────────
        json!({
            "name": "supabase_get_cost",
            "description": "SUPABASE 💰 — Récupère l'utilisation et le coût de facturation actuels d'une organisation Supabase (compute, storage, bandwidth).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_id": { "type": "string", "description": "Slug / ID de l'organisation" }
                },
                "required": ["org_id"]
            }
        }),

        // ── Branches ─────────────────────────────────────────────────────────
        json!({
            "name": "supabase_list_branches",
            "description": "SUPABASE 🌿 — Liste toutes les branches de base de données d'un projet Supabase (Preview Branches). Retourne id, name, statut et git_branch associée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),
        json!({
            "name": "supabase_create_branch",
            "description": "SUPABASE 🌿 — Crée une nouvelle Preview Branch de base de données sur un projet Supabase (associée à une branche git optionnelle).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id":  { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "branch_name": { "type": "string", "description": "Nom de la branch Supabase" },
                    "git_branch":  { "type": "string", "description": "Nom de la branche git associée (optionnel)" }
                },
                "required": ["branch_name"]
            }
        }),
        json!({
            "name": "supabase_delete_branch",
            "description": "SUPABASE 🌿 — Supprime définitivement une Preview Branch Supabase. Les données de la branch sont perdues. Utiliser supabase_list_branches pour obtenir un branch_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "branch_id":  { "type": "string", "description": "ID de la branch Supabase à supprimer" }
                },
                "required": ["branch_id"]
            }
        }),
        json!({
            "name": "supabase_merge_branch",
            "description": "SUPABASE 🌿 — Fusionne les migrations d'une Preview Branch Supabase vers le projet principal (production). Irréversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "branch_id":  { "type": "string", "description": "ID de la branch à fusionner" }
                },
                "required": ["branch_id"]
            }
        }),
        json!({
            "name": "supabase_reset_branch",
            "description": "SUPABASE 🌿 — Réinitialise une Preview Branch Supabase à une version de migration précise ou depuis le début. Toutes les données de la branch sont supprimées.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id":        { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "branch_id":         { "type": "string", "description": "ID de la branch à réinitialiser" },
                    "migration_version": { "type": "string", "description": "Version de migration cible (optionnel — défaut: depuis le début)" }
                },
                "required": ["branch_id"]
            }
        }),
        json!({
            "name": "supabase_rebase_branch",
            "description": "SUPABASE 🌿 — Rebase une Preview Branch Supabase sur le projet principal (rejoue les migrations de production puis celles de la branch).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" },
                    "branch_id":  { "type": "string", "description": "ID de la branch à rebaser" }
                },
                "required": ["branch_id"]
            }
        }),

        // ── Storage ──────────────────────────────────────────────────────────
        json!({
            "name": "supabase_list_storage_buckets",
            "description": "SUPABASE 🪣 — Liste tous les buckets Storage d'un projet Supabase (nom, visibilité publique/privée, taille max des fichiers).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),

        json!({
            "name": "supabase_get_storage_config",
            "description": "SUPABASE 🪣 — Récupère la configuration globale du Storage d'un projet Supabase (limites de taille, options de fichiers).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID (ref) du projet (optionnel si configuré dans supabase.toml)" }
                }
            }
        }),

        // ── Confirm cost ─────────────────────────────────────────────────────
        json!({
            "name": "supabase_confirm_cost",
            "description": "SUPABASE 💰 — Confirme le consentement à un coût additionnel avant d'exécuter une opération payante Supabase (ex: création de projet, upgrade de plan).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_id":   { "type": "string", "description": "ID de l'organisation" },
                    "confirm":  { "type": "boolean", "description": "true pour confirmer le coût" },
                    "message":  { "type": "string",  "description": "Message de confirmation (optionnel)" }
                },
                "required": ["org_id", "confirm"]
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = SupabaseConfig::load()
        .ok_or_else(|| "Supabase non configuré — créer ~/.osmozzz/supabase.toml avec access_token".to_string())?;

    match name {

        // ── Projects ────────────────────────────────────────────────────────

        "supabase_list_projects" => {
            let url  = cfg.mgmt("projects");
            let resp = get(&cfg, &url).await?;

            let projects = resp.as_array().cloned().unwrap_or_default();
            if projects.is_empty() {
                return Ok("Aucun projet Supabase trouvé.".to_string());
            }

            let mut out = format!("{} projet(s) :\n", projects.len());
            for p in &projects {
                let id     = p["id"].as_str().unwrap_or("—");
                let name   = p["name"].as_str().unwrap_or("—");
                let region = p["region"].as_str().unwrap_or("—");
                let status = p["status"].as_str().unwrap_or("—");
                let org_id = p["organization_id"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {name} — région: {region} — statut: {status} — org: {org_id}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "supabase_get_project" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}"));
            let resp = get(&cfg, &url).await?;

            let id     = resp["id"].as_str().unwrap_or(pid);
            let name   = resp["name"].as_str().unwrap_or("—");
            let region = resp["region"].as_str().unwrap_or("—");
            let status = resp["status"].as_str().unwrap_or("—");
            let org_id = resp["organization_id"].as_str().unwrap_or("—");
            let plan   = resp["subscription_id"].as_str().unwrap_or("—");

            Ok(format!(
                "Projet {id}\nNom          : {name}\nRégion       : {region}\nStatut       : {status}\nOrganisation : {org_id}\nPlan         : {plan}\nURL          : https://{id}.supabase.co"
            ))
        }

        "supabase_create_project" => {
            let org_id  = args["org_id"].as_str().ok_or("Paramètre 'org_id' requis")?;
            let name    = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let db_pass = args["db_pass"].as_str().ok_or("Paramètre 'db_pass' requis")?;

            let mut body = json!({
                "organization_id": org_id,
                "name":            name,
                "db_pass":         db_pass
            });
            if let Some(region) = args["region"].as_str() {
                body["region"] = json!(region);
            }

            let url  = cfg.mgmt("projects");
            let resp = post_json(&cfg, &url, &body).await?;

            let id     = resp["id"].as_str().unwrap_or("—");
            let status = resp["status"].as_str().unwrap_or("—");
            Ok(format!(
                "Projet créé.\nID     : {id}\nNom    : {name}\nStatut : {status}\nURL    : https://{id}.supabase.co"
            ))
        }

        "supabase_pause_project" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}/pause"));
            let resp = post_empty(&cfg, &url).await?;

            let status = resp["status"].as_str().unwrap_or("pause en cours");
            Ok(format!("Projet {pid} mis en pause.\nStatut : {status}"))
        }

        "supabase_restore_project" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}/restore"));
            let resp = post_empty(&cfg, &url).await?;

            let status = resp["status"].as_str().unwrap_or("restauration en cours");
            Ok(format!("Projet {pid} restauré.\nStatut : {status}"))
        }

        // ── Organizations ────────────────────────────────────────────────────

        "supabase_list_organizations" => {
            let url  = cfg.mgmt("organizations");
            let resp = get(&cfg, &url).await?;

            let orgs = resp.as_array().cloned().unwrap_or_default();
            if orgs.is_empty() {
                return Ok("Aucune organisation Supabase trouvée.".to_string());
            }

            let mut out = format!("{} organisation(s) :\n", orgs.len());
            for o in &orgs {
                let id   = o["id"].as_str().unwrap_or("—");
                let name = o["name"].as_str().unwrap_or("—");
                let plan = o["billing_email"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {name} — billing: {plan}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "supabase_get_organization" => {
            let org_id = args["org_id"].as_str().ok_or("Paramètre 'org_id' requis")?;
            let url    = cfg.mgmt(&format!("organizations/{org_id}"));
            let resp   = get(&cfg, &url).await?;

            let id    = resp["id"].as_str().unwrap_or(org_id);
            let name  = resp["name"].as_str().unwrap_or("—");
            let email = resp["billing_email"].as_str().unwrap_or("—");

            Ok(format!(
                "Organisation {id}\nNom           : {name}\nEmail billing : {email}"
            ))
        }

        // ── Database ─────────────────────────────────────────────────────────

        "supabase_list_tables" => {
            let pid = cfg.resolve_project(args)?;
            let mut url = cfg.mgmt(&format!("projects/{pid}/database/tables"));
            if let Some(schemas) = args["schemas"].as_str() {
                url.push_str(&format!("?included_schemas={}", schemas));
            }
            let resp = get(&cfg, &url).await?;

            let tables = resp.as_array().cloned().unwrap_or_default();
            if tables.is_empty() {
                return Ok(format!("Aucune table trouvée dans le projet {pid}."));
            }

            let mut out = format!("{} table(s) dans {pid} :\n", tables.len());
            for t in &tables {
                let schema  = t["schema"].as_str().unwrap_or("public");
                let name    = t["name"].as_str().unwrap_or("—");
                let cols    = t["columns"].as_array().map(|c| c.len()).unwrap_or(0);
                out.push_str(&format!("• {schema}.{name} ({cols} colonnes)\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "supabase_list_extensions" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}/database/extensions"));
            let resp = get(&cfg, &url).await?;

            let exts = resp.as_array().cloned().unwrap_or_default();
            if exts.is_empty() {
                return Ok(format!("Aucune extension trouvée dans le projet {pid}."));
            }

            let mut out = format!("{} extension(s) :\n", exts.len());
            for e in &exts {
                let name    = e["name"].as_str().unwrap_or("—");
                let version = e["installed_version"].as_str().unwrap_or("non installée");
                let default = e["default_version"].as_str().unwrap_or("—");
                let status  = if e["installed_version"].is_null() { "inactive" } else { "active" };
                out.push_str(&format!("• {name} [{status}] — installée: {version} — défaut: {default}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "supabase_list_migrations" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}/database/migrations"));
            let resp = get(&cfg, &url).await?;

            let migrations = resp.as_array().cloned().unwrap_or_default();
            if migrations.is_empty() {
                return Ok(format!("Aucune migration dans le projet {pid}."));
            }

            let mut out = format!("{} migration(s) :\n", migrations.len());
            for m in &migrations {
                let version = m["version"].as_str().unwrap_or("—");
                let name    = m["name"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{version}] {name}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "supabase_apply_migration" => {
            let pid   = cfg.resolve_project(args)?;
            let name  = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let query = args["query"].as_str().ok_or("Paramètre 'query' requis")?;

            let body = json!({ "name": name, "query": query });
            let url  = cfg.mgmt(&format!("projects/{pid}/database/migrations"));
            let resp = post_json(&cfg, &url, &body).await?;

            let version = resp["version"].as_str().unwrap_or("—");
            Ok(format!(
                "Migration appliquée.\nNom     : {name}\nVersion : {version}\nProjet  : {pid}"
            ))
        }

        "supabase_execute_sql" => {
            let pid   = cfg.resolve_project(args)?;
            let query = args["query"].as_str().ok_or("Paramètre 'query' requis")?;

            let body = json!({ "query": query });
            let url  = cfg.mgmt(&format!("projects/{pid}/database/query"));
            let resp = post_json(&cfg, &url, &body).await?;

            // Résultat peut être un tableau de lignes ou un objet d'erreur
            if let Some(err) = resp["message"].as_str() {
                return Err(format!("Erreur SQL : {err}"));
            }

            let rows = resp.as_array().cloned().unwrap_or_default();
            if rows.is_empty() {
                return Ok(format!("Requête exécutée avec succès (0 ligne retournée).\nSQL : {query}"));
            }

            let mut out = format!("{} ligne(s) :\n", rows.len());
            for (i, row) in rows.iter().enumerate().take(50) {
                out.push_str(&format!("  [{i}] {row}\n"));
            }
            if rows.len() > 50 {
                out.push_str(&format!("  … ({} lignes supplémentaires tronquées)\n", rows.len() - 50));
            }
            Ok(out.trim_end().to_string())
        }

        // ── URLs & Keys ──────────────────────────────────────────────────────

        "supabase_get_project_url" => {
            let pid = cfg.resolve_project(args)?;
            Ok(format!("URL du projet {pid} :\nhttps://{pid}.supabase.co"))
        }

        "supabase_get_publishable_keys" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}/api-keys"));
            let resp = get(&cfg, &url).await?;

            let keys = resp.as_array().cloned().unwrap_or_default();
            if keys.is_empty() {
                return Ok(format!("Aucune clé API trouvée pour le projet {pid}."));
            }

            let mut out = format!("Clés API du projet {pid} :\n");
            for k in &keys {
                let name   = k["name"].as_str().unwrap_or("—");
                let apikey = k["api_key"].as_str().unwrap_or("—");
                out.push_str(&format!("• {name} : {apikey}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Advisors ─────────────────────────────────────────────────────────

        "supabase_get_advisors" => {
            let pid          = cfg.resolve_project(args)?;
            let advisor_type = args["advisor_type"].as_str().unwrap_or("security");
            let url          = cfg.mgmt(&format!("projects/{pid}/advisors/{advisor_type}"));
            let resp         = get(&cfg, &url).await?;

            let checks = resp.as_array().cloned().unwrap_or_default();
            if checks.is_empty() {
                return Ok(format!("Aucune recommandation {advisor_type} pour le projet {pid}."));
            }

            let mut out = format!("{} recommandation(s) {advisor_type} pour {pid} :\n", checks.len());
            for c in &checks {
                let name   = c["name"].as_str().unwrap_or("—");
                let level  = c["level"].as_str().unwrap_or("info");
                let title  = c["title"].as_str().unwrap_or("—");
                let detail = c["description"].as_str().unwrap_or("");
                out.push_str(&format!("• [{level}] {name} — {title}"));
                if !detail.is_empty() {
                    let snippet = if detail.len() > 120 { &detail[..120] } else { detail };
                    out.push_str(&format!("\n  {snippet}"));
                }
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        // ── Edge Functions ───────────────────────────────────────────────────

        "supabase_list_edge_functions" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}/functions"));
            let resp = get(&cfg, &url).await?;

            let funcs = resp.as_array().cloned().unwrap_or_default();
            if funcs.is_empty() {
                return Ok(format!("Aucune Edge Function dans le projet {pid}."));
            }

            let mut out = format!("{} Edge Function(s) :\n", funcs.len());
            for f in &funcs {
                let slug    = f["slug"].as_str().unwrap_or("—");
                let status  = f["status"].as_str().unwrap_or("—");
                let updated = f["updated_at"].as_str().unwrap_or("—");
                out.push_str(&format!("• {slug} [{status}] — mis à jour: {updated}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "supabase_get_edge_function" => {
            let pid  = cfg.resolve_project(args)?;
            let slug = args["function_slug"].as_str().ok_or("Paramètre 'function_slug' requis")?;
            let url  = cfg.mgmt(&format!("projects/{pid}/functions/{slug}"));
            let resp = get(&cfg, &url).await?;

            let id         = resp["id"].as_str().unwrap_or("—");
            let name       = resp["name"].as_str().unwrap_or(slug);
            let status     = resp["status"].as_str().unwrap_or("—");
            let verify_jwt = resp["verify_jwt"].as_bool().unwrap_or(true);
            let created_at = resp["created_at"].as_str().unwrap_or("—");
            let updated_at = resp["updated_at"].as_str().unwrap_or("—");

            Ok(format!(
                "Edge Function {slug}\nID           : {id}\nNom          : {name}\nStatut       : {status}\nVerify JWT   : {verify_jwt}\nCréée le     : {created_at}\nMise à jour  : {updated_at}\nURL          : https://{pid}.supabase.co/functions/v1/{slug}"
            ))
        }

        "supabase_deploy_edge_function" => {
            let pid        = cfg.resolve_project(args)?;
            let name       = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let body_code  = args["body_code"].as_str().ok_or("Paramètre 'body_code' requis")?;
            let verify_jwt = args["verify_jwt"].as_bool().unwrap_or(true);

            let body = json!({
                "slug":       name,
                "name":       name,
                "body":       body_code,
                "verify_jwt": verify_jwt
            });

            // Tente PUT (update) puis POST (create) si 404
            let url_put  = cfg.mgmt(&format!("projects/{pid}/functions/{name}"));
            let url_post = cfg.mgmt(&format!("projects/{pid}/functions"));

            let resp = match put_json(&cfg, &url_put, &body).await {
                Ok(r) if !r["message"].is_string() => r,
                _  => post_json(&cfg, &url_post, &body).await?,
            };

            let slug   = resp["slug"].as_str().unwrap_or(name);
            let status = resp["status"].as_str().unwrap_or("déployée");
            Ok(format!(
                "Edge Function déployée.\nSlug   : {slug}\nStatut : {status}\nURL    : https://{pid}.supabase.co/functions/v1/{slug}"
            ))
        }

        // ── TypeScript types ─────────────────────────────────────────────────

        "supabase_generate_typescript_types" => {
            let pid = cfg.resolve_project(args)?;
            let mut url = cfg.mgmt(&format!("projects/{pid}/types/typescript"));
            if let Some(schemas) = args["schemas"].as_str() {
                url.push_str(&format!("?included_schemas={}", schemas));
            }

            let types_text = get_text(&cfg, &url).await?;
            let preview = if types_text.len() > 2000 {
                format!("{}\n\n… ({} caractères supplémentaires tronqués pour l'affichage)", &types_text[..2000], types_text.len() - 2000)
            } else {
                types_text
            };

            Ok(format!("Types TypeScript pour le projet {pid} :\n\n{preview}"))
        }

        // ── Logs ─────────────────────────────────────────────────────────────

        "supabase_get_logs" => {
            let pid     = cfg.resolve_project(args)?;
            let service = args["service"].as_str().ok_or("Paramètre 'service' requis (api | postgres | edge-functions | storage)")?;
            let limit   = args["limit"].as_u64().unwrap_or(100).min(500);

            let mut url = cfg.mgmt(&format!("projects/{pid}/logs?service={service}&limit={limit}"));
            if let Some(cursor) = args["cursor"].as_str() {
                url.push_str(&format!("&cursor={cursor}"));
            }

            let resp = get(&cfg, &url).await?;

            let entries = resp["data"].as_array()
                .or_else(|| resp.as_array())
                .cloned()
                .unwrap_or_default();

            if entries.is_empty() {
                return Ok(format!("Aucun log {service} dans le projet {pid}."));
            }

            let mut out = format!("{} entrée(s) de log [{service}] pour {pid} :\n", entries.len());
            for e in &entries {
                let ts      = e["timestamp"].as_str()
                    .or_else(|| e["event_message"].as_str())
                    .unwrap_or("—");
                let message = e["event_message"].as_str()
                    .or_else(|| e["message"].as_str())
                    .unwrap_or("(vide)");
                let snippet = if message.len() > 200 { &message[..200] } else { message };
                out.push_str(&format!("[{ts}] {snippet}\n"));
            }

            // Curseur suivant si disponible
            if let Some(next_cursor) = resp["next_cursor"].as_str() {
                out.push_str(&format!("\nCurseur suivant : {next_cursor}"));
            }

            Ok(out.trim_end().to_string())
        }

        // ── Billing ──────────────────────────────────────────────────────────

        "supabase_get_cost" => {
            let org_id = args["org_id"].as_str().ok_or("Paramètre 'org_id' requis")?;
            let url    = cfg.mgmt(&format!("organizations/{org_id}/billing/usage"));
            let resp   = get(&cfg, &url).await?;

            let period_start = resp["period_start"].as_str().unwrap_or("—");
            let period_end   = resp["period_end"].as_str().unwrap_or("—");

            let mut out = format!("Utilisation/coût de l'organisation {org_id}\nPériode : {period_start} → {period_end}\n\n");

            if let Some(items) = resp["usages"].as_array() {
                for item in items {
                    let metric   = item["metric"].as_str().unwrap_or("—");
                    let usage    = &item["usage"];
                    let limit    = &item["limit"];
                    let cost     = item["cost"].as_f64().unwrap_or(0.0);
                    out.push_str(&format!("• {metric} : usage={usage}, limite={limit}, coût=${cost:.2}\n"));
                }
            } else {
                out.push_str(&format!("{resp}"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Branches ─────────────────────────────────────────────────────────

        "supabase_list_branches" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}/branches"));
            let resp = get(&cfg, &url).await?;

            let branches = resp.as_array().cloned().unwrap_or_default();
            if branches.is_empty() {
                return Ok(format!("Aucune Preview Branch dans le projet {pid}."));
            }

            let mut out = format!("{} branch(es) :\n", branches.len());
            for b in &branches {
                let id         = b["id"].as_str().unwrap_or("—");
                let name       = b["name"].as_str().unwrap_or("—");
                let status     = b["status"].as_str().unwrap_or("—");
                let git_branch = b["git_branch"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {name} [{status}] — git: {git_branch}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "supabase_create_branch" => {
            let pid         = cfg.resolve_project(args)?;
            let branch_name = args["branch_name"].as_str().ok_or("Paramètre 'branch_name' requis")?;

            let mut body = json!({ "branch_name": branch_name });
            if let Some(git_branch) = args["git_branch"].as_str() {
                body["git_branch"] = json!(git_branch);
            }

            let url  = cfg.mgmt(&format!("projects/{pid}/branches"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id     = resp["id"].as_str().unwrap_or("—");
            let status = resp["status"].as_str().unwrap_or("—");
            Ok(format!(
                "Branch créée.\nID      : {id}\nNom     : {branch_name}\nStatut  : {status}\nProjet  : {pid}"
            ))
        }

        "supabase_delete_branch" => {
            let branch_id = args["branch_id"].as_str().ok_or("Paramètre 'branch_id' requis")?;
            let url       = cfg.mgmt(&format!("branches/{branch_id}"));
            delete_req(&cfg, &url).await?;
            Ok(format!("Branch {branch_id} supprimée."))
        }

        "supabase_merge_branch" => {
            let branch_id = args["branch_id"].as_str().ok_or("Paramètre 'branch_id' requis")?;
            let url       = cfg.mgmt(&format!("branches/{branch_id}/merge"));
            let resp      = post_empty(&cfg, &url).await?;

            let status = resp["status"].as_str().unwrap_or("fusion en cours");
            Ok(format!("Branch {branch_id} fusionnée.\nStatut : {status}"))
        }

        "supabase_reset_branch" => {
            let branch_id = args["branch_id"].as_str().ok_or("Paramètre 'branch_id' requis")?;

            let mut body = json!({});
            if let Some(ver) = args["migration_version"].as_str() {
                body["migration_version"] = json!(ver);
            }

            let url  = cfg.mgmt(&format!("branches/{branch_id}/reset"));
            let resp = post_json(&cfg, &url, &body).await?;

            let status = resp["status"].as_str().unwrap_or("réinitialisation en cours");
            Ok(format!("Branch {branch_id} réinitialisée.\nStatut : {status}"))
        }

        "supabase_rebase_branch" => {
            let branch_id = args["branch_id"].as_str().ok_or("Paramètre 'branch_id' requis")?;
            let url       = cfg.mgmt(&format!("branches/{branch_id}/rebase"));
            let resp      = post_empty(&cfg, &url).await?;

            let status = resp["status"].as_str().unwrap_or("rebase en cours");
            Ok(format!("Branch {branch_id} rebasée.\nStatut : {status}"))
        }

        // ── Storage ──────────────────────────────────────────────────────────

        "supabase_list_storage_buckets" => {
            let pid     = cfg.resolve_project(args)?;
            // Storage API via le projet REST
            let url     = format!("https://{pid}.supabase.co/storage/v1/bucket");
            let resp    = reqwest::Client::new()
                .get(&url)
                .header("Authorization", format!("Bearer {}", cfg.access_token))
                .header("apikey", &cfg.access_token)
                .send().await.map_err(|e| e.to_string())?
                .json::<Value>().await.map_err(|e| e.to_string())?;

            let buckets = resp.as_array().cloned().unwrap_or_default();
            if buckets.is_empty() {
                return Ok(format!("Aucun bucket Storage dans le projet {pid}."));
            }
            let mut out = format!("{} bucket(s) :\n", buckets.len());
            for b in &buckets {
                let name    = b["name"].as_str().unwrap_or("—");
                let public  = b["public"].as_bool().unwrap_or(false);
                let size    = b["file_size_limit"].as_u64().map(|s| format!("{} bytes max", s)).unwrap_or_else(|| "illimité".to_string());
                out.push_str(&format!("• {name} — {} — {size}\n", if public { "public" } else { "privé" }));
            }
            Ok(out.trim_end().to_string())
        }

        "supabase_get_storage_config" => {
            let pid  = cfg.resolve_project(args)?;
            let url  = cfg.mgmt(&format!("projects/{pid}/config/storage"));
            let resp = get(&cfg, &url).await?;
            let file_size_limit = resp["fileSizeLimit"].as_u64().unwrap_or(0);
            let storage_img_tf  = resp["storageImgproxyUrl"].as_str().unwrap_or("—");
            Ok(format!(
                "Configuration Storage du projet {pid}\nTaille max fichier : {} MB\nImage proxy URL    : {storage_img_tf}",
                file_size_limit / 1_000_000
            ))
        }

        "supabase_confirm_cost" => {
            let org_id  = args["org_id"].as_str().ok_or("Paramètre 'org_id' requis")?;
            let confirm = args["confirm"].as_bool().ok_or("Paramètre 'confirm' requis")?;
            if !confirm {
                return Ok("Coût refusé. L'opération n'a pas été effectuée.".to_string());
            }
            Ok(format!(
                "Coût confirmé pour l'organisation {org_id}.\nVous pouvez maintenant procéder à l'opération payante."
            ))
        }

        _ => Err(format!("Tool Supabase inconnu : {name}")),
    }
}
