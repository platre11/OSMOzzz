/// Connecteur Render — REST API v1 officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct RenderConfig { token: String }

impl RenderConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/render.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
    fn api(&self, path: &str) -> String {
        format!("https://api.render.com/v1/{}", path.trim_start_matches('/'))
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &RenderConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post(cfg: &RenderConfig, url: &str, body: Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn put(cfg: &RenderConfig, url: &str, body: Value) -> Result<Value, String> {
    reqwest::Client::new()
        .put(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post_empty(cfg: &RenderConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn delete_req(cfg: &RenderConfig, url: &str) -> Result<(), String> {
    let resp = reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("Render API error: HTTP {}", resp.status()))
    }
}

// ─── Formatters ──────────────────────────────────────────────────────────────

fn fmt_service_type(t: &str) -> &str {
    match t {
        "web_service"       => "Web Service",
        "static_site"       => "Static Site",
        "private_service"   => "Private Service",
        "background_worker" => "Background Worker",
        "cron_job"          => "Cron Job",
        other               => other,
    }
}

fn fmt_service_status(s: &str) -> &str {
    match s {
        "live"         => "✅ live",
        "suspended"    => "⏸️ suspended",
        "not_deployed" => "⚪ not_deployed",
        other          => other,
    }
}

fn fmt_deploy_status(s: &str) -> &str {
    match s {
        "live"          => "✅ live",
        "build_failed"  => "❌ build_failed",
        "update_failed" => "❌ update_failed",
        "canceled"      => "🚫 canceled",
        "deactivated"   => "⏸️ deactivated",
        other           => other,
    }
}

/// Extract the service object from either a bare object or a { service: {...} } wrapper.
fn svc(v: &Value) -> &Value {
    if v.get("service").is_some() { &v["service"] } else { v }
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"render_list_services","description":"RENDER 🎨 — Liste tous les services du compte avec id, nom, type (Web Service/Static Site/…), statut (live/suspended/not_deployed) et URL. Point de départ pour toutes les opérations Render.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"render_get_service","description":"RENDER 🎨 — Récupère le détail complet d'un service : URL, environnement, autoDeploy, branche, dépôt Git. Enchaîner avec render_list_deploys pour l'historique des déploiements.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render (obtenu via render_list_services)"}},"required":["service_id"]}}),
        json!({"name":"render_list_deploys","description":"RENDER 🎨 — Liste les déploiements récents d'un service avec statut, message de commit et dates. Retourne les 10 derniers par défaut.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"},"limit":{"type":"integer","default":10,"minimum":1,"maximum":50}},"required":["service_id"]}}),
        json!({"name":"render_get_deploy","description":"RENDER 🎨 — Récupère le détail complet d'un déploiement : timing, statut, informations de commit. Utiliser render_list_deploys pour obtenir le deploy_id.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"},"deploy_id":{"type":"string","description":"ID du déploiement (obtenu via render_list_deploys)"}},"required":["service_id","deploy_id"]}}),
        json!({"name":"render_trigger_deploy","description":"RENDER 🎨 — Déclenche un nouveau déploiement pour un service. Optionnel : vider le cache de build avec clear_cache=true.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"},"clear_cache":{"type":"boolean","description":"Vider le cache de build (défaut: false)","default":false}},"required":["service_id"]}}),
        json!({"name":"render_list_env_vars","description":"RENDER 🎨 — Liste les NOMS des variables d'environnement d'un service (jamais les valeurs — sécurité). Utile pour vérifier qu'une variable est bien configurée.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"}},"required":["service_id"]}}),
        json!({"name":"render_put_env_var","description":"RENDER 🎨 — Ajoute ou met à jour une variable d'environnement sur un service Render. Déclenche un redéploiement automatique si autoDeploy est activé.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"},"key":{"type":"string","description":"Nom de la variable d'environnement"},"value":{"type":"string","description":"Valeur de la variable"}},"required":["service_id","key","value"]}}),
        json!({"name":"render_suspend_service","description":"RENDER 🎨 — Suspend un service Render (arrête le service, stoppe la facturation compute). Le service peut être repris avec render_resume_service.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"}},"required":["service_id"]}}),
        json!({"name":"render_resume_service","description":"RENDER 🎨 — Reprend un service Render précédemment suspendu (redéploiement depuis le dernier build).","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"}},"required":["service_id"]}}),
        json!({"name":"render_get_logs","description":"RENDER 🎨 — Récupère les logs récents d'un service (stdout/stderr). Retourne les entrées avec timestamp et message. Utile pour diagnostiquer des erreurs de déploiement ou de runtime.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"},"limit":{"type":"integer","default":100,"minimum":1,"maximum":500,"description":"Nombre de lignes de log à retourner (défaut: 100, max: 500)"}},"required":["service_id"]}}),
        json!({"name":"render_list_custom_domains","description":"RENDER 🎨 — Liste les domaines personnalisés d'un service avec leur id, nom de domaine et statut de vérification (verified/unverified).","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"}},"required":["service_id"]}}),
        json!({"name":"render_add_custom_domain","description":"RENDER 🎨 — Ajoute un domaine personnalisé à un service Render. Le domaine doit pointer vers Render via DNS pour être vérifié.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"},"domain":{"type":"string","description":"Nom de domaine à ajouter (ex: www.monsite.com)"}},"required":["service_id","domain"]}}),
        json!({"name":"render_delete_custom_domain","description":"RENDER 🎨 — Supprime un domaine personnalisé d'un service Render. Utiliser render_list_custom_domains pour obtenir le custom_domain_id.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"},"custom_domain_id":{"type":"string","description":"ID du domaine personnalisé (obtenu via render_list_custom_domains)"}},"required":["service_id","custom_domain_id"]}}),
        json!({"name":"render_scale_service","description":"RENDER 🎨 — Met à l'échelle un service Render en ajustant le nombre d'instances (1 à 10). Utile pour gérer la charge ou réduire les coûts.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Render"},"num_instances":{"type":"integer","description":"Nombre d'instances souhaité (1 à 10)","minimum":1,"maximum":10}},"required":["service_id","num_instances"]}}),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = match RenderConfig::load() {
        Some(c) => c,
        None    => return Ok("Render non configuré (render.toml manquant)".to_string()),
    };

    match name {
        "render_list_services" => {
            let url  = cfg.api("services?limit=20");
            let data = get(&cfg, &url).await?;
            let items = data.as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Aucun service trouvé.".to_string()); }
            Ok(items.iter().map(|item| {
                let s    = svc(item);
                let id   = s["id"].as_str().unwrap_or("?");
                let name = s["name"].as_str().unwrap_or("?");
                let typ  = fmt_service_type(s["type"].as_str().unwrap_or(""));
                let stat = fmt_service_status(s["suspended"].as_str()
                    .unwrap_or_else(|| {
                        // Render API may return serviceDetails.url when live, or notDeployed flag
                        "live"
                    }));
                // Actual status lives under different keys depending on service type
                let status_str = {
                    let raw = s["serviceDetails"]["status"].as_str()
                        .or_else(|| s["status"].as_str())
                        .unwrap_or("unknown");
                    fmt_service_status(raw)
                };
                let url_str = s["serviceDetails"]["url"].as_str().unwrap_or("—");
                let _ = stat; // suppress unused warning from the closure above
                format!("🎨 {name} ({typ})\n  id={id} · {status_str}\n  url={url_str}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "render_get_service" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let url  = cfg.api(&format!("services/{}", service_id));
            let data = get(&cfg, &url).await?;
            let s    = svc(&data);
            let name        = s["name"].as_str().unwrap_or("?");
            let typ         = fmt_service_type(s["type"].as_str().unwrap_or(""));
            let status_raw  = s["serviceDetails"]["status"].as_str()
                .or_else(|| s["status"].as_str())
                .unwrap_or("unknown");
            let status      = fmt_service_status(status_raw);
            let svc_url     = s["serviceDetails"]["url"].as_str().unwrap_or("—");
            let env         = s["serviceDetails"]["env"].as_str().unwrap_or("—");
            let auto_deploy = s["autoDeploy"].as_str().unwrap_or("—");
            let branch      = s["branch"].as_str()
                .or_else(|| s["serviceDetails"]["branch"].as_str())
                .unwrap_or("—");
            let repo        = s["repo"].as_str()
                .or_else(|| s["serviceDetails"]["repoURL"].as_str())
                .or_else(|| s["serviceDetails"]["repo"].as_str())
                .unwrap_or("—");
            let created_at  = s["createdAt"].as_str().unwrap_or("—");
            let updated_at  = s["updatedAt"].as_str().unwrap_or("—");
            Ok(format!(
                "🎨 {name} ({typ})\n  id={service_id} · {status}\n  url={svc_url}\n  env={env} · autoDeploy={auto_deploy}\n  branch={branch}\n  repo={repo}\n  créé={created_at} · mis à jour={updated_at}"
            ))
        }

        "render_list_deploys" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let limit      = args["limit"].as_u64().unwrap_or(10);
            let url  = cfg.api(&format!("services/{}/deploys?limit={}", service_id, limit));
            let data = get(&cfg, &url).await?;
            let items = data.as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Aucun déploiement trouvé.".to_string()); }
            Ok(items.iter().map(|item| {
                // Render wraps each item in { "deploy": {...} }
                let d          = if item.get("deploy").is_some() { &item["deploy"] } else { item };
                let id         = d["id"].as_str().unwrap_or("?");
                let status     = fmt_deploy_status(d["status"].as_str().unwrap_or("?"));
                let msg        = d["commit"]["message"].as_str().unwrap_or("—");
                let commit_ts  = d["commit"]["createdAt"].as_str().unwrap_or("—");
                let created_at = d["createdAt"].as_str().unwrap_or("—");
                format!("{status}\n  id={id} · créé={created_at}\n  commit=[{commit_ts}] {msg}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "render_get_deploy" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let deploy_id  = args["deploy_id"].as_str().ok_or("Missing param: deploy_id")?.to_string();
            let url  = cfg.api(&format!("services/{}/deploys/{}", service_id, deploy_id));
            let data = get(&cfg, &url).await?;
            let d    = if data.get("deploy").is_some() { &data["deploy"] } else { &data };
            let id         = d["id"].as_str().unwrap_or("?");
            let status     = fmt_deploy_status(d["status"].as_str().unwrap_or("?"));
            let created_at = d["createdAt"].as_str().unwrap_or("—");
            let updated_at = d["updatedAt"].as_str().unwrap_or("—");
            let commit_id  = d["commit"]["id"].as_str().unwrap_or("—");
            let commit_msg = d["commit"]["message"].as_str().unwrap_or("—");
            let commit_ts  = d["commit"]["createdAt"].as_str().unwrap_or("—");
            Ok(format!(
                "🎨 Deploy {status}\n  id={id}\n  créé={created_at} · mis à jour={updated_at}\n  commit id={commit_id}\n  commit date={commit_ts}\n  commit msg={commit_msg}"
            ))
        }

        "render_trigger_deploy" => {
            let service_id  = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let clear_cache = args["clear_cache"].as_bool().unwrap_or(false);
            let cache_val   = if clear_cache { "clear" } else { "do_not_clear" };
            let url  = cfg.api(&format!("services/{}/deploys", service_id));
            let body = json!({"clearCache": cache_val});
            let data = post(&cfg, &url, body).await?;
            let d    = if data.get("deploy").is_some() { &data["deploy"] } else { &data };
            let id   = d["id"].as_str().unwrap_or("?");
            Ok(format!("✅ Déploiement déclenché.\n  deploy_id={id}\n  clear_cache={clear_cache}"))
        }

        "render_list_env_vars" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let url  = cfg.api(&format!("services/{}/env-vars", service_id));
            let data = get(&cfg, &url).await?;
            let items = data.as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Aucune variable d'environnement trouvée.".to_string()); }
            // Each item may be { "envVar": { "key": "...", "value": "..." } } or bare
            let keys: Vec<String> = items.iter().map(|item| {
                let ev = if item.get("envVar").is_some() { &item["envVar"] } else { item };
                ev["key"].as_str().unwrap_or("?").to_string()
            }).collect();
            Ok(format!(
                "Variables d'environnement ({}) — noms uniquement (valeurs masquées) :\n{}",
                keys.len(),
                keys.join("\n")
            ))
        }

        "render_put_env_var" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let key        = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let value      = args["value"].as_str().ok_or("Missing param: value")?.to_string();
            let url  = cfg.api(&format!("services/{}/env-vars", service_id));
            let body = json!([{"key": key, "value": value}]);
            put(&cfg, &url, body).await?;
            Ok(format!("✅ Variable {key} définie sur le service {service_id}."))
        }

        "render_suspend_service" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let url = cfg.api(&format!("services/{}/suspend", service_id));
            post_empty(&cfg, &url).await?;
            Ok(format!("⏸️ Service {service_id} suspendu."))
        }

        "render_resume_service" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let url = cfg.api(&format!("services/{}/resume", service_id));
            post_empty(&cfg, &url).await?;
            Ok(format!("✅ Service {service_id} repris."))
        }

        "render_get_logs" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let limit      = args["limit"].as_u64().unwrap_or(100).min(500);
            let url  = cfg.api(&format!("services/{}/logs?limit={}", service_id, limit));
            let data = get(&cfg, &url).await?;
            let items = data.as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Aucun log disponible.".to_string()); }
            let lines: Vec<String> = items.iter().map(|item| {
                // Render may return { "log": { "timestamp": "...", "message": "..." } } or bare
                let entry = if item.get("log").is_some() { &item["log"] } else { item };
                let ts  = entry["timestamp"].as_str().unwrap_or("—");
                let msg = entry["message"].as_str().unwrap_or("—");
                format!("[{ts}] {msg}")
            }).collect();
            Ok(format!("Logs du service {} ({} entrées) :\n{}", service_id, lines.len(), lines.join("\n")))
        }

        "render_list_custom_domains" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let url  = cfg.api(&format!("services/{}/custom-domains", service_id));
            let data = get(&cfg, &url).await?;
            let items = data.as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Aucun domaine personnalisé configuré.".to_string()); }
            let lines: Vec<String> = items.iter().map(|item| {
                // Render may wrap in { "customDomain": {...} } or return bare
                let cd = if item.get("customDomain").is_some() { &item["customDomain"] } else { item };
                let id     = cd["id"].as_str().unwrap_or("?");
                let domain = cd["name"].as_str()
                    .or_else(|| cd["domain"].as_str())
                    .unwrap_or("?");
                let status = cd["verificationStatus"].as_str()
                    .or_else(|| cd["status"].as_str())
                    .unwrap_or("unknown");
                let status_icon = if status == "verified" { "✅" } else { "⚠️" };
                format!("{status_icon} {domain}\n  id={id} · statut={status}")
            }).collect();
            Ok(format!("Domaines personnalisés du service {} :\n\n{}", service_id, lines.join("\n\n")))
        }

        "render_add_custom_domain" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let domain     = args["domain"].as_str().ok_or("Missing param: domain")?.to_string();
            let url  = cfg.api(&format!("services/{}/custom-domains", service_id));
            let body = json!({"name": domain});
            let data = post(&cfg, &url, body).await?;
            let cd = if data.get("customDomain").is_some() { &data["customDomain"] } else { &data };
            let id     = cd["id"].as_str().unwrap_or("?");
            let status = cd["verificationStatus"].as_str()
                .or_else(|| cd["status"].as_str())
                .unwrap_or("pending");
            Ok(format!("✅ Domaine {domain} ajouté au service {service_id}.\n  id={id} · statut={status}"))
        }

        "render_delete_custom_domain" => {
            let service_id       = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let custom_domain_id = args["custom_domain_id"].as_str().ok_or("Missing param: custom_domain_id")?.to_string();
            let url = cfg.api(&format!("services/{}/custom-domains/{}", service_id, custom_domain_id));
            delete_req(&cfg, &url).await?;
            Ok(format!("✅ Domaine personnalisé {custom_domain_id} supprimé du service {service_id}."))
        }

        "render_scale_service" => {
            let service_id    = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let num_instances = args["num_instances"].as_u64().ok_or("Missing param: num_instances")?;
            if num_instances < 1 || num_instances > 10 {
                return Err("num_instances doit être compris entre 1 et 10.".to_string());
            }
            let url  = cfg.api(&format!("services/{}/scale", service_id));
            let body = json!({"numInstances": num_instances});
            post(&cfg, &url, body).await?;
            Ok(format!("✅ Service {service_id} mis à l'échelle : {num_instances} instance(s)."))
        }

        other => Err(format!("Unknown render tool: {other}")),
    }
}
