/// Exécuteur d'actions approuvées.
///
/// Appelé après qu'un utilisateur a cliqué "Approuver" dans le dashboard.
/// Chaque tool `act_*` a son implémentation ici.
use osmozzz_core::action::ActionRequest;
use tracing::{info, warn};

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message as EmailMessage, Tokio1Executor,
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
};

// ─── Point d'entrée ──────────────────────────────────────────────────────────

/// Exécute une action approuvée. Retourne "ok: ..." ou "err: ...".
pub async fn execute(action: &ActionRequest) -> String {
    info!("[Executor] Exécution de '{}' (id={})", action.tool, action.id);
    let result = match action.tool.as_str() {
        "act_send_email"         => execute_send_email(action).await,
        "act_create_notion_page" => execute_notion_page(action).await,
        "act_send_slack_message" => execute_slack_message(action).await,
        "act_create_linear_issue"=> execute_linear_issue(action).await,
        "act_create_todoist_task"=> execute_todoist_task(action).await,
        "act_create_github_issue"=> execute_github_issue(action).await,
        "act_create_trello_card" => execute_trello_card(action).await,
        "act_create_gitlab_issue"=> execute_gitlab_issue(action).await,
        other => Err(format!("tool '{other}' non supporté par l'executor")),
    };
    match result {
        Ok(msg)  => { info!("[Executor] Succès: {msg}"); format!("ok: {msg}") }
        Err(e)   => { warn!("[Executor] Erreur: {e}"); format!("err: {e}") }
    }
}

// ─── act_send_email ───────────────────────────────────────────────────────────

async fn execute_send_email(action: &ActionRequest) -> Result<String, String> {
    let to      = str_param(action, "to")?;
    let subject = action.params["subject"].as_str().unwrap_or("(sans objet)");
    let body    = action.params["body"].as_str().unwrap_or("");

    let (username, password) = load_toml_kv("gmail.toml", &["username", "password"])
        .map(|mut v| (v.remove("username").unwrap_or_default(), v.remove("password").unwrap_or_default()))
        .ok_or_else(|| "Gmail non configuré — configurez-le dans le dashboard".to_string())?;

    let from_addr = username.parse::<lettre::Address>()
        .map_err(|e| format!("adresse from invalide: {e}"))?;
    let to_addr = to.parse::<lettre::Address>()
        .map_err(|e| format!("adresse to invalide: {e}"))?;

    let email = EmailMessage::builder()
        .from(lettre::message::Mailbox::new(None, from_addr))
        .to(lettre::message::Mailbox::new(None, to_addr))
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| format!("construction email: {e}"))?;

    let creds = Credentials::new(username.clone(), password);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .map_err(|e| format!("relay SMTP: {e}"))?
        .credentials(creds)
        .build();

    mailer.send(email).await
        .map(|_| format!("email envoyé à {to}"))
        .map_err(|e| format!("envoi SMTP: {e}"))
}

// ─── act_create_notion_page ──────────────────────────────────────────────────

async fn execute_notion_page(action: &ActionRequest) -> Result<String, String> {
    let title   = str_param(action, "title")?;
    let content = action.params["content"].as_str().unwrap_or("");
    let parent_id = action.params["parent_id"].as_str().unwrap_or("");

    let token = load_toml_single("notion.toml", "token")
        .ok_or_else(|| "Notion non configuré — configurez-le dans le dashboard".to_string())?;

    if parent_id.is_empty() {
        return Err("parent_id requis pour créer une page Notion".to_string());
    }

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "parent": { "page_id": parent_id },
        "properties": {
            "title": { "title": [{ "text": { "content": title } }] }
        },
        "children": [{
            "object": "block",
            "type": "paragraph",
            "paragraph": { "rich_text": [{ "text": { "content": content } }] }
        }]
    });

    let resp = client
        .post("https://api.notion.com/v1/pages")
        .bearer_auth(&token)
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send().await
        .map_err(|e| format!("requête Notion: {e}"))?;

    if resp.status().is_success() {
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        let url = data["url"].as_str().unwrap_or("(url inconnue)");
        Ok(format!("page Notion créée : {url}"))
    } else {
        let err: serde_json::Value = resp.json().await.unwrap_or_default();
        Err(format!("Notion API: {}", err["message"].as_str().unwrap_or("erreur inconnue")))
    }
}

// ─── act_send_slack_message ───────────────────────────────────────────────────

async fn execute_slack_message(action: &ActionRequest) -> Result<String, String> {
    let channel = str_param(action, "channel")?;
    let message = str_param(action, "message")?;

    let token = load_toml_single("slack.toml", "token")
        .ok_or_else(|| "Slack non configuré — configurez-le dans le dashboard".to_string())?;

    let client = reqwest::Client::new();
    let resp = client
        .post("https://slack.com/api/chat.postMessage")
        .bearer_auth(&token)
        .json(&serde_json::json!({ "channel": channel, "text": message }))
        .send().await
        .map_err(|e| format!("requête Slack: {e}"))?;

    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("réponse Slack: {e}"))?;

    if data["ok"].as_bool().unwrap_or(false) {
        Ok(format!("message envoyé dans #{channel}"))
    } else {
        Err(format!("Slack API: {}", data["error"].as_str().unwrap_or("erreur inconnue")))
    }
}

// ─── act_create_linear_issue ─────────────────────────────────────────────────

async fn execute_linear_issue(action: &ActionRequest) -> Result<String, String> {
    let title       = str_param(action, "title")?;
    let description = action.params["description"].as_str().unwrap_or("");

    let api_key = load_toml_single("linear.toml", "api_key")
        .ok_or_else(|| "Linear non configuré — configurez-le dans le dashboard".to_string())?;

    let client = reqwest::Client::new();

    // Récupère le premier team si team_id non fourni
    let team_id = if let Some(tid) = action.params["team_id"].as_str() {
        tid.to_string()
    } else {
        let teams_query = serde_json::json!({
            "query": "{ teams { nodes { id name } } }"
        });
        let resp: serde_json::Value = client
            .post("https://api.linear.app/graphql")
            .header("Authorization", &api_key)
            .json(&teams_query)
            .send().await
            .map_err(|e| format!("requête Linear teams: {e}"))?
            .json().await
            .map_err(|e| format!("réponse Linear teams: {e}"))?;
        resp["data"]["teams"]["nodes"][0]["id"]
            .as_str()
            .ok_or_else(|| "Aucune équipe Linear trouvée".to_string())?
            .to_string()
    };

    let mutation = serde_json::json!({
        "query": "mutation CreateIssue($input: IssueCreateInput!) { issueCreate(input: $input) { success issue { id title url } } }",
        "variables": { "input": { "title": title, "description": description, "teamId": team_id } }
    });

    let resp: serde_json::Value = client
        .post("https://api.linear.app/graphql")
        .header("Authorization", &api_key)
        .json(&mutation)
        .send().await
        .map_err(|e| format!("requête Linear: {e}"))?
        .json().await
        .map_err(|e| format!("réponse Linear: {e}"))?;

    if resp["data"]["issueCreate"]["success"].as_bool().unwrap_or(false) {
        let url = resp["data"]["issueCreate"]["issue"]["url"].as_str().unwrap_or("");
        Ok(format!("issue Linear créée : {url}"))
    } else {
        let errs = resp["errors"][0]["message"].as_str().unwrap_or("erreur inconnue");
        Err(format!("Linear API: {errs}"))
    }
}

// ─── act_create_todoist_task ─────────────────────────────────────────────────

async fn execute_todoist_task(action: &ActionRequest) -> Result<String, String> {
    let content    = str_param(action, "content")?;
    let due_string = action.params["due_string"].as_str().unwrap_or("");
    let project_id = action.params["project_id"].as_str().unwrap_or("");

    let token = load_toml_single("todoist.toml", "token")
        .ok_or_else(|| "Todoist non configuré — configurez-le dans le dashboard".to_string())?;

    let mut body = serde_json::json!({ "content": content });
    if !due_string.is_empty() { body["due_string"] = due_string.into(); }
    if !project_id.is_empty() { body["project_id"] = project_id.into(); }

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.todoist.com/rest/v2/tasks")
        .bearer_auth(&token)
        .json(&body)
        .send().await
        .map_err(|e| format!("requête Todoist: {e}"))?;

    if resp.status().is_success() {
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        let url = data["url"].as_str().unwrap_or("(url inconnue)");
        Ok(format!("tâche Todoist créée : {url}"))
    } else {
        Err(format!("Todoist API: HTTP {}", resp.status()))
    }
}

// ─── act_create_github_issue ─────────────────────────────────────────────────

async fn execute_github_issue(action: &ActionRequest) -> Result<String, String> {
    let title = str_param(action, "title")?;
    let body  = action.params["body"].as_str().unwrap_or("");

    // Repo : depuis les params ou premier repo de la config
    let repo = if let Some(r) = action.params["repo"].as_str() {
        r.to_string()
    } else {
        load_toml_array_first("github.toml", "repos")
            .ok_or_else(|| "Aucun repo GitHub configuré".to_string())?
    };

    let token = load_toml_single("github.toml", "token")
        .ok_or_else(|| "GitHub non configuré — configurez-le dans le dashboard".to_string())?;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("https://api.github.com/repos/{repo}/issues"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "osmozzz/1.0")
        .json(&serde_json::json!({ "title": title, "body": body }))
        .send().await
        .map_err(|e| format!("requête GitHub: {e}"))?;

    if resp.status().is_success() {
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        let url = data["html_url"].as_str().unwrap_or("(url inconnue)");
        Ok(format!("issue GitHub créée : {url}"))
    } else {
        let err: serde_json::Value = resp.json().await.unwrap_or_default();
        Err(format!("GitHub API: {}", err["message"].as_str().unwrap_or("erreur inconnue")))
    }
}

// ─── act_create_trello_card ───────────────────────────────────────────────────

async fn execute_trello_card(action: &ActionRequest) -> Result<String, String> {
    let name    = str_param(action, "name")?;
    let list_id = str_param(action, "list_id")?;
    let desc    = action.params["description"].as_str().unwrap_or("");

    let api_key = load_toml_single("trello.toml", "api_key")
        .ok_or_else(|| "Trello non configuré — configurez-le dans le dashboard".to_string())?;
    let token = load_toml_single("trello.toml", "token")
        .ok_or_else(|| "Trello non configuré — configurez-le dans le dashboard".to_string())?;

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.trello.com/1/cards")
        .query(&[
            ("key", api_key.as_str()), ("token", token.as_str()),
            ("idList", &list_id), ("name", &name), ("desc", desc),
        ])
        .send().await
        .map_err(|e| format!("requête Trello: {e}"))?;

    if resp.status().is_success() {
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        let url = data["shortUrl"].as_str().unwrap_or("(url inconnue)");
        Ok(format!("carte Trello créée : {url}"))
    } else {
        Err(format!("Trello API: HTTP {}", resp.status()))
    }
}

// ─── act_create_gitlab_issue ──────────────────────────────────────────────────

async fn execute_gitlab_issue(action: &ActionRequest) -> Result<String, String> {
    let title      = str_param(action, "title")?;
    let project_id = str_param(action, "project_id")?;
    let description = action.params["description"].as_str().unwrap_or("");

    let token = load_toml_single("gitlab.toml", "token")
        .ok_or_else(|| "GitLab non configuré — configurez-le dans le dashboard".to_string())?;
    let base_url = load_toml_single("gitlab.toml", "base_url")
        .unwrap_or_else(|| "https://gitlab.com".to_string());

    let encoded_id = project_id.replace('/', "%2F");
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base_url}/api/v4/projects/{encoded_id}/issues"))
        .header("PRIVATE-TOKEN", &token)
        .json(&serde_json::json!({ "title": title, "description": description }))
        .send().await
        .map_err(|e| format!("requête GitLab: {e}"))?;

    if resp.status().is_success() {
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        let url = data["web_url"].as_str().unwrap_or("(url inconnue)");
        Ok(format!("issue GitLab créée : {url}"))
    } else {
        let err: serde_json::Value = resp.json().await.unwrap_or_default();
        Err(format!("GitLab API: {}", err["message"].as_str().unwrap_or("erreur inconnue")))
    }
}

// ─── Helpers toml ─────────────────────────────────────────────────────────────

fn str_param<'a>(action: &'a ActionRequest, key: &str) -> Result<&'a str, String> {
    action.params[key].as_str()
        .ok_or_else(|| format!("paramètre '{key}' manquant"))
}

fn load_toml_single(filename: &str, key: &str) -> Option<String> {
    let path = dirs_next::home_dir()?.join(".osmozzz").join(filename);
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            if let Some(val) = rest.trim_start_matches(|c: char| c == ' ' || c == '=').strip_prefix('"') {
                return val.strip_suffix('"').map(String::from);
            }
        }
    }
    None
}

fn load_toml_kv(filename: &str, keys: &[&str]) -> Option<std::collections::HashMap<String, String>> {
    let path = dirs_next::home_dir()?.join(".osmozzz").join(filename);
    let content = std::fs::read_to_string(path).ok()?;
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        for &key in keys {
            if let Some(rest) = line.strip_prefix(key) {
                if let Some(val) = rest.trim_start_matches(|c: char| c == ' ' || c == '=').strip_prefix('"') {
                    if let Some(v) = val.strip_suffix('"') {
                        map.insert(key.to_string(), v.to_string());
                    }
                }
            }
        }
    }
    if map.is_empty() { None } else { Some(map) }
}

fn load_toml_array_first(filename: &str, key: &str) -> Option<String> {
    let path = dirs_next::home_dir()?.join(".osmozzz").join(filename);
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_array = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(&format!("{key}")) && line.contains('[') {
            in_array = true;
        }
        if in_array {
            if let Some(start) = line.find('"') {
                let rest = &line[start + 1..];
                if let Some(end) = rest.find('"') {
                    return Some(rest[..end].to_string());
                }
            }
            if line.contains(']') { break; }
        }
    }
    None
}
