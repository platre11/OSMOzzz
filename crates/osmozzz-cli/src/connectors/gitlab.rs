/// Connecteur GitLab — API REST v4 officielle.
/// Remplace le subprocess npm @zereight/mcp-gitlab (dev solo, CVE patché).
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct GitlabConfig {
    token:    String,
    #[serde(default = "default_base_url")]
    base_url: String,
    #[serde(default)]
    groups:   Vec<String>,
}

fn default_base_url() -> String { "https://gitlab.com".to_string() }

impl GitlabConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/gitlab.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
    fn api(&self, path: &str) -> String {
        format!("{}/api/v4/{}", self.base_url.trim_end_matches('/'), path)
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(token: &str, url: &str) -> Result<Value, String> {
    reqwest::Client::new().get(url)
        .header("PRIVATE-TOKEN", token)
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post(token: &str, url: &str, body: Value) -> Result<Value, String> {
    reqwest::Client::new().post(url)
        .header("PRIVATE-TOKEN", token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn put(token: &str, url: &str, body: Value) -> Result<Value, String> {
    reqwest::Client::new().put(url)
        .header("PRIVATE-TOKEN", token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

// ─── Formatters ──────────────────────────────────────────────────────────────

fn fmt_issues(issues: &[Value]) -> String {
    if issues.is_empty() { return "Aucune issue trouvée.".to_string(); }
    issues.iter().map(|i| {
        let iid    = i["iid"].as_u64().unwrap_or(0);
        let title  = i["title"].as_str().unwrap_or("?");
        let state  = i["state"].as_str().unwrap_or("?");
        let asgn   = i["assignee"]["name"].as_str().unwrap_or("—");
        let labels = i["labels"].as_array()
            .map(|a| a.iter().filter_map(|l| l.as_str()).collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        let id = i["id"].as_u64().unwrap_or(0);
        let mut line = format!("[#{iid}] {title}\n  état={state} · assigné={asgn} · id={id}");
        if !labels.is_empty() { line.push_str(&format!(" · labels={labels}")); }
        line
    }).collect::<Vec<_>>().join("\n\n")
}

fn fmt_issue(i: &Value) -> String {
    let iid    = i["iid"].as_u64().unwrap_or(0);
    let title  = i["title"].as_str().unwrap_or("?");
    let state  = i["state"].as_str().unwrap_or("?");
    let asgn   = i["assignee"]["name"].as_str().unwrap_or("—");
    let desc   = i["description"].as_str().unwrap_or("—");
    let labels = i["labels"].as_array()
        .map(|a| a.iter().filter_map(|l| l.as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();
    let url    = i["web_url"].as_str().unwrap_or("");
    format!("[#{iid}] {title}\nÉtat: {state} · Assigné: {asgn}\nLabels: {labels}\n{url}\n\n{desc}")
}

fn fmt_mrs(mrs: &[Value]) -> String {
    if mrs.is_empty() { return "Aucune merge request.".to_string(); }
    mrs.iter().map(|m| {
        let iid    = m["iid"].as_u64().unwrap_or(0);
        let title  = m["title"].as_str().unwrap_or("?");
        let state  = m["state"].as_str().unwrap_or("?");
        let author = m["author"]["name"].as_str().unwrap_or("?");
        let source = m["source_branch"].as_str().unwrap_or("?");
        let target = m["target_branch"].as_str().unwrap_or("?");
        let id     = m["id"].as_u64().unwrap_or(0);
        format!("[!{iid}] {title}\n  état={state} · auteur={author} · {source}→{target} · id={id}")
    }).collect::<Vec<_>>().join("\n\n")
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // Issues
        json!({"name":"gitlab_list_issues","description":"GITLAB — Liste les issues d'un projet (filtres: state, label, assignee). Retourne liste compacte avec IIDs. Enchaîner avec gitlab_get_issue pour le détail.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou chemin du projet (ex: 42 ou 'group/repo')"},"state":{"type":"string","description":"open, closed, all","default":"open"},"labels":{"type":"string","description":"Labels séparés par virgule (ex: bug,urgent)"},"assignee_id":{"type":"integer","description":"ID de l'assigné"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_issue","description":"GITLAB — Récupère le détail complet d'une issue (description, labels, assigné, URL).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou chemin du projet"},"issue_iid":{"type":"integer","description":"IID de l'issue (numéro affiché dans GitLab, ex: 42)"}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_create_issue","description":"GITLAB — Crée une nouvelle issue dans un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"description":{"type":"string"},"labels":{"type":"string","description":"Labels séparés par virgule"},"assignee_ids":{"type":"array","items":{"type":"integer"}},"milestone_id":{"type":"integer"}},"required":["project_id","title"]}}),
        json!({"name":"gitlab_update_issue","description":"GITLAB — Met à jour une issue (titre, description, état, labels, assigné).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"title":{"type":"string"},"description":{"type":"string"},"state_event":{"type":"string","description":"close ou reopen"},"labels":{"type":"string"},"assignee_ids":{"type":"array","items":{"type":"integer"}}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_close_issue","description":"GITLAB — Ferme une issue.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_add_comment","description":"GITLAB — Ajoute un commentaire (note) à une issue.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","issue_iid","body"]}}),
        json!({"name":"gitlab_get_comments","description":"GITLAB — Récupère les commentaires d'une issue.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_assign_issue","description":"GITLAB — Assigne une issue à un ou plusieurs utilisateurs. Utiliser gitlab_list_members pour obtenir les IDs.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"assignee_ids":{"type":"array","items":{"type":"integer"},"description":"IDs des utilisateurs (obtenu via gitlab_list_members)"}},"required":["project_id","issue_iid","assignee_ids"]}}),
        json!({"name":"gitlab_list_labels","description":"GITLAB — Liste les labels d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        // Merge Requests
        json!({"name":"gitlab_list_mrs","description":"GITLAB — Liste les merge requests d'un projet (filtres: state, source_branch). Retourne liste compacte avec IIDs.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"state":{"type":"string","description":"opened, closed, merged, all","default":"opened"},"source_branch":{"type":"string"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_mr","description":"GITLAB — Récupère le détail complet d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer","description":"IID de la MR (numéro affiché dans GitLab)"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_create_mr","description":"GITLAB — Crée une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"source_branch":{"type":"string"},"target_branch":{"type":"string","default":"main"},"description":{"type":"string"},"assignee_id":{"type":"integer"},"remove_source_branch":{"type":"boolean","default":false}},"required":["project_id","title","source_branch"]}}),
        json!({"name":"gitlab_merge_mr","description":"GITLAB — Fusionne une merge request (si les conditions sont remplies).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"merge_commit_message":{"type":"string"},"should_remove_source_branch":{"type":"boolean","default":false}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_add_mr_comment","description":"GITLAB — Ajoute un commentaire à une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","mr_iid","body"]}}),
        // Pipelines
        json!({"name":"gitlab_list_pipelines","description":"GITLAB — Liste les pipelines CI/CD d'un projet (filtres: status, branch).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"status":{"type":"string","description":"running, pending, success, failed, canceled, skipped"},"ref":{"type":"string","description":"Branche ou tag"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_pipeline","description":"GITLAB — Récupère le statut détaillé d'un pipeline (jobs, durée, statut).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"pipeline_id":{"type":"integer"}},"required":["project_id","pipeline_id"]}}),
        json!({"name":"gitlab_retry_pipeline","description":"GITLAB — Relance un pipeline échoué.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"pipeline_id":{"type":"integer"}},"required":["project_id","pipeline_id"]}}),
        json!({"name":"gitlab_cancel_pipeline","description":"GITLAB — Annule un pipeline en cours d'exécution.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"pipeline_id":{"type":"integer"}},"required":["project_id","pipeline_id"]}}),
        // Projet & Utilisateurs
        json!({"name":"gitlab_list_projects","description":"GITLAB — Liste les projets accessibles (les groupes configurés dans gitlab.toml sont utilisés si présents).","inputSchema":{"type":"object","properties":{"search":{"type":"string","description":"Filtrer par nom de projet"},"limit":{"type":"integer","default":20,"maximum":100}}}}),
        json!({"name":"gitlab_get_project","description":"GITLAB — Récupère les détails d'un projet (description, URL, stats).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou chemin du projet (ex: 42 ou 'group/repo')"}},"required":["project_id"]}}),
        json!({"name":"gitlab_list_members","description":"GITLAB — Liste les membres d'un projet avec leurs IDs. Nécessaire pour les assignations.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_current_user","description":"GITLAB — Retourne les informations de l'utilisateur authentifié (nom, email, ID). Utile pour filtrer les issues assignées à soi-même.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"gitlab_list_branches","description":"GITLAB — Liste les branches d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"search":{"type":"string","description":"Filtrer par nom de branche"}},"required":["project_id"]}}),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = match GitlabConfig::load() {
        Some(c) => c,
        None => return Ok("GitLab non configuré (gitlab.toml manquant)".to_string()),
    };
    let token = cfg.token.clone();

    // Encode le project_id pour les URLs (ex: "group/repo" → "group%2Frepo")
    let encode_pid = |s: &str| urlencoding::encode(s).into_owned();

    match name {

        // ── Issues ────────────────────────────────────────────────────────────

        "gitlab_list_issues" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let state = args["state"].as_str().unwrap_or("open");
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/issues?state={}&per_page={}", encode_pid(pid), state, limit));
            if let Some(l) = args["labels"].as_str()     { url.push_str(&format!("&labels={}", urlencoding::encode(l))); }
            if let Some(a) = args["assignee_id"].as_u64() { url.push_str(&format!("&assignee_id={a}")); }
            let data   = get(&token, &url).await?;
            let issues = data.as_array().cloned().unwrap_or_default();
            Ok(fmt_issues(&issues))
        }

        "gitlab_get_issue" => {
            let pid = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            let data = get(&token, &url).await?;
            Ok(fmt_issue(&data))
        }

        "gitlab_create_issue" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let title = args["title"].as_str().ok_or("Missing param: title")?;
            let url   = cfg.api(&format!("projects/{}/issues", encode_pid(pid)));
            let mut body = json!({"title": title});
            if let Some(d) = args["description"].as_str()     { body["description"]  = json!(d); }
            if let Some(l) = args["labels"].as_str()          { body["labels"]        = json!(l); }
            if let Some(a) = args["assignee_ids"].as_array()  { body["assignee_ids"]  = json!(a); }
            if let Some(m) = args["milestone_id"].as_u64()    { body["milestone_id"]  = json!(m); }
            let data = post(&token, &url, body).await?;
            let iid  = data["iid"].as_u64().unwrap_or(0);
            let wurl = data["web_url"].as_str().unwrap_or("");
            Ok(format!("✅ Issue créée : #{iid}\n{wurl}"))
        }

        "gitlab_update_issue" => {
            let pid = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            let mut body = json!({});
            if let Some(v) = args["title"].as_str()            { body["title"]        = json!(v); }
            if let Some(v) = args["description"].as_str()      { body["description"]  = json!(v); }
            if let Some(v) = args["state_event"].as_str()      { body["state_event"]  = json!(v); }
            if let Some(v) = args["labels"].as_str()           { body["labels"]       = json!(v); }
            if let Some(v) = args["assignee_ids"].as_array()   { body["assignee_ids"] = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ Issue #{iid} mise à jour."))
        }

        "gitlab_close_issue" => {
            let pid = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            put(&token, &url, json!({"state_event": "close"})).await?;
            Ok(format!("✅ Issue #{iid} fermée."))
        }

        "gitlab_add_comment" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid  = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let body = args["body"].as_str().ok_or("Missing param: body")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}/notes", encode_pid(pid), iid));
            post(&token, &url, json!({"body": body})).await?;
            Ok("✅ Commentaire ajouté.".to_string())
        }

        "gitlab_get_comments" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid  = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}/notes?sort=asc", encode_pid(pid), iid));
            let data = get(&token, &url).await?;
            let notes = data.as_array().cloned().unwrap_or_default();
            if notes.is_empty() { return Ok("Aucun commentaire.".to_string()); }
            Ok(notes.iter().filter(|n| !n["system"].as_bool().unwrap_or(false)).map(|n| {
                let author = n["author"]["name"].as_str().unwrap_or("?");
                let date   = n["created_at"].as_str().unwrap_or("?");
                let body   = n["body"].as_str().unwrap_or("");
                format!("[{date}] {author}:\n{body}")
            }).collect::<Vec<_>>().join("\n\n---\n\n"))
        }

        "gitlab_assign_issue" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid         = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let assignee_ids = args["assignee_ids"].as_array().ok_or("Missing param: assignee_ids")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            put(&token, &url, json!({"assignee_ids": assignee_ids})).await?;
            Ok(format!("✅ Issue #{iid} assignée."))
        }

        "gitlab_list_labels" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url    = cfg.api(&format!("projects/{}/labels?per_page=100", encode_pid(pid)));
            let data   = get(&token, &url).await?;
            let labels = data.as_array().cloned().unwrap_or_default();
            if labels.is_empty() { return Ok("Aucun label.".to_string()); }
            Ok(labels.iter().map(|l| {
                let name  = l["name"].as_str().unwrap_or("?");
                let color = l["color"].as_str().unwrap_or("?");
                format!("{name}  couleur={color}")
            }).collect::<Vec<_>>().join("\n"))
        }

        // ── Merge Requests ────────────────────────────────────────────────────

        "gitlab_list_mrs" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let state = args["state"].as_str().unwrap_or("opened");
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/merge_requests?state={}&per_page={}", encode_pid(pid), state, limit));
            if let Some(b) = args["source_branch"].as_str() { url.push_str(&format!("&source_branch={}", urlencoding::encode(b))); }
            let data = get(&token, &url).await?;
            let mrs  = data.as_array().cloned().unwrap_or_default();
            Ok(fmt_mrs(&mrs))
        }

        "gitlab_get_mr" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}", encode_pid(pid), mr_iid));
            let m      = get(&token, &url).await?;
            let title  = m["title"].as_str().unwrap_or("?");
            let state  = m["state"].as_str().unwrap_or("?");
            let author = m["author"]["name"].as_str().unwrap_or("?");
            let source = m["source_branch"].as_str().unwrap_or("?");
            let target = m["target_branch"].as_str().unwrap_or("?");
            let desc   = m["description"].as_str().unwrap_or("—");
            let wurl   = m["web_url"].as_str().unwrap_or("");
            Ok(format!("[!{mr_iid}] {title}\nÉtat: {state} · Auteur: {author}\n{source} → {target}\n{wurl}\n\n{desc}"))
        }

        "gitlab_create_mr" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let title  = args["title"].as_str().ok_or("Missing param: title")?;
            let source = args["source_branch"].as_str().ok_or("Missing param: source_branch")?;
            let target = args["target_branch"].as_str().unwrap_or("main");
            let url    = cfg.api(&format!("projects/{}/merge_requests", encode_pid(pid)));
            let mut body = json!({"title": title, "source_branch": source, "target_branch": target});
            if let Some(d) = args["description"].as_str()          { body["description"]          = json!(d); }
            if let Some(a) = args["assignee_id"].as_u64()          { body["assignee_id"]           = json!(a); }
            if let Some(r) = args["remove_source_branch"].as_bool() { body["remove_source_branch"] = json!(r); }
            let data  = post(&token, &url, body).await?;
            let iid   = data["iid"].as_u64().unwrap_or(0);
            let wurl  = data["web_url"].as_str().unwrap_or("");
            Ok(format!("✅ MR créée : !{iid}\n{wurl}"))
        }

        "gitlab_merge_mr" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/merge", encode_pid(pid), mr_iid));
            let mut body = json!({});
            if let Some(m) = args["merge_commit_message"].as_str()       { body["merge_commit_message"]        = json!(m); }
            if let Some(r) = args["should_remove_source_branch"].as_bool() { body["should_remove_source_branch"] = json!(r); }
            post(&token, &url, body).await?;
            Ok(format!("✅ MR !{mr_iid} fusionnée."))
        }

        "gitlab_add_mr_comment" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let body   = args["body"].as_str().ok_or("Missing param: body")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/notes", encode_pid(pid), mr_iid));
            post(&token, &url, json!({"body": body})).await?;
            Ok("✅ Commentaire ajouté sur la MR.".to_string())
        }

        // ── Pipelines ─────────────────────────────────────────────────────────

        "gitlab_list_pipelines" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/pipelines?per_page={}", encode_pid(pid), limit));
            if let Some(s) = args["status"].as_str() { url.push_str(&format!("&status={s}")); }
            if let Some(r) = args["ref"].as_str()    { url.push_str(&format!("&ref={}", urlencoding::encode(r))); }
            let data      = get(&token, &url).await?;
            let pipelines = data.as_array().cloned().unwrap_or_default();
            if pipelines.is_empty() { return Ok("Aucun pipeline.".to_string()); }
            Ok(pipelines.iter().map(|p| {
                let id     = p["id"].as_u64().unwrap_or(0);
                let status = p["status"].as_str().unwrap_or("?");
                let ref_   = p["ref"].as_str().unwrap_or("?");
                let sha    = p["sha"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
                format!("[{id}] {status} · branche={ref_} · sha={sha}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_pipeline" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let pipeline_id = args["pipeline_id"].as_u64().ok_or("Missing param: pipeline_id")?;
            let url         = cfg.api(&format!("projects/{}/pipelines/{}", encode_pid(pid), pipeline_id));
            let p           = get(&token, &url).await?;
            let status   = p["status"].as_str().unwrap_or("?");
            let ref_     = p["ref"].as_str().unwrap_or("?");
            let sha      = p["sha"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
            let created  = p["created_at"].as_str().unwrap_or("?");
            let duration = p["duration"].as_u64().unwrap_or(0);
            let wurl     = p["web_url"].as_str().unwrap_or("");
            Ok(format!("Pipeline #{pipeline_id}\nStatut: {status} · Branche: {ref_} · SHA: {sha}\nCréé: {created} · Durée: {duration}s\n{wurl}"))
        }

        "gitlab_retry_pipeline" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let pipeline_id = args["pipeline_id"].as_u64().ok_or("Missing param: pipeline_id")?;
            let url         = cfg.api(&format!("projects/{}/pipelines/{}/retry", encode_pid(pid), pipeline_id));
            let data        = post(&token, &url, json!({})).await?;
            let new_id      = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Pipeline relancé — nouveau ID: {new_id}"))
        }

        "gitlab_cancel_pipeline" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let pipeline_id = args["pipeline_id"].as_u64().ok_or("Missing param: pipeline_id")?;
            let url         = cfg.api(&format!("projects/{}/pipelines/{}/cancel", encode_pid(pid), pipeline_id));
            post(&token, &url, json!({})).await?;
            Ok(format!("✅ Pipeline #{pipeline_id} annulé."))
        }

        // ── Projet & Utilisateurs ─────────────────────────────────────────────

        "gitlab_list_projects" => {
            let limit  = args["limit"].as_u64().unwrap_or(20);
            let mut url = if !cfg.groups.is_empty() {
                let gid = urlencoding::encode(&cfg.groups[0]).into_owned();
                cfg.api(&format!("groups/{}/projects?per_page={}&order_by=last_activity_at", gid, limit))
            } else {
                cfg.api(&format!("projects?membership=true&per_page={}&order_by=last_activity_at", limit))
            };
            if let Some(s) = args["search"].as_str() { url.push_str(&format!("&search={}", urlencoding::encode(s))); }
            let data     = get(&token, &url).await?;
            let projects = data.as_array().cloned().unwrap_or_default();
            if projects.is_empty() { return Ok("Aucun projet trouvé.".to_string()); }
            Ok(projects.iter().map(|p| {
                let name = p["path_with_namespace"].as_str().unwrap_or("?");
                let id   = p["id"].as_u64().unwrap_or(0);
                let desc = p["description"].as_str().unwrap_or("—");
                format!("{name}  id={id}\n  {desc}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "gitlab_get_project" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url  = cfg.api(&format!("projects/{}", encode_pid(pid)));
            let p    = get(&token, &url).await?;
            let name = p["path_with_namespace"].as_str().unwrap_or("?");
            let desc = p["description"].as_str().unwrap_or("—");
            let wurl = p["web_url"].as_str().unwrap_or("");
            let stars = p["star_count"].as_u64().unwrap_or(0);
            let forks = p["forks_count"].as_u64().unwrap_or(0);
            let branch = p["default_branch"].as_str().unwrap_or("main");
            Ok(format!("{name}\n{wurl}\nBranche défaut: {branch} · ⭐{stars} · 🍴{forks}\n\n{desc}"))
        }

        "gitlab_list_members" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url     = cfg.api(&format!("projects/{}/members/all?per_page=100", encode_pid(pid)));
            let data    = get(&token, &url).await?;
            let members = data.as_array().cloned().unwrap_or_default();
            if members.is_empty() { return Ok("Aucun membre.".to_string()); }
            Ok(members.iter().map(|m| {
                let name     = m["name"].as_str().unwrap_or("?");
                let username = m["username"].as_str().unwrap_or("?");
                let id       = m["id"].as_u64().unwrap_or(0);
                format!("{name} (@{username})  id={id}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_current_user" => {
            let url  = cfg.api("user");
            let data = get(&token, &url).await?;
            let name     = data["name"].as_str().unwrap_or("?");
            let username = data["username"].as_str().unwrap_or("?");
            let email    = data["email"].as_str().unwrap_or("?");
            let id       = data["id"].as_u64().unwrap_or(0);
            Ok(format!("{name} (@{username}) <{email}>\nid={id}"))
        }

        "gitlab_list_branches" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mut url = cfg.api(&format!("projects/{}/repository/branches?per_page=50", encode_pid(pid)));
            if let Some(s) = args["search"].as_str() { url.push_str(&format!("&search={}", urlencoding::encode(s))); }
            let data     = get(&token, &url).await?;
            let branches = data.as_array().cloned().unwrap_or_default();
            if branches.is_empty() { return Ok("Aucune branche.".to_string()); }
            Ok(branches.iter().map(|b| {
                let name      = b["name"].as_str().unwrap_or("?");
                let protected = b["protected"].as_bool().unwrap_or(false);
                let sha       = b["commit"]["id"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
                let flag      = if protected { " 🔒" } else { "" };
                format!("{name}{flag}  sha={sha}")
            }).collect::<Vec<_>>().join("\n"))
        }

        other => Err(format!("Unknown gitlab tool: {other}")),
    }
}
