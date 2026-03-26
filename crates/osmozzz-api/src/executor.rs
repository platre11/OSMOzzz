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
        "act_send_imessage"       => execute_send_imessage(action).await,
        "act_create_calendar_event"  => execute_create_calendar_event(action).await,
        "act_delete_calendar_event"  => execute_delete_calendar_event(action).await,
        "act_delete_note"            => execute_delete_note(action).await,
        "act_create_folder"          => execute_create_folder(action).await,
        "act_rename_file"         => execute_rename_file(action).await,
        "act_delete_file"         => execute_delete_file(action).await,
        "act_run_command"         => execute_run_command(action).await,
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

    let client = reqwest::Client::new();

    // Si pas de parent_id → cherche automatiquement une page accessible
    let resolved_parent_id: String;
    let parent = if !parent_id.is_empty() {
        serde_json::json!({ "page_id": parent_id })
    } else {
        let search: serde_json::Value = client
            .post("https://api.notion.com/v1/search")
            .bearer_auth(&token)
            .header("Notion-Version", "2022-06-28")
            .json(&serde_json::json!({ "filter": { "value": "page", "property": "object" }, "page_size": 1 }))
            .send().await
            .map_err(|e| format!("recherche Notion: {e}"))?
            .json().await
            .map_err(|e| format!("réponse Notion search: {e}"))?;
        resolved_parent_id = search["results"][0]["id"]
            .as_str()
            .ok_or_else(|| "Aucune page Notion accessible — partagez au moins une page avec l'intégration OSMOzzz".to_string())?
            .to_string();
        serde_json::json!({ "page_id": resolved_parent_id })
    };

    let body = serde_json::json!({
        "parent": parent,
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

// ─── act_send_imessage ────────────────────────────────────────────────────────

async fn execute_send_imessage(action: &ActionRequest) -> Result<String, String> {
    let to      = str_param(action, "to")?;
    let message = str_param(action, "message")?;

    // Échappe les guillemets pour AppleScript
    let escaped_msg = message.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_to  = to.replace('"', "\\\"");

    let script = format!(
        r#"tell application "Messages"
            set targetService to 1st service whose service type = iMessage
            set targetBuddy to buddy "{escaped_to}" of targetService
            send "{escaped_msg}" to targetBuddy
        end tell"#
    );

    run_osascript(&script)
        .await
        .map(|_| format!("iMessage envoyé à {to}"))
}

// ─── act_create_calendar_event ────────────────────────────────────────────────

async fn execute_create_calendar_event(action: &ActionRequest) -> Result<String, String> {
    let title      = str_param(action, "title")?;
    let start_date = str_param(action, "start_date")?; // "2026-03-17 14:00"
    let end_date   = action.params["end_date"].as_str().unwrap_or("");
    let calendar_hint = action.params["calendar"].as_str().unwrap_or("").trim().to_string();
    let notes      = action.params["notes"].as_str().unwrap_or("");

    // end_date = start_date + 1h si non fourni (même valeur → Calendar l'ajuste)
    let end = if end_date.is_empty() { start_date.to_string() } else { end_date.to_string() };

    // Résolution du nom de calendrier : si vide ou inconnu, on prend le premier disponible
    let calendar = if calendar_hint.is_empty() {
        // Récupère le nom du premier calendrier local
        let list_script = r#"tell application "Calendar"
            set calName to name of first calendar
            return calName
        end tell"#;
        run_osascript(list_script).await.unwrap_or_else(|_| "Calendrier".to_string())
    } else {
        // Vérifie que le calendrier demandé existe ; sinon retombe sur le premier
        let check_script = format!(
            r#"tell application "Calendar"
                set calList to name of every calendar
                if calList contains "{cal}" then
                    return "{cal}"
                else
                    return name of first calendar
                end if
            end tell"#,
            cal = calendar_hint.replace('"', "\\\"")
        );
        run_osascript(&check_script).await.unwrap_or(calendar_hint.clone())
    };

    let escaped_title    = title.replace('"', "\\\"");
    let escaped_notes    = notes.replace('"', "\\\"");
    let escaped_calendar = calendar.trim().replace('"', "\\\"");

    let script = format!(
        r#"tell application "Calendar"
            tell calendar "{escaped_calendar}"
                set newEvent to make new event with properties {{summary:"{escaped_title}", start date:date "{start_date}", end date:date "{end}", description:"{escaped_notes}"}}
            end tell
            reload calendars
        end tell"#
    );

    run_osascript(&script)
        .await
        .map(|_| format!("événement '{title}' créé dans le calendrier '{calendar}'"))
}

// ─── act_delete_calendar_event ───────────────────────────────────────────────

async fn execute_delete_calendar_event(action: &ActionRequest) -> Result<String, String> {
    let title = str_param(action, "title")?;
    let date  = action.params["date"].as_str().unwrap_or(""); // ex: "2026-03-17"

    let escaped_title = title.replace('"', "\\\"");

    // Itération manuelle — le filtre "whose" ne fonctionne pas sur les calendriers iCloud
    let script = if date.is_empty() {
        format!(
            r#"tell application "Calendar"
                repeat with c in every calendar
                    try
                        set evList to every event of c
                        repeat with e in evList
                            try
                                if summary of e is "{escaped_title}" then
                                    delete e
                                    return "ok"
                                end if
                            end try
                        end repeat
                    end try
                end repeat
                return "not found"
            end tell"#
        )
    } else {
        // Parse "YYYY-MM-DD"
        let parts: Vec<&str> = date.split('-').collect();
        let (yr, mo, dy) = if parts.len() == 3 {
            (parts[0], parts[1], parts[2])
        } else {
            ("2000", "1", "1")
        };
        format!(
            r#"tell application "Calendar"
                set d to current date
                set year of d to {yr}
                set month of d to {mo}
                set day of d to {dy}
                set hours of d to 0
                set minutes of d to 0
                set seconds of d to 0
                set dEnd to d + 1 * days
                repeat with c in every calendar
                    try
                        set evList to every event of c
                        repeat with e in evList
                            try
                                if summary of e is "{escaped_title}" then
                                    if start date of e >= d and start date of e < dEnd then
                                        delete e
                                        return "ok"
                                    end if
                                end if
                            end try
                        end repeat
                    end try
                end repeat
                return "not found"
            end tell"#
        )
    };

    let out = run_osascript(&script).await?;
    if out.contains("not found") {
        Err(format!("Aucun événement '{title}' trouvé{}", if date.is_empty() { String::new() } else { format!(" le {date}") }))
    } else {
        Ok(format!("Événement '{title}' supprimé du calendrier"))
    }
}

// ─── act_delete_note ─────────────────────────────────────────────────────────

async fn execute_delete_note(action: &ActionRequest) -> Result<String, String> {
    let title = str_param(action, "title")?;
    let escaped_title = title.replace('"', "\\\"");

    let script = format!(
        r#"tell application "Notes"
            set matchedNotes to every note whose name is "{escaped_title}"
            if (count of matchedNotes) is 0 then
                return "not found"
            end if
            delete (item 1 of matchedNotes)
            return "ok"
        end tell"#
    );

    let out = run_osascript(&script).await?;
    if out.contains("not found") {
        Err(format!("Aucune note '{title}' trouvée"))
    } else {
        Ok(format!("Note '{title}' supprimée"))
    }
}

// ─── act_create_folder ────────────────────────────────────────────────────────

async fn execute_create_folder(action: &ActionRequest) -> Result<String, String> {
    let path = str_param(action, "path")?;
    let expanded = shellexpand::tilde(path).to_string();
    std::fs::create_dir_all(&expanded)
        .map(|_| format!("dossier créé : {expanded}"))
        .map_err(|e| format!("création dossier: {e}"))
}

// ─── act_rename_file ──────────────────────────────────────────────────────────

async fn execute_rename_file(action: &ActionRequest) -> Result<String, String> {
    let from = str_param(action, "from")?;
    let to   = str_param(action, "to")?;
    let from_exp = shellexpand::tilde(from).to_string();
    let to_exp   = shellexpand::tilde(to).to_string();
    std::fs::rename(&from_exp, &to_exp)
        .map(|_| format!("renommé : {from_exp} → {to_exp}"))
        .map_err(|e| format!("renommage: {e}"))
}

// ─── act_delete_file ──────────────────────────────────────────────────────────

async fn execute_delete_file(action: &ActionRequest) -> Result<String, String> {
    let path = str_param(action, "path")?;
    let expanded = shellexpand::tilde(path).to_string();
    let p = std::path::Path::new(&expanded);
    if p.is_dir() {
        std::fs::remove_dir_all(&expanded)
            .map(|_| format!("dossier supprimé : {expanded}"))
            .map_err(|e| format!("suppression dossier: {e}"))
    } else {
        std::fs::remove_file(&expanded)
            .map(|_| format!("fichier supprimé : {expanded}"))
            .map_err(|e| format!("suppression fichier: {e}"))
    }
}

// ─── act_run_command ──────────────────────────────────────────────────────────

async fn execute_run_command(action: &ActionRequest) -> Result<String, String> {
    let command = str_param(action, "command")?;
    let workdir = action.params["workdir"].as_str().unwrap_or("~");
    let expanded_dir = shellexpand::tilde(workdir).to_string();

    let output = tokio::process::Command::new("zsh")
        .arg("-c")
        .arg(command)
        .current_dir(&expanded_dir)
        .output()
        .await
        .map_err(|e| format!("exécution commande: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        let result = if stdout.is_empty() { "(pas de sortie)".to_string() } else { stdout };
        Ok(format!("$ {command}\n{result}"))
    } else {
        let err = if stderr.is_empty() { stdout } else { stderr };
        Err(format!("$ {command}\nexit {}: {err}", output.status.code().unwrap_or(-1)))
    }
}

// ─── Helper AppleScript ───────────────────────────────────────────────────────

async fn run_osascript(script: &str) -> Result<String, String> {
    let output = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .await
        .map_err(|e| format!("osascript: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// ─── Re-sync après action ────────────────────────────────────────────────────

/// Déclenche une re-sync immédiate de la source concernée après une action réussie.
/// Mapping tool → source harvester.
pub async fn sync_source_after_action(tool: &str, vault: &std::sync::Arc<osmozzz_embedder::Vault>) {
    use osmozzz_core::{Embedder, Harvester};

    let source = match tool {
        "act_create_notion_page"    => "notion",
        "act_create_linear_issue"   => "linear",
        "act_create_todoist_task"   => "todoist",
        "act_create_github_issue"   => "github",
        "act_create_trello_card"    => "trello",
        "act_create_gitlab_issue"   => "gitlab",
        "act_send_slack_message"    => "slack",
        "act_create_calendar_event" => "calendar",
        _ => return, // email, imessage, fichiers, run_command : pas de source re-sync automatique
    };

    info!("[Executor] Re-sync immédiate de la source '{source}' après action réussie");

    let docs = match source {
        "notion"  => osmozzz_harvester::NotionHarvester::new().harvest().await,
        "linear"  => osmozzz_harvester::LinearHarvester::new().harvest().await,
        "todoist" => osmozzz_harvester::TodoistHarvester::new().harvest().await,
        "github"  => osmozzz_harvester::GithubHarvester::new().harvest().await,
        "trello"  => osmozzz_harvester::TrelloHarvester::new().harvest().await,
        "gitlab"  => osmozzz_harvester::GitlabHarvester::new().harvest().await,
        "slack"    => osmozzz_harvester::SlackHarvester::new().harvest().await,
        #[cfg(target_os = "macos")]
        "calendar" => osmozzz_harvester::CalendarHarvester::new().harvest().await,
        _ => return,
    };

    match docs {
        Ok(docs) => {
            let count = docs.len();
            if let Err(e) = { let mut errs = 0usize; for doc in &docs { if vault.upsert(doc).await.is_err() { errs += 1; } } if errs > 0 { Err(osmozzz_core::OsmozzError::Storage(format!("{errs} docs failed"))) } else { Ok(()) } } {
                warn!("[Executor] Erreur re-sync {source}: {e}");
            } else {
                info!("[Executor] Re-sync {source}: {count} docs upsertés");
            }
        }
        Err(e) => warn!("[Executor] Re-sync {source} harvest échoué: {e}"),
    }
}
