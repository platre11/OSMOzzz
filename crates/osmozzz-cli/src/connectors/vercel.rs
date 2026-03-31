/// Connecteur Vercel — REST API officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct VercelConfig {
    token: String,
    team_id: Option<String>,
}

impl VercelConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/vercel.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    /// Append ?teamId=... (or &teamId=...) if team_id is set.
    fn with_team(&self, url: &str) -> String {
        match &self.team_id {
            Some(tid) if !tid.is_empty() => {
                if url.contains('?') {
                    format!("{}&teamId={}", url, tid)
                } else {
                    format!("{}?teamId={}", url, tid)
                }
            }
            _ => url.to_string(),
        }
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &VercelConfig, url: &str) -> Result<Value, String> {
    let full = cfg.with_team(url);
    reqwest::Client::new()
        .get(&full)
        .bearer_auth(&cfg.token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn patch(cfg: &VercelConfig, url: &str, body: Value) -> Result<Value, String> {
    let full = cfg.with_team(url);
    reqwest::Client::new()
        .patch(&full)
        .bearer_auth(&cfg.token)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_req(cfg: &VercelConfig, url: &str, body: Value) -> Result<Value, String> {
    let full = cfg.with_team(url);
    reqwest::Client::new()
        .post(&full)
        .bearer_auth(&cfg.token)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn delete_req(cfg: &VercelConfig, url: &str) -> Result<reqwest::StatusCode, String> {
    let full = cfg.with_team(url);
    let resp = reqwest::Client::new()
        .delete(&full)
        .bearer_auth(&cfg.token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.status())
}

// ─── Formatters ──────────────────────────────────────────────────────────────

fn fmt_state(state: &str) -> &str {
    match state {
        "READY"    => "✅ READY",
        "ERROR"    => "❌ ERROR",
        "BUILDING" => "⏳ BUILDING",
        "QUEUED"   => "⏳ QUEUED",
        "CANCELED" => "❌ CANCELED",
        other      => other,
    }
}

fn fmt_ts(ms: Option<i64>) -> String {
    match ms {
        Some(ms) => {
            let secs = ms / 1000;
            match chrono::DateTime::from_timestamp(secs, 0) {
                Some(dt) => dt.format("%Y-%m-%d %H:%M UTC").to_string(),
                None     => ms.to_string(),
            }
        }
        None => "—".to_string(),
    }
}

fn fmt_ts_val(v: &Value) -> String {
    fmt_ts(v.as_i64())
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"vercel_list_projects","description":"VERCEL — Liste tous les projets avec leur nom, id, framework et statut du dernier déploiement.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"vercel_get_project","description":"VERCEL — Récupère le détail complet d'un projet (framework, repo lié, derniers déploiements).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou nom du projet Vercel"}},"required":["project_id"]}}),
        json!({"name":"vercel_list_deployments","description":"VERCEL — Liste les déploiements récents. Filtrable par projet. Retourne id, url, état, date, cible (production/preview).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"Filtrer par projet (optionnel)"},"limit":{"type":"integer","default":10,"minimum":1,"maximum":100}}}}),
        json!({"name":"vercel_get_deployment","description":"VERCEL — Récupère le statut détaillé d'un déploiement spécifique (url, état, erreur éventuelle).","inputSchema":{"type":"object","properties":{"deployment_id":{"type":"string","description":"ID du déploiement (ex: dpl_xxx)"}},"required":["deployment_id"]}}),
        json!({"name":"vercel_list_domains","description":"VERCEL — Liste les domaines associés au compte avec leur état de configuration et SSL.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"vercel_list_env","description":"VERCEL — Liste les NOMS des variables d'environnement d'un projet (jamais les valeurs — sécurité). Indique le type (encrypted/plain) et les cibles (production/preview/development).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou nom du projet"}},"required":["project_id"]}}),
        json!({"name":"vercel_cancel_deployment","description":"VERCEL — Annule un déploiement en cours (état BUILDING ou QUEUED).","inputSchema":{"type":"object","properties":{"deployment_id":{"type":"string","description":"ID du déploiement à annuler"}},"required":["deployment_id"]}}),
        json!({"name":"vercel_list_teams","description":"VERCEL — Liste les équipes auxquelles l'utilisateur appartient avec leurs IDs.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"vercel_check_alias","description":"VERCEL — Vérifie vers quel déploiement pointe un alias (ex: myapp.vercel.app).","inputSchema":{"type":"object","properties":{"alias":{"type":"string","description":"Alias à vérifier (ex: myapp.vercel.app)"}},"required":["alias"]}}),
        json!({"name":"vercel_get_build_logs","description":"VERCEL — Récupère les logs de build d'un déploiement (events de type build). Retourne chaque ligne formatée \"[type] message\".","inputSchema":{"type":"object","properties":{"deployment_id":{"type":"string","description":"ID du déploiement (ex: dpl_xxx)"}},"required":["deployment_id"]}}),
        json!({"name":"vercel_redeploy","description":"VERCEL — Redéploie un déploiement existant en production. Récupère automatiquement le nom du projet depuis l'ID du déploiement, puis déclenche un redeploy.","inputSchema":{"type":"object","properties":{"deployment_id":{"type":"string","description":"ID du déploiement à redéployer (ex: dpl_xxx)"}},"required":["deployment_id"]}}),
        json!({"name":"vercel_delete_project","description":"VERCEL — Supprime définitivement un projet Vercel et tous ses déploiements. Action irréversible.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou nom du projet à supprimer"}},"required":["project_id"]}}),
        json!({"name":"vercel_add_domain_to_project","description":"VERCEL — Ajoute un domaine personnalisé à un projet Vercel.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou nom du projet"},"domain":{"type":"string","description":"Nom de domaine à ajouter (ex: monsite.com)"}},"required":["project_id","domain"]}}),
        json!({"name":"vercel_remove_domain_from_project","description":"VERCEL — Retire un domaine d'un projet Vercel.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou nom du projet"},"domain":{"type":"string","description":"Nom de domaine à retirer (ex: monsite.com)"}},"required":["project_id","domain"]}}),
        json!({"name":"vercel_get_project_members","description":"VERCEL — Liste les membres d'un projet avec leur username, email, rôle et uid.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou nom du projet"}},"required":["project_id"]}}),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = match VercelConfig::load() {
        Some(c) => c,
        None    => return Ok("Vercel non configuré (vercel.toml manquant)".to_string()),
    };

    match name {
        "vercel_list_projects" => {
            let url  = "https://api.vercel.com/v9/projects?limit=20";
            let data = get(&cfg, url).await?;
            let projects = data["projects"].as_array().cloned().unwrap_or_default();
            if projects.is_empty() { return Ok("Aucun projet trouvé.".to_string()); }
            Ok(projects.iter().map(|p| {
                let name      = p["name"].as_str().unwrap_or("?");
                let id        = p["id"].as_str().unwrap_or("?");
                let framework = p["framework"].as_str().unwrap_or("—");
                let state     = p["latestDeployments"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|d| d["readyState"].as_str())
                    .unwrap_or("—");
                format!("🚀 {name}\n  id={id} · framework={framework} · dernier deploy={}", fmt_state(state))
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "vercel_get_project" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let url  = format!("https://api.vercel.com/v9/projects/{}", project_id);
            let data = get(&cfg, &url).await?;
            let name      = data["name"].as_str().unwrap_or("?");
            let id        = data["id"].as_str().unwrap_or("?");
            let framework = data["framework"].as_str().unwrap_or("—");
            let repo      = data["link"]["repo"].as_str().unwrap_or("—");
            let deploys   = data["latestDeployments"].as_array().cloned().unwrap_or_default();
            let mut out = format!("🚀 {name}\n  id={id} · framework={framework} · repo={repo}\n\nDerniers déploiements:");
            for d in deploys.iter().take(5) {
                let did    = d["uid"].as_str().unwrap_or("?");
                let durl   = d["url"].as_str().unwrap_or("—");
                let state  = d["readyState"].as_str().unwrap_or("?");
                let target = d["target"].as_str().unwrap_or("preview");
                let ts     = fmt_ts_val(&d["createdAt"]);
                out.push_str(&format!("\n  {} [{}] {} — {} ({})", fmt_state(state), target, durl, did, ts));
            }
            Ok(out)
        }

        "vercel_list_deployments" => {
            let limit = args["limit"].as_u64().unwrap_or(10);
            let mut url = format!("https://api.vercel.com/v6/deployments?limit={}", limit);
            if let Some(pid) = args["project_id"].as_str() {
                url.push_str(&format!("&projectId={}", pid));
            }
            let data  = get(&cfg, &url).await?;
            let deploys = data["deployments"].as_array().cloned().unwrap_or_default();
            if deploys.is_empty() { return Ok("Aucun déploiement trouvé.".to_string()); }
            Ok(deploys.iter().map(|d| {
                let did    = d["uid"].as_str().unwrap_or("?");
                let durl   = d["url"].as_str().unwrap_or("—");
                let state  = d["state"].as_str().unwrap_or("?");
                let target = d["target"].as_str().unwrap_or("preview");
                let ts     = fmt_ts_val(&d["createdAt"]);
                format!("{} [{}] {}\n  id={} · {}", fmt_state(state), target, durl, did, ts)
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "vercel_get_deployment" => {
            let deployment_id = args["deployment_id"].as_str().ok_or("Missing param: deployment_id")?.to_string();
            let url  = format!("https://api.vercel.com/v13/deployments/{}", deployment_id);
            let data = get(&cfg, &url).await?;
            let durl   = data["url"].as_str().unwrap_or("—");
            let state  = data["readyState"].as_str().unwrap_or("?");
            let target = data["target"].as_str().unwrap_or("preview");
            let ts     = fmt_ts_val(&data["createdAt"]);
            let ready  = fmt_ts(data["ready"].as_i64());
            let mut out = format!(
                "{} [{}] {}\n  id={} · créé={} · prêt={}\n",
                fmt_state(state), target, durl, deployment_id, ts, ready
            );
            if let Some(err) = data["errorMessage"].as_str() {
                if !err.is_empty() { out.push_str(&format!("  ❌ Erreur: {}\n", err)); }
            }
            Ok(out)
        }

        "vercel_list_domains" => {
            let url  = "https://api.vercel.com/v5/domains?limit=20";
            let data = get(&cfg, url).await?;
            let domains = data["domains"].as_array().cloned().unwrap_or_default();
            if domains.is_empty() { return Ok("Aucun domaine trouvé.".to_string()); }
            Ok(domains.iter().map(|d| {
                let name       = d["name"].as_str().unwrap_or("?");
                let configured = d["configured"].as_bool().unwrap_or(false);
                let ssl_status = d["ssl"]["state"].as_str().unwrap_or("—");
                let cfg_icon   = if configured { "✅" } else { "❌" };
                format!("{} {}\n  SSL={}", cfg_icon, name, ssl_status)
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "vercel_list_env" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let url  = format!("https://api.vercel.com/v9/projects/{}/env", project_id);
            let data = get(&cfg, &url).await?;
            let envs = data["envs"].as_array().cloned().unwrap_or_default();
            if envs.is_empty() { return Ok("Aucune variable d'environnement.".to_string()); }
            Ok(envs.iter().map(|e| {
                let key     = e["key"].as_str().unwrap_or("?");
                let typ     = e["type"].as_str().unwrap_or("?");
                let targets = e["target"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                    .unwrap_or_else(|| "—".to_string());
                // NEVER show the value — security
                format!("{key}  [{typ}] → {targets}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "vercel_cancel_deployment" => {
            let deployment_id = args["deployment_id"].as_str().ok_or("Missing param: deployment_id")?.to_string();
            let url  = format!("https://api.vercel.com/v12/deployments/{}/cancel", deployment_id);
            let data = patch(&cfg, &url, json!({})).await?;
            let state = data["readyState"].as_str().unwrap_or("?");
            Ok(format!("✅ Déploiement {deployment_id} annulé. État: {}", fmt_state(state)))
        }

        "vercel_list_teams" => {
            // Note: teams endpoint does not require teamId query param
            let url  = format!(
                "https://api.vercel.com/v2/teams?limit=20&accessToken={}",
                cfg.token
            );
            // Use a plain GET without the team suffix here
            let data = reqwest::Client::new()
                .get(&url)
                .bearer_auth(&cfg.token)
                .header("Accept", "application/json")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json::<Value>()
                .await
                .map_err(|e| e.to_string())?;
            let teams = data["teams"].as_array().cloned().unwrap_or_default();
            if teams.is_empty() { return Ok("Aucune équipe trouvée (compte personnel uniquement).".to_string()); }
            Ok(teams.iter().map(|t| {
                let name = t["name"].as_str().unwrap_or("?");
                let slug = t["slug"].as_str().unwrap_or("?");
                let tid  = t["id"].as_str().unwrap_or("?");
                format!("{name} (@{slug})\n  id={tid}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "vercel_check_alias" => {
            let alias = args["alias"].as_str().ok_or("Missing param: alias")?.to_string();
            let url   = format!("https://api.vercel.com/v4/aliases/{}", alias);
            let data  = get(&cfg, &url).await?;
            let dep_id  = data["deploymentId"].as_str().unwrap_or("?");
            let dep_url = data["deployment"]["url"].as_str().unwrap_or("—");
            let created = fmt_ts_val(&data["createdAt"]);
            Ok(format!("Alias: {alias}\n  → déploiement={dep_id}\n  url={dep_url}\n  créé={created}"))
        }

        "vercel_get_build_logs" => {
            let deployment_id = args["deployment_id"].as_str().ok_or("Missing param: deployment_id")?.to_string();
            let url  = format!("https://api.vercel.com/v2/deployments/{}/events?builds=1&limit=100", deployment_id);
            let data = get(&cfg, &url).await?;
            let events = match data.as_array() {
                Some(arr) => arr.clone(),
                None => data["events"].as_array().cloned().unwrap_or_default(),
            };
            if events.is_empty() { return Ok("Aucun log de build disponible.".to_string()); }
            let lines: Vec<String> = events.iter().map(|e| {
                let typ  = e["type"].as_str().unwrap_or("log");
                let msg  = e["payload"]["text"].as_str()
                    .or_else(|| e["text"].as_str())
                    .or_else(|| e["message"].as_str())
                    .unwrap_or("");
                format!("[{typ}] {msg}")
            }).collect();
            Ok(lines.join("\n"))
        }

        "vercel_redeploy" => {
            let deployment_id = args["deployment_id"].as_str().ok_or("Missing param: deployment_id")?.to_string();
            // Fetch deployment to get its name
            let get_url  = format!("https://api.vercel.com/v13/deployments/{}", deployment_id);
            let dep_data = get(&cfg, &get_url).await?;
            let dep_name = dep_data["name"].as_str().unwrap_or("").to_string();
            if dep_name.is_empty() {
                return Err(format!("Impossible de récupérer le nom du déploiement {deployment_id}"));
            }
            let post_url = format!("https://api.vercel.com/v13/deployments/{}/redeploy", deployment_id);
            let body     = json!({ "name": dep_name, "target": "production" });
            let result   = post_req(&cfg, &post_url, body).await?;
            let new_id   = result["id"].as_str().unwrap_or("?");
            let new_url  = result["url"].as_str().unwrap_or("—");
            let state    = result["readyState"].as_str().unwrap_or("?");
            Ok(format!(
                "✅ Redeploy lancé pour {dep_name}\n  nouveau id={new_id}\n  url={new_url}\n  état={}",
                fmt_state(state)
            ))
        }

        "vercel_delete_project" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let url    = format!("https://api.vercel.com/v9/projects/{}", project_id);
            let status = delete_req(&cfg, &url).await?;
            if status == reqwest::StatusCode::NO_CONTENT || status.is_success() {
                Ok(format!("✅ Projet {project_id} supprimé définitivement."))
            } else {
                Err(format!("Échec de la suppression du projet {project_id} (HTTP {})", status))
            }
        }

        "vercel_add_domain_to_project" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let domain     = args["domain"].as_str().ok_or("Missing param: domain")?.to_string();
            let url        = format!("https://api.vercel.com/v9/projects/{}/domains", project_id);
            let body       = json!({ "name": domain });
            let result     = post_req(&cfg, &url, body).await?;
            let name       = result["name"].as_str().unwrap_or(&domain);
            let verified   = result["verified"].as_bool().unwrap_or(false);
            let verified_icon = if verified { "✅" } else { "⏳ (vérification en attente)" };
            Ok(format!("Domaine ajouté au projet {project_id}\n  domaine={name}\n  vérifié={verified_icon}"))
        }

        "vercel_remove_domain_from_project" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let domain     = args["domain"].as_str().ok_or("Missing param: domain")?.to_string();
            let url        = format!("https://api.vercel.com/v9/projects/{}/domains/{}", project_id, domain);
            let status     = delete_req(&cfg, &url).await?;
            if status == reqwest::StatusCode::NO_CONTENT || status.is_success() {
                Ok(format!("✅ Domaine {domain} retiré du projet {project_id}."))
            } else {
                Err(format!("Échec du retrait du domaine {domain} (HTTP {})", status))
            }
        }

        "vercel_get_project_members" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let url        = format!("https://api.vercel.com/v9/projects/{}/members?limit=20", project_id);
            let data       = get(&cfg, &url).await?;
            let members    = data["members"].as_array().cloned().unwrap_or_default();
            if members.is_empty() { return Ok("Aucun membre trouvé pour ce projet.".to_string()); }
            Ok(members.iter().map(|m| {
                let username = m["username"].as_str().unwrap_or("?");
                let email    = m["email"].as_str().unwrap_or("—");
                let role     = m["role"].as_str().unwrap_or("—");
                let uid      = m["uid"].as_str().unwrap_or("?");
                format!("{username} ({email})\n  rôle={role} · uid={uid}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        other => Err(format!("Unknown vercel tool: {other}")),
    }
}
