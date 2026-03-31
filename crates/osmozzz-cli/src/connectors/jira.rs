/// Connecteur Jira — REST API v3 officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct JiraConfig { base_url: String, email: String, token: String }

impl JiraConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/jira.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
    fn api(&self, path: &str) -> String {
        format!("{}/rest/api/3/{}", self.base_url.trim_end_matches('/'), path)
    }
    fn agile(&self, path: &str) -> String {
        format!("{}/rest/agile/1.0/{}", self.base_url.trim_end_matches('/'), path)
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &JiraConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new().get(url)
        .basic_auth(&cfg.email, Some(&cfg.token))
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post(cfg: &JiraConfig, url: &str, body: Value) -> Result<Value, String> {
    reqwest::Client::new().post(url)
        .basic_auth(&cfg.email, Some(&cfg.token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn put(cfg: &JiraConfig, url: &str, body: Value) -> Result<Value, String> {
    reqwest::Client::new().put(url)
        .basic_auth(&cfg.email, Some(&cfg.token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

// ─── ADF helpers ─────────────────────────────────────────────────────────────

fn adf(text: &str) -> Value {
    json!({"type":"doc","version":1,"content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]})
}

fn extract_text(v: &Value) -> String {
    match v {
        Value::Object(obj) => {
            if let Some(Value::String(s)) = obj.get("text") { return s.clone(); }
            if let Some(content) = obj.get("content") { return extract_text(content); }
            String::new()
        }
        Value::Array(arr) => arr.iter().map(extract_text).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }
}

// ─── Formatters ──────────────────────────────────────────────────────────────

fn fmt_issues(issues: &[Value]) -> String {
    if issues.is_empty() { return "Aucune issue trouvée.".to_string(); }
    issues.iter().map(|i| {
        let key    = i["key"].as_str().unwrap_or("?");
        let title  = i["fields"]["summary"].as_str().unwrap_or("?");
        let status = i["fields"]["status"]["name"].as_str().unwrap_or("?");
        let asgn   = i["fields"]["assignee"]["displayName"].as_str().unwrap_or("—");
        let prio   = i["fields"]["priority"]["name"].as_str().unwrap_or("?");
        format!("[{key}] {title}\n  état={status} · priorité={prio} · assigné={asgn}")
    }).collect::<Vec<_>>().join("\n\n")
}

fn fmt_issue(i: &Value) -> String {
    let key    = i["key"].as_str().unwrap_or("?");
    let f      = &i["fields"];
    let title  = f["summary"].as_str().unwrap_or("?");
    let status = f["status"]["name"].as_str().unwrap_or("?");
    let asgn   = f["assignee"]["displayName"].as_str().unwrap_or("—");
    let prio   = f["priority"]["name"].as_str().unwrap_or("?");
    let itype  = f["issuetype"]["name"].as_str().unwrap_or("?");
    let desc   = extract_text(&f["description"]);
    format!("[{key}] {title}\nType: {itype} · État: {status} · Priorité: {prio} · Assigné: {asgn}\n\n{desc}")
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"jira_search_issues","description":"JIRA — Recherche des issues via JQL (Jira Query Language). Ex: 'project = ENG AND status = \"In Progress\"'. Retourne liste compacte avec clés (ex: ENG-42). Enchaîner avec jira_get_issue pour le détail.","inputSchema":{"type":"object","properties":{"jql":{"type":"string","description":"Requête JQL (ex: project=ENG AND assignee=currentUser())"},"limit":{"type":"integer","default":20,"maximum":50}},"required":["jql"]}}),
        json!({"name":"jira_get_issue","description":"JIRA — Récupère le détail complet d'une issue par sa clé (ex: ENG-42).","inputSchema":{"type":"object","properties":{"key":{"type":"string","description":"Clé Jira (ex: ENG-42, PROJ-1)"}},"required":["key"]}}),
        json!({"name":"jira_create_issue","description":"JIRA — Crée une nouvelle issue. Utiliser jira_list_projects pour le projectKey et jira_get_issue_types pour le type.","inputSchema":{"type":"object","properties":{"project_key":{"type":"string","description":"Clé du projet (ex: ENG)"},"summary":{"type":"string"},"issue_type":{"type":"string","description":"Type d'issue (ex: Bug, Task, Story)","default":"Task"},"description":{"type":"string"},"assignee_account_id":{"type":"string"},"priority":{"type":"string","description":"Ex: High, Medium, Low"}},"required":["project_key","summary"]}}),
        json!({"name":"jira_update_issue","description":"JIRA — Met à jour les champs d'une issue existante.","inputSchema":{"type":"object","properties":{"key":{"type":"string"},"summary":{"type":"string"},"description":{"type":"string"},"priority":{"type":"string"},"assignee_account_id":{"type":"string"}},"required":["key"]}}),
        json!({"name":"jira_add_comment","description":"JIRA — Ajoute un commentaire à une issue.","inputSchema":{"type":"object","properties":{"key":{"type":"string","description":"Clé Jira (ex: ENG-42)"},"body":{"type":"string"}},"required":["key","body"]}}),
        json!({"name":"jira_get_comments","description":"JIRA — Récupère les commentaires d'une issue.","inputSchema":{"type":"object","properties":{"key":{"type":"string"}},"required":["key"]}}),
        json!({"name":"jira_transition_issue","description":"JIRA — Change le statut d'une issue (ex: passer en 'Done'). Utiliser jira_list_transitions pour obtenir les transition_ids disponibles.","inputSchema":{"type":"object","properties":{"key":{"type":"string"},"transition_id":{"type":"string","description":"ID de la transition (obtenu via jira_list_transitions)"}},"required":["key","transition_id"]}}),
        json!({"name":"jira_list_transitions","description":"JIRA — Liste les transitions (changements de statut) disponibles pour une issue.","inputSchema":{"type":"object","properties":{"key":{"type":"string"}},"required":["key"]}}),
        json!({"name":"jira_assign_issue","description":"JIRA — Assigne une issue à un utilisateur. Utiliser jira_search_users pour obtenir l'accountId.","inputSchema":{"type":"object","properties":{"key":{"type":"string"},"account_id":{"type":"string","description":"accountId de l'utilisateur (obtenu via jira_search_users)"}},"required":["key","account_id"]}}),
        json!({"name":"jira_list_projects","description":"JIRA — Liste tous les projets Jira accessibles avec leurs clés.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"jira_get_issue_types","description":"JIRA — Liste les types d'issues disponibles pour un projet (Bug, Task, Story, Epic…).","inputSchema":{"type":"object","properties":{"project_key":{"type":"string"}},"required":["project_key"]}}),
        json!({"name":"jira_list_priorities","description":"JIRA — Liste les priorités disponibles (Highest, High, Medium, Low, Lowest).","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"jira_search_users","description":"JIRA — Recherche des utilisateurs Jira par nom ou email. Retourne accountId nécessaire pour les assignations.","inputSchema":{"type":"object","properties":{"query":{"type":"string","description":"Nom ou email à chercher"}},"required":["query"]}}),
        json!({"name":"jira_add_worklog","description":"JIRA — Enregistre du temps passé sur une issue (time tracking).","inputSchema":{"type":"object","properties":{"key":{"type":"string"},"time_spent":{"type":"string","description":"Temps au format Jira (ex: 1h 30m, 2h, 30m)"},"comment":{"type":"string"}},"required":["key","time_spent"]}}),
        json!({"name":"jira_list_boards","description":"JIRA — Liste les tableaux agile (Scrum/Kanban) disponibles.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"jira_list_sprints","description":"JIRA — Liste les sprints d'un tableau agile.","inputSchema":{"type":"object","properties":{"board_id":{"type":"integer","description":"ID du tableau (obtenu via jira_list_boards)"}},"required":["board_id"]}}),
        json!({"name":"jira_delete_issue","description":"JIRA — Supprime définitivement une issue Jira.","inputSchema":{"type":"object","properties":{"key":{"type":"string","description":"Clé Jira (ex: ENG-42)"}},"required":["key"]}}),
        json!({"name":"jira_link_issues","description":"JIRA — Crée un lien entre deux issues (ex: 'blocks', 'is blocked by', 'relates to', 'duplicates'). Utiliser jira_list_link_types pour les types disponibles.","inputSchema":{"type":"object","properties":{"inward_key":{"type":"string","description":"Clé de l'issue source (ex: ENG-42)"},"outward_key":{"type":"string","description":"Clé de l'issue cible (ex: ENG-99)"},"link_type":{"type":"string","description":"Type de lien (ex: Blocks, Relates, Duplicate)"}},"required":["inward_key","outward_key","link_type"]}}),
        json!({"name":"jira_list_link_types","description":"JIRA — Liste les types de liens disponibles entre issues (ex: Blocks, Relates to, Duplicates).","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"jira_get_current_user","description":"JIRA — Retourne les informations de l'utilisateur authentifié (displayName, email, accountId). Utile pour filtrer les issues assignées à soi-même.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"jira_list_versions","description":"JIRA — Liste les versions/releases d'un projet Jira.","inputSchema":{"type":"object","properties":{"project_key":{"type":"string","description":"Clé du projet (ex: ENG)"}},"required":["project_key"]}}),
        json!({"name":"jira_move_to_sprint","description":"JIRA — Déplace une ou plusieurs issues dans un sprint. Utiliser jira_list_sprints pour obtenir le sprint_id.","inputSchema":{"type":"object","properties":{"sprint_id":{"type":"integer","description":"ID du sprint (obtenu via jira_list_sprints)"},"keys":{"type":"array","items":{"type":"string"},"description":"Liste des clés Jira à déplacer (ex: [\"ENG-42\", \"ENG-43\"])"}},"required":["sprint_id","keys"]}}),
        json!({"name":"jira_get_fields","description":"JIRA — Liste tous les champs disponibles dans Jira (natifs + custom). Utile pour connaître les IDs des champs custom avant de créer/mettre à jour une issue.","inputSchema":{"type":"object","properties":{}}}),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = match JiraConfig::load() {
        Some(c) => c,
        None => return Ok("Jira non configuré (jira.toml manquant)".to_string()),
    };

    match name {
        "jira_search_issues" => {
            let jql   = args["jql"].as_str().ok_or("Missing param: jql")?.to_string();
            let limit = args["limit"].as_u64().unwrap_or(20);
            let url   = cfg.api(&format!("search?jql={}&maxResults={}&fields=summary,status,assignee,priority,issuetype", urlencoding::encode(&jql), limit));
            let data  = get(&cfg, &url).await?;
            let issues = data["issues"].as_array().cloned().unwrap_or_default();
            Ok(fmt_issues(&issues))
        }

        "jira_get_issue" => {
            let key = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let url = cfg.api(&format!("issue/{}?fields=summary,status,assignee,priority,issuetype,description,comment", key));
            let data = get(&cfg, &url).await?;
            Ok(fmt_issue(&data))
        }

        "jira_create_issue" => {
            let project_key = args["project_key"].as_str().ok_or("Missing param: project_key")?.to_string();
            let summary     = args["summary"].as_str().ok_or("Missing param: summary")?.to_string();
            let issue_type  = args["issue_type"].as_str().unwrap_or("Task");
            let mut fields = json!({
                "project":   {"key": project_key},
                "summary":   summary,
                "issuetype": {"name": issue_type}
            });
            if let Some(d) = args["description"].as_str()         { fields["description"] = adf(d); }
            if let Some(a) = args["assignee_account_id"].as_str() { fields["assignee"] = json!({"accountId": a}); }
            if let Some(p) = args["priority"].as_str()            { fields["priority"] = json!({"name": p}); }
            let url  = cfg.api("issue");
            let data = post(&cfg, &url, json!({"fields": fields})).await?;
            let key  = data["key"].as_str().unwrap_or("?");
            let self_url = data["self"].as_str().unwrap_or("");
            Ok(format!("✅ Issue créée : {key}\n{self_url}"))
        }

        "jira_update_issue" => {
            let key = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let mut fields = json!({});
            if let Some(v) = args["summary"].as_str()             { fields["summary"]     = json!(v); }
            if let Some(v) = args["description"].as_str()         { fields["description"] = adf(v); }
            if let Some(v) = args["priority"].as_str()            { fields["priority"]    = json!({"name":v}); }
            if let Some(v) = args["assignee_account_id"].as_str() { fields["assignee"]    = json!({"accountId":v}); }
            let url = cfg.api(&format!("issue/{}", key));
            put(&cfg, &url, json!({"fields": fields})).await?;
            Ok(format!("✅ {key} mis à jour."))
        }

        "jira_add_comment" => {
            let key  = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let body = args["body"].as_str().ok_or("Missing param: body")?.to_string();
            let url  = cfg.api(&format!("issue/{}/comment", key));
            post(&cfg, &url, json!({"body": adf(&body)})).await?;
            Ok("✅ Commentaire ajouté.".to_string())
        }

        "jira_get_comments" => {
            let key      = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let url      = cfg.api(&format!("issue/{}/comment", key));
            let data     = get(&cfg, &url).await?;
            let comments = data["comments"].as_array().cloned().unwrap_or_default();
            if comments.is_empty() { return Ok("Aucun commentaire.".to_string()); }
            Ok(comments.iter().map(|c| {
                let author = c["author"]["displayName"].as_str().unwrap_or("?");
                let date   = c["created"].as_str().unwrap_or("?");
                let body   = extract_text(&c["body"]);
                format!("[{date}] {author}:\n{body}")
            }).collect::<Vec<_>>().join("\n\n---\n\n"))
        }

        "jira_transition_issue" => {
            let key           = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let transition_id = args["transition_id"].as_str().ok_or("Missing param: transition_id")?.to_string();
            let url           = cfg.api(&format!("issue/{}/transitions", key));
            post(&cfg, &url, json!({"transition":{"id":transition_id}})).await?;
            Ok(format!("✅ {key} transition effectuée."))
        }

        "jira_list_transitions" => {
            let key         = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let url         = cfg.api(&format!("issue/{}/transitions", key));
            let data        = get(&cfg, &url).await?;
            let transitions = data["transitions"].as_array().cloned().unwrap_or_default();
            Ok(transitions.iter().map(|t| {
                let name = t["name"].as_str().unwrap_or("?");
                let tid  = t["id"].as_str().unwrap_or("?");
                format!("{name}  id={tid}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "jira_assign_issue" => {
            let key        = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let account_id = args["account_id"].as_str().ok_or("Missing param: account_id")?.to_string();
            let url        = cfg.api(&format!("issue/{}/assignee", key));
            put(&cfg, &url, json!({"accountId": account_id})).await?;
            Ok(format!("✅ {key} assigné."))
        }

        "jira_list_projects" => {
            let url      = cfg.api("project?maxResults=50");
            let data     = get(&cfg, &url).await?;
            let projects = data.as_array().cloned().unwrap_or_default();
            if projects.is_empty() { return Ok("Aucun projet.".to_string()); }
            Ok(projects.iter().map(|p| {
                let name = p["name"].as_str().unwrap_or("?");
                let key  = p["key"].as_str().unwrap_or("?");
                format!("[{key}] {name}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "jira_get_issue_types" => {
            let project_key = args["project_key"].as_str().ok_or("Missing param: project_key")?.to_string();
            let url   = cfg.api(&format!("project/{}/statuses", project_key));
            let data  = get(&cfg, &url).await?;
            let types = data.as_array().cloned().unwrap_or_default();
            Ok(types.iter().map(|t| t["name"].as_str().unwrap_or("?")).collect::<Vec<_>>().join(", "))
        }

        "jira_list_priorities" => {
            let url   = cfg.api("priority");
            let data  = get(&cfg, &url).await?;
            let prios = data.as_array().cloned().unwrap_or_default();
            Ok(prios.iter().map(|p| p["name"].as_str().unwrap_or("?")).collect::<Vec<_>>().join(", "))
        }

        "jira_search_users" => {
            let query = args["query"].as_str().ok_or("Missing param: query")?.to_string();
            let url   = cfg.api(&format!("user/search?query={}&maxResults=20", urlencoding::encode(&query)));
            let data  = get(&cfg, &url).await?;
            let users = data.as_array().cloned().unwrap_or_default();
            if users.is_empty() { return Ok("Aucun utilisateur trouvé.".to_string()); }
            Ok(users.iter().map(|u| {
                let name    = u["displayName"].as_str().unwrap_or("?");
                let email   = u["emailAddress"].as_str().unwrap_or("?");
                let account = u["accountId"].as_str().unwrap_or("?");
                format!("{name} <{email}>\n  accountId={account}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "jira_add_worklog" => {
            let key        = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let time_spent = args["time_spent"].as_str().ok_or("Missing param: time_spent")?.to_string();
            let mut body   = json!({"timeSpent": time_spent});
            if let Some(c) = args["comment"].as_str() { body["comment"] = adf(c); }
            let url = cfg.api(&format!("issue/{}/worklog", key));
            post(&cfg, &url, body).await?;
            Ok(format!("✅ {time_spent} enregistré sur {key}."))
        }

        "jira_list_boards" => {
            let url    = cfg.agile("board?maxResults=50");
            let data   = get(&cfg, &url).await?;
            let boards = data["values"].as_array().cloned().unwrap_or_default();
            if boards.is_empty() { return Ok("Aucun tableau.".to_string()); }
            Ok(boards.iter().map(|b| {
                let name = b["name"].as_str().unwrap_or("?");
                let typ  = b["type"].as_str().unwrap_or("?");
                let bid  = b["id"].as_u64().unwrap_or(0);
                format!("[{bid}] {name} ({typ})")
            }).collect::<Vec<_>>().join("\n"))
        }

        "jira_list_sprints" => {
            let board_id = args["board_id"].as_u64().ok_or("Missing param: board_id")?;
            let url     = cfg.agile(&format!("board/{}/sprint?maxResults=20", board_id));
            let data    = get(&cfg, &url).await?;
            let sprints = data["values"].as_array().cloned().unwrap_or_default();
            if sprints.is_empty() { return Ok("Aucun sprint.".to_string()); }
            Ok(sprints.iter().map(|s| {
                let name  = s["name"].as_str().unwrap_or("?");
                let state = s["state"].as_str().unwrap_or("?");
                let sid   = s["id"].as_u64().unwrap_or(0);
                format!("[{sid}] {name} — {state}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "jira_delete_issue" => {
            let key = args["key"].as_str().ok_or("Missing param: key")?.to_string();
            let url = cfg.api(&format!("issue/{}", key));
            reqwest::Client::new().delete(&url)
                .basic_auth(&cfg.email, Some(&cfg.token))
                .send().await.map_err(|e| e.to_string())?;
            Ok(format!("✅ {key} supprimée."))
        }

        "jira_link_issues" => {
            let inward_key  = args["inward_key"].as_str().ok_or("Missing param: inward_key")?.to_string();
            let outward_key = args["outward_key"].as_str().ok_or("Missing param: outward_key")?.to_string();
            let link_type   = args["link_type"].as_str().ok_or("Missing param: link_type")?.to_string();
            let url  = cfg.api("issueLink");
            let body = json!({
                "type":         {"name": link_type},
                "inwardIssue":  {"key": inward_key},
                "outwardIssue": {"key": outward_key}
            });
            post(&cfg, &url, body).await?;
            Ok(format!("✅ Lien créé : {inward_key} ←[{link_type}]→ {outward_key}"))
        }

        "jira_list_link_types" => {
            let url   = cfg.api("issueLinkType");
            let data  = get(&cfg, &url).await?;
            let types = data["issueLinkTypes"].as_array().cloned().unwrap_or_default();
            if types.is_empty() { return Ok("Aucun type de lien.".to_string()); }
            Ok(types.iter().map(|t| {
                let name    = t["name"].as_str().unwrap_or("?");
                let inward  = t["inward"].as_str().unwrap_or("?");
                let outward = t["outward"].as_str().unwrap_or("?");
                format!("{name}\n  ← {inward} / → {outward}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "jira_get_current_user" => {
            let url  = cfg.api("myself");
            let data = get(&cfg, &url).await?;
            let name    = data["displayName"].as_str().unwrap_or("?");
            let email   = data["emailAddress"].as_str().unwrap_or("?");
            let account = data["accountId"].as_str().unwrap_or("?");
            Ok(format!("{name} <{email}>\naccountId={account}"))
        }

        "jira_list_versions" => {
            let project_key = args["project_key"].as_str().ok_or("Missing param: project_key")?.to_string();
            let url      = cfg.api(&format!("project/{}/versions", project_key));
            let data     = get(&cfg, &url).await?;
            let versions = data.as_array().cloned().unwrap_or_default();
            if versions.is_empty() { return Ok("Aucune version.".to_string()); }
            Ok(versions.iter().map(|v| {
                let name     = v["name"].as_str().unwrap_or("?");
                let released = v["released"].as_bool().unwrap_or(false);
                let date     = v["releaseDate"].as_str().unwrap_or("—");
                let vid      = v["id"].as_str().unwrap_or("?");
                let status   = if released { "released" } else { "unreleased" };
                format!("{name} [{status}] — {date}\n  id={vid}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "jira_move_to_sprint" => {
            let sprint_id = args["sprint_id"].as_u64().ok_or("Missing param: sprint_id")?;
            let keys = args["keys"].as_array().ok_or("Missing param: keys")?
                .iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>();
            let url  = cfg.agile(&format!("sprint/{}/issue", sprint_id));
            post(&cfg, &url, json!({"issues": keys})).await?;
            Ok(format!("✅ {} issue(s) déplacée(s) dans le sprint {sprint_id}.", keys.len()))
        }

        "jira_get_fields" => {
            let url    = cfg.api("field");
            let data   = get(&cfg, &url).await?;
            let fields = data.as_array().cloned().unwrap_or_default();
            Ok(fields.iter().map(|f| {
                let name   = f["name"].as_str().unwrap_or("?");
                let id     = f["id"].as_str().unwrap_or("?");
                let custom = f["custom"].as_bool().unwrap_or(false);
                let kind   = if custom { "custom" } else { "system" };
                format!("{name} [{kind}]  id={id}")
            }).collect::<Vec<_>>().join("\n"))
        }

        other => Err(format!("Unknown jira tool: {other}")),
    }
}
