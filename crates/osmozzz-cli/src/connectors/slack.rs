/// Connecteur Slack — Slack Web API.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct SlackConfig {
    pub token:   String,
    pub team_id: Option<String>,
}

impl SlackConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/slack.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!("https://slack.com/api/{}", path.trim_start_matches('/'))
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &SlackConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &SlackConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
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

/// Vérifie le champ `ok` de la réponse Slack et retourne une erreur lisible si false.
fn slack_ok(resp: &Value) -> Result<(), String> {
    if resp["ok"].as_bool().unwrap_or(false) {
        Ok(())
    } else {
        let err = resp["error"].as_str().unwrap_or("unknown_error");
        let detail = resp["detail"].as_str().unwrap_or("");
        if detail.is_empty() {
            Err(format!("Slack API error: {err}"))
        } else {
            Err(format!("Slack API error: {err} — {detail}"))
        }
    }
}

// ─── Tool definitions ─────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Channels ────────────────────────────────────────────────────────
        json!({
            "name": "slack_list_channels",
            "description": "SLACK 💬 — Liste les channels du workspace Slack (publics et/ou privés). Retourne nom, ID et sujet. Supporte la pagination via cursor.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit":            { "type": "integer", "description": "Nombre max de channels à retourner (défaut: 100, max: 1000)", "default": 100 },
                    "cursor":           { "type": "string",  "description": "Curseur de pagination (next_cursor issu d'un appel précédent)" },
                    "exclude_archived": { "type": "boolean", "description": "Exclure les channels archivés (défaut: true)", "default": true },
                    "types":            { "type": "string",  "description": "Types de channels : public_channel, private_channel, mpim, im (séparés par virgule, défaut: public_channel)", "default": "public_channel" }
                }
            }
        }),
        json!({
            "name": "slack_get_channel_info",
            "description": "SLACK 💬 — Récupère les détails d'un channel Slack : nom, sujet, objectif, nombre de membres, date de création.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel (ex: C01234ABC)" }
                },
                "required": ["channel_id"]
            }
        }),
        json!({
            "name": "slack_create_channel",
            "description": "SLACK 💬 — Crée un nouveau channel Slack public ou privé. Retourne l'ID et le nom du channel créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":       { "type": "string",  "description": "Nom du channel (lowercase, sans espaces)" },
                    "is_private": { "type": "boolean", "description": "Créer un channel privé (défaut: false)", "default": false }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "slack_invite_to_channel",
            "description": "SLACK 💬 — Invite un ou plusieurs utilisateurs dans un channel Slack.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "user_ids":   { "type": "array", "items": { "type": "string" }, "description": "Liste des IDs utilisateurs à inviter" }
                },
                "required": ["channel_id", "user_ids"]
            }
        }),
        json!({
            "name": "slack_set_topic",
            "description": "SLACK 💬 — Définit ou modifie le sujet d'un channel Slack.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "topic":      { "type": "string", "description": "Nouveau sujet du channel" }
                },
                "required": ["channel_id", "topic"]
            }
        }),
        // ── Messages ─────────────────────────────────────────────────────────
        json!({
            "name": "slack_post_message",
            "description": "SLACK 💬 — Envoie un message dans un channel Slack. Supporte le formatage Markdown Slack (mrkdwn) et les blocks optionnels. Retourne le timestamp du message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel ou nom (@user pour DM)" },
                    "text":       { "type": "string", "description": "Texte du message" },
                    "blocks":     { "type": "array",  "description": "Blocks Block Kit optionnels (JSON)" },
                    "thread_ts":  { "type": "string", "description": "Timestamp du message parent pour répondre en fil" },
                    "mrkdwn":     { "type": "boolean","description": "Activer le rendu Markdown Slack (défaut: true)", "default": true }
                },
                "required": ["channel_id", "text"]
            }
        }),
        json!({
            "name": "slack_reply_to_thread",
            "description": "SLACK 💬 — Répond dans un fil (thread) Slack existant. Utiliser slack_get_channel_history pour obtenir le thread_ts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "thread_ts":  { "type": "string", "description": "Timestamp du message parent (ex: 1234567890.123456)" },
                    "text":       { "type": "string", "description": "Texte de la réponse" },
                    "blocks":     { "type": "array",  "description": "Blocks Block Kit optionnels" },
                    "mrkdwn":     { "type": "boolean","description": "Activer le rendu Markdown Slack (défaut: true)", "default": true }
                },
                "required": ["channel_id", "thread_ts", "text"]
            }
        }),
        json!({
            "name": "slack_update_message",
            "description": "SLACK 💬 — Modifie le contenu d'un message Slack existant. Nécessite le channel_id et le timestamp (ts) du message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "ts":         { "type": "string", "description": "Timestamp du message à modifier" },
                    "text":       { "type": "string", "description": "Nouveau texte du message" },
                    "blocks":     { "type": "array",  "description": "Nouveaux blocks Block Kit (optionnel)" }
                },
                "required": ["channel_id", "ts", "text"]
            }
        }),
        json!({
            "name": "slack_delete_message",
            "description": "SLACK 💬 — Supprime un message Slack. Nécessite le channel_id et le timestamp (ts) du message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "ts":         { "type": "string", "description": "Timestamp du message à supprimer" }
                },
                "required": ["channel_id", "ts"]
            }
        }),
        json!({
            "name": "slack_add_reaction",
            "description": "SLACK 💬 — Ajoute une réaction emoji à un message Slack. Le nom de l'emoji ne doit pas contenir les ':'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id":    { "type": "string", "description": "ID du channel" },
                    "timestamp":     { "type": "string", "description": "Timestamp du message (ts)" },
                    "reaction_name": { "type": "string", "description": "Nom de l'emoji sans ':' (ex: thumbsup, rocket, heart)" }
                },
                "required": ["channel_id", "timestamp", "reaction_name"]
            }
        }),
        // ── History & Threads ────────────────────────────────────────────────
        json!({
            "name": "slack_get_channel_history",
            "description": "SLACK 💬 — Récupère l'historique des messages d'un channel Slack. Retourne les messages avec auteur, texte et timestamp. Supporte filtre par date (oldest/latest).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "limit":      { "type": "integer","description": "Nombre de messages à retourner (défaut: 20, max: 200)", "default": 20 },
                    "cursor":     { "type": "string", "description": "Curseur de pagination" },
                    "oldest":     { "type": "string", "description": "Timestamp Unix minimum (messages après cette date)" },
                    "latest":     { "type": "string", "description": "Timestamp Unix maximum (messages avant cette date)" }
                },
                "required": ["channel_id"]
            }
        }),
        json!({
            "name": "slack_get_thread_replies",
            "description": "SLACK 💬 — Récupère toutes les réponses d'un fil (thread) Slack. Utiliser slack_get_channel_history pour obtenir le thread_ts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "thread_ts":  { "type": "string", "description": "Timestamp du message parent" },
                    "limit":      { "type": "integer","description": "Nombre de réponses à retourner (défaut: 20, max: 200)", "default": 20 },
                    "cursor":     { "type": "string", "description": "Curseur de pagination" }
                },
                "required": ["channel_id", "thread_ts"]
            }
        }),
        json!({
            "name": "slack_search_messages",
            "description": "SLACK 💬 — Recherche des messages dans le workspace Slack par mots-clés. Nécessite un token utilisateur (xoxp-) et non un bot token. Retourne les messages correspondants avec leur contexte.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string",  "description": "Termes de recherche (supporte les opérateurs Slack: in:#channel, from:@user, etc.)" },
                    "sort":  { "type": "string",  "description": "Tri : score (pertinence, défaut) ou timestamp", "enum": ["score", "timestamp"], "default": "score" },
                    "count": { "type": "integer", "description": "Nombre de résultats par page (défaut: 20, max: 100)", "default": 20 },
                    "page":  { "type": "integer", "description": "Numéro de page (défaut: 1)", "default": 1 }
                },
                "required": ["query"]
            }
        }),
        // ── Users ─────────────────────────────────────────────────────────────
        json!({
            "name": "slack_get_users",
            "description": "SLACK 💬 — Liste tous les utilisateurs du workspace Slack avec leur nom, email et statut. Supporte la pagination.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit":  { "type": "integer", "description": "Nombre d'utilisateurs à retourner (défaut: 100, max: 1000)", "default": 100 },
                    "cursor": { "type": "string",  "description": "Curseur de pagination" }
                }
            }
        }),
        json!({
            "name": "slack_get_user_profile",
            "description": "SLACK 💬 — Récupère le profil complet d'un utilisateur Slack : nom complet, email, titre, statut, timezone.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "user_id": { "type": "string", "description": "ID de l'utilisateur Slack (ex: U01234ABC)" }
                },
                "required": ["user_id"]
            }
        }),
        // ── Files ─────────────────────────────────────────────────────────────
        json!({
            "name": "slack_upload_file",
            "description": "SLACK 💬 — Uploade un fichier texte dans un channel Slack avec un titre et commentaire optionnels.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id":      { "type": "string", "description": "ID du channel où partager le fichier" },
                    "content":         { "type": "string", "description": "Contenu textuel du fichier" },
                    "filename":        { "type": "string", "description": "Nom du fichier (ex: rapport.txt)" },
                    "title":           { "type": "string", "description": "Titre affiché dans Slack (optionnel)" },
                    "initial_comment": { "type": "string", "description": "Commentaire accompagnant le fichier (optionnel)" }
                },
                "required": ["channel_id", "content", "filename"]
            }
        }),
        // ── Team ─────────────────────────────────────────────────────────────
        json!({
            "name": "slack_get_team_info",
            "description": "SLACK 💬 — Récupère les informations du workspace Slack : nom, domaine, icône, plan.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        // ── DM ────────────────────────────────────────────────────────────────
        json!({
            "name": "slack_open_dm",
            "description": "SLACK 💬 — Ouvre une conversation DM (message direct) avec un utilisateur Slack. Retourne le channel_id du DM pour envoyer des messages via slack_post_message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "user_id": { "type": "string", "description": "ID de l'utilisateur Slack avec qui ouvrir le DM" }
                },
                "required": ["user_id"]
            }
        }),
    ]
}

// ─── Dispatcher ───────────────────────────────────────────────────────────────

pub async fn handle(tool: &str, args: &Value) -> Result<String, String> {
    let cfg = SlackConfig::load()
        .ok_or_else(|| "Slack non configuré — créer ~/.osmozzz/slack.toml avec token".to_string())?;

    match tool {
        // ── Channels ────────────────────────────────────────────────────────
        "slack_list_channels" => {
            let limit            = args["limit"].as_u64().unwrap_or(100).min(1000);
            let exclude_archived = args["exclude_archived"].as_bool().unwrap_or(true);
            let types            = args["types"].as_str().unwrap_or("public_channel");

            let mut url = format!(
                "{}?limit={limit}&exclude_archived={exclude_archived}&types={types}",
                cfg.api("conversations.list")
            );
            if let Some(cursor) = args["cursor"].as_str() {
                if !cursor.is_empty() {
                    url.push_str(&format!("&cursor={cursor}"));
                }
            }

            let resp = get(&cfg, &url).await?;
            slack_ok(&resp)?;

            let channels = resp["channels"].as_array().cloned().unwrap_or_default();
            if channels.is_empty() {
                return Ok("Aucun channel trouvé.".to_string());
            }

            let next_cursor = resp["response_metadata"]["next_cursor"].as_str().unwrap_or("");
            let mut out = format!("{} channel(s) :\n", channels.len());
            for c in &channels {
                let id      = c["id"].as_str().unwrap_or("—");
                let name    = c["name"].as_str().unwrap_or("—");
                let topic   = c["topic"]["value"].as_str().unwrap_or("");
                let members = c["num_members"].as_u64().unwrap_or(0);
                if topic.is_empty() {
                    out.push_str(&format!("• [{id}] #{name} ({members} membres)\n"));
                } else {
                    let topic_short = if topic.len() > 60 { &topic[..60] } else { topic };
                    out.push_str(&format!("• [{id}] #{name} ({members} membres) — {topic_short}\n"));
                }
            }
            if !next_cursor.is_empty() {
                out.push_str(&format!("\nPagination : cursor={next_cursor}"));
            }
            Ok(out.trim_end().to_string())
        }

        "slack_get_channel_info" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let url  = format!("{}?channel={channel_id}", cfg.api("conversations.info"));
            let resp = get(&cfg, &url).await?;
            slack_ok(&resp)?;

            let c       = &resp["channel"];
            let id      = c["id"].as_str().unwrap_or(channel_id);
            let name    = c["name"].as_str().unwrap_or("—");
            let topic   = c["topic"]["value"].as_str().unwrap_or("—");
            let purpose = c["purpose"]["value"].as_str().unwrap_or("—");
            let members = c["num_members"].as_u64().unwrap_or(0);
            let created = c["created"].as_u64().unwrap_or(0);
            let is_priv = c["is_private"].as_bool().unwrap_or(false);
            let archived = c["is_archived"].as_bool().unwrap_or(false);

            Ok(format!(
                "Channel {id}\nNom        : #{name}\nType       : {}\nMembres    : {members}\nSujet      : {topic}\nObjectif   : {purpose}\nCréé       : {created}\nArchivé    : {archived}",
                if is_priv { "privé" } else { "public" }
            ))
        }

        "slack_create_channel" => {
            let name       = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let is_private = args["is_private"].as_bool().unwrap_or(false);

            let body = json!({ "name": name, "is_private": is_private });
            let resp = post_json(&cfg, &cfg.api("conversations.create"), &body).await?;
            slack_ok(&resp)?;

            let c  = &resp["channel"];
            let id = c["id"].as_str().unwrap_or("—");
            Ok(format!(
                "Channel créé.\nID   : {id}\nNom  : #{name}\nType : {}",
                if is_private { "privé" } else { "public" }
            ))
        }

        "slack_invite_to_channel" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let user_ids: Vec<&str> = args["user_ids"].as_array()
                .ok_or("Paramètre 'user_ids' requis (tableau)")?
                .iter()
                .filter_map(|v| v.as_str())
                .collect();

            if user_ids.is_empty() {
                return Err("'user_ids' ne peut pas être vide".to_string());
            }

            let body = json!({
                "channel": channel_id,
                "users":   user_ids.join(",")
            });
            let resp = post_json(&cfg, &cfg.api("conversations.invite"), &body).await?;
            slack_ok(&resp)?;

            Ok(format!(
                "{} utilisateur(s) invité(s) dans le channel {channel_id}.",
                user_ids.len()
            ))
        }

        "slack_set_topic" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let topic      = args["topic"].as_str().ok_or("Paramètre 'topic' requis")?;

            let body = json!({ "channel": channel_id, "topic": topic });
            let resp = post_json(&cfg, &cfg.api("conversations.setTopic"), &body).await?;
            slack_ok(&resp)?;

            Ok(format!("Sujet du channel {channel_id} mis à jour : {topic}"))
        }

        // ── Messages ─────────────────────────────────────────────────────────
        "slack_post_message" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let text       = args["text"].as_str().ok_or("Paramètre 'text' requis")?;
            let mrkdwn     = args["mrkdwn"].as_bool().unwrap_or(true);

            let mut body = json!({
                "channel": channel_id,
                "text":    text,
                "mrkdwn":  mrkdwn
            });
            if let Some(blocks) = args["blocks"].as_array() {
                body["blocks"] = json!(blocks);
            }
            if let Some(ts) = args["thread_ts"].as_str() {
                if !ts.is_empty() {
                    body["thread_ts"] = json!(ts);
                }
            }

            let resp = post_json(&cfg, &cfg.api("chat.postMessage"), &body).await?;
            slack_ok(&resp)?;

            let ts = resp["ts"].as_str().unwrap_or("—");
            Ok(format!("Message envoyé dans {channel_id}.\nTimestamp : {ts}"))
        }

        "slack_reply_to_thread" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let thread_ts  = args["thread_ts"].as_str().ok_or("Paramètre 'thread_ts' requis")?;
            let text       = args["text"].as_str().ok_or("Paramètre 'text' requis")?;
            let mrkdwn     = args["mrkdwn"].as_bool().unwrap_or(true);

            let mut body = json!({
                "channel":   channel_id,
                "thread_ts": thread_ts,
                "text":      text,
                "mrkdwn":    mrkdwn
            });
            if let Some(blocks) = args["blocks"].as_array() {
                body["blocks"] = json!(blocks);
            }

            let resp = post_json(&cfg, &cfg.api("chat.postMessage"), &body).await?;
            slack_ok(&resp)?;

            let ts = resp["ts"].as_str().unwrap_or("—");
            Ok(format!("Réponse envoyée dans le fil {thread_ts} (channel: {channel_id}).\nTimestamp : {ts}"))
        }

        "slack_update_message" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let ts         = args["ts"].as_str().ok_or("Paramètre 'ts' requis")?;
            let text       = args["text"].as_str().ok_or("Paramètre 'text' requis")?;

            let mut body = json!({
                "channel": channel_id,
                "ts":      ts,
                "text":    text
            });
            if let Some(blocks) = args["blocks"].as_array() {
                body["blocks"] = json!(blocks);
            }

            let resp = post_json(&cfg, &cfg.api("chat.update"), &body).await?;
            slack_ok(&resp)?;

            Ok(format!("Message {ts} modifié dans {channel_id}."))
        }

        "slack_delete_message" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let ts         = args["ts"].as_str().ok_or("Paramètre 'ts' requis")?;

            let body = json!({ "channel": channel_id, "ts": ts });
            let resp = post_json(&cfg, &cfg.api("chat.delete"), &body).await?;
            slack_ok(&resp)?;

            Ok(format!("Message {ts} supprimé du channel {channel_id}."))
        }

        "slack_add_reaction" => {
            let channel_id    = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let timestamp     = args["timestamp"].as_str().ok_or("Paramètre 'timestamp' requis")?;
            let reaction_name = args["reaction_name"].as_str().ok_or("Paramètre 'reaction_name' requis")?;

            // Retirer les ':' si l'utilisateur les inclut
            let clean_name = reaction_name.trim_matches(':');

            let body = json!({
                "channel":   channel_id,
                "timestamp": timestamp,
                "name":      clean_name
            });
            let resp = post_json(&cfg, &cfg.api("reactions.add"), &body).await?;
            slack_ok(&resp)?;

            Ok(format!("Réaction :{clean_name}: ajoutée au message {timestamp}."))
        }

        // ── History & Threads ────────────────────────────────────────────────
        "slack_get_channel_history" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let limit      = args["limit"].as_u64().unwrap_or(20).min(200);

            let mut url = format!(
                "{}?channel={channel_id}&limit={limit}",
                cfg.api("conversations.history")
            );
            if let Some(cursor) = args["cursor"].as_str() {
                if !cursor.is_empty() { url.push_str(&format!("&cursor={cursor}")); }
            }
            if let Some(oldest) = args["oldest"].as_str() {
                if !oldest.is_empty() { url.push_str(&format!("&oldest={oldest}")); }
            }
            if let Some(latest) = args["latest"].as_str() {
                if !latest.is_empty() { url.push_str(&format!("&latest={latest}")); }
            }

            let resp = get(&cfg, &url).await?;
            slack_ok(&resp)?;

            let messages = resp["messages"].as_array().cloned().unwrap_or_default();
            if messages.is_empty() {
                return Ok(format!("Aucun message dans #{channel_id}."));
            }

            let next_cursor = resp["response_metadata"]["next_cursor"].as_str().unwrap_or("");
            let mut out = format!("{} message(s) dans #{channel_id} :\n", messages.len());
            for m in &messages {
                let ts      = m["ts"].as_str().unwrap_or("—");
                let user    = m["user"].as_str().unwrap_or("(bot/app)");
                let text    = m["text"].as_str().unwrap_or("(contenu non textuel)");
                let preview = if text.len() > 120 { &text[..120] } else { text };
                let has_thread = m["reply_count"].as_u64().unwrap_or(0);
                if has_thread > 0 {
                    out.push_str(&format!("[{ts}] {user}: {preview} [🧵 {has_thread} réponse(s)]\n"));
                } else {
                    out.push_str(&format!("[{ts}] {user}: {preview}\n"));
                }
            }
            if !next_cursor.is_empty() {
                out.push_str(&format!("\nPagination : cursor={next_cursor}"));
            }
            Ok(out.trim_end().to_string())
        }

        "slack_get_thread_replies" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let thread_ts  = args["thread_ts"].as_str().ok_or("Paramètre 'thread_ts' requis")?;
            let limit      = args["limit"].as_u64().unwrap_or(20).min(200);

            let mut url = format!(
                "{}?channel={channel_id}&ts={thread_ts}&limit={limit}",
                cfg.api("conversations.replies")
            );
            if let Some(cursor) = args["cursor"].as_str() {
                if !cursor.is_empty() { url.push_str(&format!("&cursor={cursor}")); }
            }

            let resp = get(&cfg, &url).await?;
            slack_ok(&resp)?;

            let messages = resp["messages"].as_array().cloned().unwrap_or_default();
            if messages.is_empty() {
                return Ok(format!("Aucune réponse dans le fil {thread_ts}."));
            }

            let next_cursor = resp["response_metadata"]["next_cursor"].as_str().unwrap_or("");
            let mut out = format!("{} message(s) dans le fil {thread_ts} :\n", messages.len());
            for (i, m) in messages.iter().enumerate() {
                let ts   = m["ts"].as_str().unwrap_or("—");
                let user = m["user"].as_str().unwrap_or("(bot/app)");
                let text = m["text"].as_str().unwrap_or("(contenu non textuel)");
                let preview = if text.len() > 120 { &text[..120] } else { text };
                let prefix = if i == 0 { "[parent]" } else { "[réponse]" };
                out.push_str(&format!("{prefix} [{ts}] {user}: {preview}\n"));
            }
            if !next_cursor.is_empty() {
                out.push_str(&format!("\nPagination : cursor={next_cursor}"));
            }
            Ok(out.trim_end().to_string())
        }

        "slack_search_messages" => {
            let query = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let sort  = args["sort"].as_str().unwrap_or("score");
            let count = args["count"].as_u64().unwrap_or(20).min(100);
            let page  = args["page"].as_u64().unwrap_or(1);

            let url = format!(
                "{}?query={query}&sort={sort}&count={count}&page={page}",
                cfg.api("search.messages"),
                query = urlencoding_simple(query)
            );
            let resp = get(&cfg, &url).await?;
            slack_ok(&resp)?;

            let matches = resp["messages"]["matches"].as_array().cloned().unwrap_or_default();
            let total   = resp["messages"]["total"].as_u64().unwrap_or(0);

            if matches.is_empty() {
                return Ok(format!("Aucun message trouvé pour '{query}'."));
            }

            let mut out = format!("{} résultat(s) (total: {total}) pour '{query}' :\n", matches.len());
            for m in &matches {
                let ts      = m["ts"].as_str().unwrap_or("—");
                let user    = m["username"].as_str().unwrap_or("—");
                let channel = m["channel"]["name"].as_str().unwrap_or("—");
                let text    = m["text"].as_str().unwrap_or("—");
                let preview = if text.len() > 100 { &text[..100] } else { text };
                out.push_str(&format!("• [{ts}] @{user} dans #{channel}: {preview}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Users ─────────────────────────────────────────────────────────────
        "slack_get_users" => {
            let limit = args["limit"].as_u64().unwrap_or(100).min(1000);

            let mut url = format!("{}?limit={limit}", cfg.api("users.list"));
            if let Some(cursor) = args["cursor"].as_str() {
                if !cursor.is_empty() { url.push_str(&format!("&cursor={cursor}")); }
            }

            let resp = get(&cfg, &url).await?;
            slack_ok(&resp)?;

            let members = resp["members"].as_array().cloned().unwrap_or_default();
            if members.is_empty() {
                return Ok("Aucun utilisateur trouvé.".to_string());
            }

            let next_cursor = resp["response_metadata"]["next_cursor"].as_str().unwrap_or("");
            // Filtrer les bots et utilisateurs supprimés
            let active: Vec<&Value> = members.iter()
                .filter(|u| !u["deleted"].as_bool().unwrap_or(false))
                .collect();

            let mut out = format!("{} utilisateur(s) actif(s) :\n", active.len());
            for u in &active {
                let id       = u["id"].as_str().unwrap_or("—");
                let name     = u["name"].as_str().unwrap_or("—");
                let real     = u["real_name"].as_str().unwrap_or("—");
                let is_bot   = u["is_bot"].as_bool().unwrap_or(false);
                let bot_tag  = if is_bot { " [bot]" } else { "" };
                out.push_str(&format!("• [{id}] @{name} ({real}){bot_tag}\n"));
            }
            if !next_cursor.is_empty() {
                out.push_str(&format!("\nPagination : cursor={next_cursor}"));
            }
            Ok(out.trim_end().to_string())
        }

        "slack_get_user_profile" => {
            let user_id = args["user_id"].as_str().ok_or("Paramètre 'user_id' requis")?;
            let url  = format!("{}?user={user_id}", cfg.api("users.profile.get"));
            let resp = get(&cfg, &url).await?;
            slack_ok(&resp)?;

            let p         = &resp["profile"];
            let real      = p["real_name"].as_str().unwrap_or("—");
            let display   = p["display_name"].as_str().unwrap_or("—");
            let title     = p["title"].as_str().unwrap_or("—");
            let email     = p["email"].as_str().unwrap_or("—");
            let phone     = p["phone"].as_str().unwrap_or("—");
            let tz        = p["tz"].as_str().unwrap_or("—");
            let status    = p["status_text"].as_str().unwrap_or("—");
            let status_em = p["status_emoji"].as_str().unwrap_or("");

            Ok(format!(
                "Profil {user_id}\nNom réel     : {real}\nNom affiché  : {display}\nTitre        : {title}\nEmail        : {email}\nTéléphone    : {phone}\nTimezone     : {tz}\nStatut       : {status_em} {status}"
            ))
        }

        // ── Files ─────────────────────────────────────────────────────────────
        "slack_upload_file" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let content    = args["content"].as_str().ok_or("Paramètre 'content' requis")?;
            let filename   = args["filename"].as_str().ok_or("Paramètre 'filename' requis")?;

            // Encode les champs en application/x-www-form-urlencoded (pas besoin de feature multipart)
            let mut params = vec![
                ("channels", channel_id.to_string()),
                ("filename", filename.to_string()),
                ("content",  content.to_string()),
            ];
            if let Some(title) = args["title"].as_str() {
                params.push(("title", title.to_string()));
            }
            if let Some(comment) = args["initial_comment"].as_str() {
                params.push(("initial_comment", comment.to_string()));
            }

            let resp = reqwest::Client::new()
                .post(cfg.api("files.upload"))
                .header("Authorization", format!("Bearer {}", cfg.token))
                .form(&params)
                .send()
                .await
                .map_err(|e: reqwest::Error| e.to_string())?
                .json::<Value>()
                .await
                .map_err(|e: reqwest::Error| e.to_string())?;

            slack_ok(&resp)?;

            let file_id   = resp["file"]["id"].as_str().unwrap_or("—");
            let file_name = resp["file"]["name"].as_str().unwrap_or(filename);
            Ok(format!(
                "Fichier uploadé dans {channel_id}.\nID   : {file_id}\nNom  : {file_name}"
            ))
        }

        // ── Team ─────────────────────────────────────────────────────────────
        "slack_get_team_info" => {
            let mut url = cfg.api("team.info");
            if let Some(team_id) = cfg.team_id.as_deref() {
                if !team_id.is_empty() {
                    url = format!("{}?team={team_id}", cfg.api("team.info"));
                }
            }

            let resp = get(&cfg, &url).await?;
            slack_ok(&resp)?;

            let t      = &resp["team"];
            let id     = t["id"].as_str().unwrap_or("—");
            let name   = t["name"].as_str().unwrap_or("—");
            let domain = t["domain"].as_str().unwrap_or("—");
            let email_domain = t["email_domain"].as_str().unwrap_or("—");
            let plan   = t["plan"].as_str().unwrap_or("—");

            Ok(format!(
                "Workspace Slack\nID       : {id}\nNom      : {name}\nDomaine  : {domain}.slack.com\nEmail    : @{email_domain}\nPlan     : {plan}"
            ))
        }

        // ── DM ────────────────────────────────────────────────────────────────
        "slack_open_dm" => {
            let user_id = args["user_id"].as_str().ok_or("Paramètre 'user_id' requis")?;

            let body = json!({ "users": user_id });
            let resp = post_json(&cfg, &cfg.api("conversations.open"), &body).await?;
            slack_ok(&resp)?;

            let channel_id = resp["channel"]["id"].as_str().unwrap_or("—");
            let already    = resp["already_open"].as_bool().unwrap_or(false);

            Ok(format!(
                "Conversation DM {} avec {user_id}.\nChannel ID : {channel_id}\n(Utiliser slack_post_message avec ce channel_id pour envoyer un message.)",
                if already { "déjà ouverte" } else { "ouverte" }
            ))
        }

        _ => Err(format!("Tool Slack inconnu: {tool}")),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Encodage URL minimal pour les paramètres de query string.
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
            ' ' => out.push('+'),
            c => {
                let encoded = c.to_string();
                for byte in encoded.as_bytes() {
                    out.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    out
}
