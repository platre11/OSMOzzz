/// Connecteur Railway — GraphQL API officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct RailwayConfig {
    token: String,
}

impl RailwayConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/railway.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
}

// ─── GraphQL helper ───────────────────────────────────────────────────────────

async fn gql(token: &str, query: &str, variables: Value) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .post("https://backboard.railway.app/graphql/v2")
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .json(&json!({"query": query, "variables": variables}))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;
    if let Some(errs) = resp.get("errors") {
        return Err(errs.to_string());
    }
    Ok(resp["data"].clone())
}

// ─── Formatters ──────────────────────────────────────────────────────────────

fn fmt_deploy_status(status: &str) -> &str {
    match status {
        "SUCCESS"   => "✅ SUCCESS",
        "FAILED"    => "❌ FAILED",
        "CRASHED"   => "❌ CRASHED",
        "DEPLOYING" => "⏳ DEPLOYING",
        "BUILDING"  => "⏳ BUILDING",
        "REMOVED"   => "🗑 REMOVED",
        "REMOVING"  => "🗑 REMOVING",
        "SLEEPING"  => "💤 SLEEPING",
        other       => other,
    }
}

fn fmt_ts(s: &str) -> String {
    // Railway returns ISO 8601 timestamps
    if s.is_empty() || s == "?" { return "—".to_string(); }
    match chrono::DateTime::parse_from_rfc3339(s) {
        Ok(dt) => dt.format("%Y-%m-%d %H:%M UTC").to_string(),
        Err(_) => s.to_string(),
    }
}

fn fmt_ts_val(v: &Value) -> String {
    fmt_ts(v.as_str().unwrap_or(""))
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"railway_list_projects","description":"RAILWAY 🚂 — Liste tous les projets du compte avec nom, id, description et date de création.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"railway_get_project","description":"RAILWAY 🚂 — Récupère le détail d'un projet : services et environnements disponibles.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID du projet Railway"}},"required":["project_id"]}}),
        json!({"name":"railway_list_services","description":"RAILWAY 🚂 — Liste les services d'un projet avec leur commande de démarrage par environnement.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID du projet Railway"}},"required":["project_id"]}}),
        json!({"name":"railway_list_deployments","description":"RAILWAY 🚂 — Liste les déploiements récents d'un projet (filtrable par service). Retourne statut, image, date.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID du projet Railway"},"service_id":{"type":"string","description":"Filtrer par service (optionnel)"}},"required":["project_id"]}}),
        json!({"name":"railway_get_logs","description":"RAILWAY 🚂 — Récupère les logs d'un déploiement (timestamp + message).","inputSchema":{"type":"object","properties":{"deployment_id":{"type":"string","description":"ID du déploiement"}},"required":["deployment_id"]}}),
        json!({"name":"railway_get_variables","description":"RAILWAY 🚂 — Liste les NOMS des variables d'environnement d'un service (jamais les valeurs — sécurité).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"environment_id":{"type":"string"},"service_id":{"type":"string"}},"required":["project_id","environment_id","service_id"]}}),
        json!({"name":"railway_trigger_deploy","description":"RAILWAY 🚂 — Déclenche un redéploiement d'un service dans un environnement.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service"},"environment_id":{"type":"string","description":"ID de l'environnement"}},"required":["service_id","environment_id"]}}),
        json!({"name":"railway_list_environments","description":"RAILWAY 🚂 — Liste les environnements d'un projet (ex: production, staging).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID du projet"}},"required":["project_id"]}}),
        json!({"name":"railway_get_service","description":"RAILWAY 🚂 — Récupère le détail d'un service : id, nom, templateServiceId et templateThreadSlug.","inputSchema":{"type":"object","properties":{"service_id":{"type":"string","description":"ID du service Railway"}},"required":["service_id"]}}),
        json!({"name":"railway_build_logs","description":"RAILWAY 🚂 — Récupère les logs de build d'un déploiement avec timestamp, message et sévérité. Format : [severity] [timestamp] message.","inputSchema":{"type":"object","properties":{"deployment_id":{"type":"string","description":"ID du déploiement"},"limit":{"type":"integer","description":"Nombre maximum de lignes (défaut 50)"}},"required":["deployment_id"]}}),
        json!({"name":"railway_restart_deployment","description":"RAILWAY 🚂 — Redémarre un déploiement existant via son ID.","inputSchema":{"type":"object","properties":{"deployment_id":{"type":"string","description":"ID du déploiement à redémarrer"}},"required":["deployment_id"]}}),
        json!({"name":"railway_create_project","description":"RAILWAY 🚂 — Crée un nouveau projet Railway avec un nom et une description optionnelle. Retourne l'id et le nom du projet créé.","inputSchema":{"type":"object","properties":{"name":{"type":"string","description":"Nom du projet"},"description":{"type":"string","description":"Description du projet (optionnel)"}},"required":["name"]}}),
        json!({"name":"railway_delete_project","description":"RAILWAY 🚂 — Supprime définitivement un projet Railway par son ID. Action irréversible.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID du projet à supprimer"}},"required":["project_id"]}}),
        json!({"name":"railway_get_usage","description":"RAILWAY 🚂 — Récupère l'utilisation et les coûts estimés d'un projet pour une période donnée (CPU, mémoire, coût en dollars). Par défaut : 30 derniers jours.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID du projet"},"environment_id":{"type":"string","description":"ID de l'environnement"},"start_date":{"type":"string","description":"Date de début YYYY-MM-DD (défaut : -30 jours)"},"end_date":{"type":"string","description":"Date de fin YYYY-MM-DD (défaut : aujourd'hui)"}},"required":["project_id","environment_id"]}}),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = match RailwayConfig::load() {
        Some(c) => c,
        None    => return Ok("Railway non configuré (railway.toml manquant)".to_string()),
    };

    match name {
        "railway_list_projects" => {
            let q = "query { me { projects { edges { node { id name description createdAt } } } } }";
            let data     = gql(&cfg.token, q, json!({})).await?;
            let edges    = data["me"]["projects"]["edges"].as_array().cloned().unwrap_or_default();
            if edges.is_empty() { return Ok("Aucun projet trouvé.".to_string()); }
            Ok(edges.iter().map(|e| {
                let n     = &e["node"];
                let name  = n["name"].as_str().unwrap_or("?");
                let id    = n["id"].as_str().unwrap_or("?");
                let desc  = n["description"].as_str().unwrap_or("—");
                let ts    = fmt_ts_val(&n["createdAt"]);
                format!("🚂 {name}\n  id={id} · créé={ts}\n  {desc}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "railway_get_project" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let q = "query($id: String!) { project(id: $id) { id name description \
                services { edges { node { id name } } } \
                environments { edges { node { id name } } } \
            } }";
            let data    = gql(&cfg.token, q, json!({"id": project_id})).await?;
            let project = &data["project"];
            let name    = project["name"].as_str().unwrap_or("?");
            let desc    = project["description"].as_str().unwrap_or("—");
            let services = project["services"]["edges"].as_array().cloned().unwrap_or_default();
            let envs     = project["environments"]["edges"].as_array().cloned().unwrap_or_default();
            let svc_list = services.iter().map(|e| {
                let n = &e["node"];
                format!("  • {} (id={})", n["name"].as_str().unwrap_or("?"), n["id"].as_str().unwrap_or("?"))
            }).collect::<Vec<_>>().join("\n");
            let env_list = envs.iter().map(|e| {
                let n = &e["node"];
                format!("  • {} (id={})", n["name"].as_str().unwrap_or("?"), n["id"].as_str().unwrap_or("?"))
            }).collect::<Vec<_>>().join("\n");
            Ok(format!(
                "🚂 {name}\n  id={project_id}\n  {desc}\n\nServices ({}):\n{}\n\nEnvironnements ({}):\n{}",
                services.len(), svc_list, envs.len(), env_list
            ))
        }

        "railway_list_services" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let q = "query($projectId: String!) { project(id: $projectId) { \
                services { edges { node { id name \
                    serviceInstances { edges { node { serviceId environmentId startCommand } } } \
                } } } \
            } }";
            let data     = gql(&cfg.token, q, json!({"projectId": project_id})).await?;
            let edges    = data["project"]["services"]["edges"].as_array().cloned().unwrap_or_default();
            if edges.is_empty() { return Ok("Aucun service trouvé.".to_string()); }
            Ok(edges.iter().map(|e| {
                let n    = &e["node"];
                let name = n["name"].as_str().unwrap_or("?");
                let id   = n["id"].as_str().unwrap_or("?");
                let instances = n["serviceInstances"]["edges"].as_array().cloned().unwrap_or_default();
                let inst_str = instances.iter().map(|ie| {
                    let inst    = &ie["node"];
                    let env_id  = inst["environmentId"].as_str().unwrap_or("?");
                    let cmd     = inst["startCommand"].as_str().unwrap_or("—");
                    format!("    env={env_id} · cmd={cmd}")
                }).collect::<Vec<_>>().join("\n");
                if inst_str.is_empty() {
                    format!("🚂 {name}\n  id={id}")
                } else {
                    format!("🚂 {name}\n  id={id}\n{inst_str}")
                }
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "railway_list_deployments" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let service_id = args["service_id"].as_str().map(String::from);
            let q = "query($projectId: String!, $serviceId: String) { \
                deployments(projectId: $projectId, serviceId: $serviceId, first: 10) { \
                    edges { node { id status createdAt meta { image } } } \
                } \
            }";
            let vars = json!({"projectId": project_id, "serviceId": service_id});
            let data  = gql(&cfg.token, q, vars).await?;
            let edges = data["deployments"]["edges"].as_array().cloned().unwrap_or_default();
            if edges.is_empty() { return Ok("Aucun déploiement trouvé.".to_string()); }
            Ok(edges.iter().map(|e| {
                let n      = &e["node"];
                let id     = n["id"].as_str().unwrap_or("?");
                let status = n["status"].as_str().unwrap_or("?");
                let ts     = fmt_ts_val(&n["createdAt"]);
                let image  = n["meta"]["image"].as_str().unwrap_or("—");
                format!("{}\n  id={} · {} · image={}", fmt_deploy_status(status), id, ts, image)
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "railway_get_logs" => {
            let deployment_id = args["deployment_id"].as_str().ok_or("Missing param: deployment_id")?.to_string();
            let q = "query($deploymentId: String!) { \
                deploymentLogs(deploymentId: $deploymentId) { timestamp message } \
            }";
            let data = gql(&cfg.token, q, json!({"deploymentId": deployment_id})).await?;
            let logs = data["deploymentLogs"].as_array().cloned().unwrap_or_default();
            if logs.is_empty() { return Ok("Aucun log disponible.".to_string()); }
            Ok(logs.iter().map(|l| {
                let ts  = fmt_ts(l["timestamp"].as_str().unwrap_or(""));
                let msg = l["message"].as_str().unwrap_or("");
                format!("[{ts}] {msg}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "railway_get_variables" => {
            let project_id     = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let environment_id = args["environment_id"].as_str().ok_or("Missing param: environment_id")?.to_string();
            let service_id     = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let q = "query($projectId: String!, $environmentId: String!, $serviceId: String!) { \
                variables(projectId: $projectId, environmentId: $environmentId, serviceId: $serviceId) \
            }";
            let vars = json!({
                "projectId":     project_id,
                "environmentId": environment_id,
                "serviceId":     service_id,
            });
            let data = gql(&cfg.token, q, vars).await?;
            // variables is an object: { KEY: "VALUE", ... } — return NAMES only
            match data["variables"].as_object() {
                Some(map) if !map.is_empty() => {
                    let keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
                    Ok(format!(
                        "Variables ({}) — noms uniquement (valeurs masquées) :\n{}",
                        keys.len(),
                        keys.join("\n")
                    ))
                }
                _ => Ok("Aucune variable trouvée.".to_string()),
            }
        }

        "railway_trigger_deploy" => {
            let service_id     = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let environment_id = args["environment_id"].as_str().ok_or("Missing param: environment_id")?.to_string();
            let q = "mutation($serviceId: String!, $environmentId: String!) { \
                serviceInstanceRedeploy(serviceId: $serviceId, environmentId: $environmentId) \
            }";
            let vars = json!({"serviceId": service_id, "environmentId": environment_id});
            gql(&cfg.token, q, vars).await?;
            Ok(format!("✅ Redéploiement déclenché pour le service {service_id} (env={environment_id})."))
        }

        "railway_list_environments" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let q = "query($projectId: String!) { \
                project(id: $projectId) { \
                    environments { edges { node { id name } } } \
                } \
            }";
            let data  = gql(&cfg.token, q, json!({"projectId": project_id})).await?;
            let edges = data["project"]["environments"]["edges"].as_array().cloned().unwrap_or_default();
            if edges.is_empty() { return Ok("Aucun environnement trouvé.".to_string()); }
            Ok(edges.iter().map(|e| {
                let n    = &e["node"];
                let name = n["name"].as_str().unwrap_or("?");
                let id   = n["id"].as_str().unwrap_or("?");
                format!("🚂 {name}  id={id}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "railway_get_service" => {
            let service_id = args["service_id"].as_str().ok_or("Missing param: service_id")?.to_string();
            let q = "query($id: String!) { \
                service(id: $id) { id name templateServiceId templateThreadSlug } \
            }";
            let data    = gql(&cfg.token, q, json!({"id": service_id})).await?;
            let service = &data["service"];
            let name    = service["name"].as_str().unwrap_or("?");
            let id      = service["id"].as_str().unwrap_or("?");
            let tpl_id  = service["templateServiceId"].as_str().unwrap_or("—");
            let tpl_slug = service["templateThreadSlug"].as_str().unwrap_or("—");
            Ok(format!(
                "🚂 {name}\n  id={id}\n  templateServiceId={tpl_id}\n  templateThreadSlug={tpl_slug}"
            ))
        }

        "railway_build_logs" => {
            let deployment_id = args["deployment_id"].as_str().ok_or("Missing param: deployment_id")?.to_string();
            let limit = args["limit"].as_i64().unwrap_or(50);
            let q = "query($deploymentId: String!, $limit: Int) { \
                buildLogs(deploymentId: $deploymentId, limit: $limit) { timestamp message severity } \
            }";
            let data = gql(&cfg.token, q, json!({"deploymentId": deployment_id, "limit": limit})).await?;
            let logs = data["buildLogs"].as_array().cloned().unwrap_or_default();
            if logs.is_empty() { return Ok("Aucun log de build disponible.".to_string()); }
            Ok(logs.iter().map(|l| {
                let ts       = fmt_ts(l["timestamp"].as_str().unwrap_or(""));
                let msg      = l["message"].as_str().unwrap_or("");
                let severity = l["severity"].as_str().unwrap_or("INFO");
                format!("[{severity}] [{ts}] {msg}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "railway_restart_deployment" => {
            let deployment_id = args["deployment_id"].as_str().ok_or("Missing param: deployment_id")?.to_string();
            let q = "mutation($id: String!) { deploymentRestart(id: $id) }";
            gql(&cfg.token, q, json!({"id": deployment_id})).await?;
            Ok(format!("✅ Déploiement {deployment_id} redémarré avec succès."))
        }

        "railway_create_project" => {
            let name        = args["name"].as_str().ok_or("Missing param: name")?.to_string();
            let description = args["description"].as_str().map(String::from);
            let q = "mutation($name: String!, $description: String) { \
                projectCreate(input: { name: $name, description: $description }) { id name } \
            }";
            let vars = json!({"name": name, "description": description});
            let data    = gql(&cfg.token, q, vars).await?;
            let project = &data["projectCreate"];
            let id      = project["id"].as_str().unwrap_or("?");
            let pname   = project["name"].as_str().unwrap_or("?");
            Ok(format!("✅ Projet créé : {pname}\n  id={id}"))
        }

        "railway_delete_project" => {
            let project_id = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let q = "mutation($id: String!) { projectDelete(id: $id) }";
            gql(&cfg.token, q, json!({"id": project_id})).await?;
            Ok(format!("✅ Projet {project_id} supprimé définitivement."))
        }

        "railway_get_usage" => {
            let project_id     = args["project_id"].as_str().ok_or("Missing param: project_id")?.to_string();
            let environment_id = args["environment_id"].as_str().ok_or("Missing param: environment_id")?.to_string();
            let now            = chrono::Utc::now();
            let end_date = args["end_date"].as_str()
                .map(String::from)
                .unwrap_or_else(|| now.format("%Y-%m-%d").to_string());
            let start_date = args["start_date"].as_str()
                .map(String::from)
                .unwrap_or_else(|| (now - chrono::Duration::days(30)).format("%Y-%m-%d").to_string());
            let q = "query($projectId: String!, $environmentId: String!, $startDate: String!, $endDate: String!) { \
                usageForProject(projectId: $projectId, environmentId: $environmentId, startDate: $startDate, endDate: $endDate) { \
                    service { serviceName } \
                    estimated { dollars } \
                    cpu { cpuSeconds } \
                    memory { usedGB } \
                } \
            }";
            let vars = json!({
                "projectId":     project_id,
                "environmentId": environment_id,
                "startDate":     start_date,
                "endDate":       end_date,
            });
            let data  = gql(&cfg.token, q, vars).await?;
            let items = data["usageForProject"].as_array().cloned().unwrap_or_default();
            if items.is_empty() {
                return Ok(format!("Aucune donnée d'utilisation pour la période {start_date} → {end_date}."));
            }
            let mut lines = vec![
                format!("📊 Utilisation du projet ({start_date} → {end_date}):\n"),
            ];
            for item in &items {
                let svc_name   = item["service"]["serviceName"].as_str().unwrap_or("?");
                let dollars    = item["estimated"]["dollars"].as_f64().unwrap_or(0.0);
                let cpu_secs   = item["cpu"]["cpuSeconds"].as_f64().unwrap_or(0.0);
                let memory_gb  = item["memory"]["usedGB"].as_f64().unwrap_or(0.0);
                lines.push(format!(
                    "🚂 {svc_name}\n  Coût estimé : ${dollars:.4}\n  CPU : {cpu_secs:.2} s\n  Mémoire : {memory_gb:.4} GB"
                ));
            }
            Ok(lines.join("\n\n"))
        }

        other => Err(format!("Unknown railway tool: {other}")),
    }
}
