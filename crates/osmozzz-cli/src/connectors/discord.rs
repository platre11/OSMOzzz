/// Connecteur Discord — REST API v10 officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct DiscordConfig {
    bot_token: String,
    guild_id:  Option<String>,
}

impl DiscordConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/discord.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!("https://discord.com/api/v10/{}", path.trim_start_matches('/'))
    }

    /// Résout le guild_id depuis les args ou depuis la config.
    fn resolve_guild<'a>(&'a self, args: &'a Value) -> Result<&'a str, String> {
        if let Some(gid) = args["guild_id"].as_str() {
            if !gid.is_empty() {
                return Ok(gid);
            }
        }
        self.guild_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "guild_id requis — fournir en argument ou configurer dans discord.toml".to_string())
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &DiscordConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bot {}", cfg.bot_token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &DiscordConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bot {}", cfg.bot_token))
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

async fn patch_json(cfg: &DiscordConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .patch(url)
        .header("Authorization", format!("Bot {}", cfg.bot_token))
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

async fn delete_req(cfg: &DiscordConfig, url: &str) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bot {}", cfg.bot_token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // Discord DELETE endpoints often return 204 No Content
    if resp.status().as_u16() == 204 {
        return Ok(json!({"success": true}));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

async fn delete_with_reason(cfg: &DiscordConfig, url: &str, reason: &str) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bot {}", cfg.bot_token))
        .header("Accept", "application/json")
        .header("X-Audit-Log-Reason", reason)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 204 {
        return Ok(json!({"success": true}));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

async fn put_empty(cfg: &DiscordConfig, url: &str) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .put(url)
        .header("Authorization", format!("Bot {}", cfg.bot_token))
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 204 {
        return Ok(json!({"success": true}));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

async fn put_json(cfg: &DiscordConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .put(url)
        .header("Authorization", format!("Bot {}", cfg.bot_token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

/// Envoie un message via webhook URL (pas d'auth Bot nécessaire).
async fn post_webhook(url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
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

// ─── Helpers de formatage ─────────────────────────────────────────────────────

/// Convertit une couleur hex "#RRGGBB" ou "RRGGBB" vers un entier.
fn hex_color_to_int(hex: &str) -> u32 {
    let clean = hex.trim_start_matches('#');
    u32::from_str_radix(clean, 16).unwrap_or(0)
}

/// Convertit un channel type Discord (entier) vers une description lisible.
fn channel_type_name(t: u64) -> &'static str {
    match t {
        0  => "texte",
        1  => "DM",
        2  => "vocal",
        3  => "DM groupe",
        4  => "catégorie",
        5  => "annonces",
        10 => "fil annonces",
        11 => "fil public",
        12 => "fil privé",
        13 => "stage vocal",
        14 => "répertoire",
        15 => "forum",
        _  => "inconnu",
    }
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Messages ────────────────────────────────────────────────────────
        json!({
            "name": "discord_send_message",
            "description": "DISCORD 💬 — Envoie un message dans un channel Discord. Supporte un contenu texte et optionnellement un embed (titre + description). Retourne l'id du message créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id":        { "type": "string", "description": "ID du channel cible" },
                    "content":           { "type": "string", "description": "Contenu texte du message" },
                    "embed_title":       { "type": "string", "description": "Titre de l'embed (optionnel)" },
                    "embed_description": { "type": "string", "description": "Description de l'embed (optionnel)" }
                },
                "required": ["channel_id", "content"]
            }
        }),
        json!({
            "name": "discord_edit_message",
            "description": "DISCORD 💬 — Modifie le contenu d'un message existant dans un channel Discord. Le bot doit être l'auteur du message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "message_id": { "type": "string", "description": "ID du message à modifier" },
                    "content":    { "type": "string", "description": "Nouveau contenu du message" }
                },
                "required": ["channel_id", "message_id", "content"]
            }
        }),
        json!({
            "name": "discord_delete_message",
            "description": "DISCORD 💬 — Supprime définitivement un message dans un channel Discord. Le bot doit avoir la permission MANAGE_MESSAGES.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "message_id": { "type": "string", "description": "ID du message à supprimer" }
                },
                "required": ["channel_id", "message_id"]
            }
        }),
        json!({
            "name": "discord_get_message",
            "description": "DISCORD 💬 — Récupère le détail d'un message Discord : auteur, contenu, date, reactions. Utiliser discord_list_messages pour obtenir un message_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "message_id": { "type": "string", "description": "ID du message" }
                },
                "required": ["channel_id", "message_id"]
            }
        }),
        json!({
            "name": "discord_list_messages",
            "description": "DISCORD 💬 — Liste les derniers messages d'un channel Discord avec auteur, contenu et date. Retourne 50 messages par défaut (max 100).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "limit":      { "type": "integer", "description": "Nombre de messages à retourner (défaut: 50, max: 100)", "default": 50, "minimum": 1, "maximum": 100 }
                },
                "required": ["channel_id"]
            }
        }),
        // ── Channels ────────────────────────────────────────────────────────
        json!({
            "name": "discord_list_channels",
            "description": "DISCORD 💬 — Liste tous les channels d'un serveur Discord avec leur nom, type et catégorie parente. guild_id optionnel si configuré dans discord.toml.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" }
                }
            }
        }),
        json!({
            "name": "discord_get_channel",
            "description": "DISCORD 💬 — Récupère les détails d'un channel Discord : nom, type, sujet, nombre de membres. Utiliser discord_list_channels pour obtenir le channel_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" }
                },
                "required": ["channel_id"]
            }
        }),
        json!({
            "name": "discord_create_channel",
            "description": "DISCORD 💬 — Crée un nouveau channel dans un serveur Discord. Types disponibles : text, voice, category. Retourne l'id du channel créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" },
                    "name":     { "type": "string", "description": "Nom du channel" },
                    "type":     { "type": "string", "description": "Type : text (défaut), voice, category", "enum": ["text", "voice", "category"] },
                    "topic":    { "type": "string", "description": "Sujet du channel (optionnel, texte seulement)" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "discord_edit_channel",
            "description": "DISCORD 💬 — Modifie le nom et/ou le sujet d'un channel Discord. Le bot doit avoir la permission MANAGE_CHANNELS.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel à modifier" },
                    "name":       { "type": "string", "description": "Nouveau nom (optionnel)" },
                    "topic":      { "type": "string", "description": "Nouveau sujet (optionnel)" }
                },
                "required": ["channel_id"]
            }
        }),
        json!({
            "name": "discord_delete_channel",
            "description": "DISCORD 💬 — Supprime définitivement un channel Discord. Action irréversible. Le bot doit avoir la permission MANAGE_CHANNELS.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel à supprimer" }
                },
                "required": ["channel_id"]
            }
        }),
        // ── Members ─────────────────────────────────────────────────────────
        json!({
            "name": "discord_list_members",
            "description": "DISCORD 💬 — Liste les membres d'un serveur Discord avec leur username, surnom et rôles. Retourne 100 membres par défaut (max 1000).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" },
                    "limit":    { "type": "integer", "description": "Nombre de membres à retourner (défaut: 100, max: 1000)", "default": 100, "minimum": 1, "maximum": 1000 }
                }
            }
        }),
        json!({
            "name": "discord_get_member",
            "description": "DISCORD 💬 — Récupère les détails d'un membre Discord : username, surnom, rôles, date d'arrivée sur le serveur.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" },
                    "user_id":  { "type": "string", "description": "ID de l'utilisateur Discord" }
                },
                "required": ["user_id"]
            }
        }),
        json!({
            "name": "discord_kick_member",
            "description": "DISCORD 💬 — Exclut (kick) un membre d'un serveur Discord. Le bot doit avoir la permission KICK_MEMBERS.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" },
                    "user_id":  { "type": "string", "description": "ID de l'utilisateur à kick" },
                    "reason":   { "type": "string", "description": "Raison du kick (optionnel, visible dans le journal d'audit)" }
                },
                "required": ["user_id"]
            }
        }),
        // ── Roles ────────────────────────────────────────────────────────────
        json!({
            "name": "discord_list_roles",
            "description": "DISCORD 💬 — Liste tous les rôles d'un serveur Discord avec leur nom, couleur et permissions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" }
                }
            }
        }),
        json!({
            "name": "discord_create_role",
            "description": "DISCORD 💬 — Crée un nouveau rôle dans un serveur Discord. Retourne l'id du rôle créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" },
                    "name":     { "type": "string", "description": "Nom du rôle" },
                    "color":    { "type": "string", "description": "Couleur hex du rôle (ex: #ff0000, optionnel)" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "discord_add_role_to_member",
            "description": "DISCORD 💬 — Attribue un rôle à un membre d'un serveur Discord. Le bot doit avoir la permission MANAGE_ROLES.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" },
                    "user_id":  { "type": "string", "description": "ID de l'utilisateur" },
                    "role_id":  { "type": "string", "description": "ID du rôle à attribuer" }
                },
                "required": ["user_id", "role_id"]
            }
        }),
        json!({
            "name": "discord_remove_role_from_member",
            "description": "DISCORD 💬 — Retire un rôle à un membre d'un serveur Discord. Le bot doit avoir la permission MANAGE_ROLES.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" },
                    "user_id":  { "type": "string", "description": "ID de l'utilisateur" },
                    "role_id":  { "type": "string", "description": "ID du rôle à retirer" }
                },
                "required": ["user_id", "role_id"]
            }
        }),
        // ── Webhooks ─────────────────────────────────────────────────────────
        json!({
            "name": "discord_list_webhooks",
            "description": "DISCORD 💬 — Liste tous les webhooks d'un channel Discord avec leur nom, id et URL. Utiliser l'URL pour discord_send_webhook_message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" }
                },
                "required": ["channel_id"]
            }
        }),
        json!({
            "name": "discord_create_webhook",
            "description": "DISCORD 💬 — Crée un webhook dans un channel Discord. Retourne l'URL du webhook à utiliser avec discord_send_webhook_message.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id": { "type": "string", "description": "ID du channel" },
                    "name":       { "type": "string", "description": "Nom du webhook (affiché comme auteur des messages)" }
                },
                "required": ["channel_id", "name"]
            }
        }),
        json!({
            "name": "discord_send_webhook_message",
            "description": "DISCORD 💬 — Envoie un message via un webhook Discord (sans nécessiter d'auth Bot). Utiliser discord_list_webhooks ou discord_create_webhook pour obtenir l'URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "webhook_url": { "type": "string", "description": "URL complète du webhook Discord" },
                    "content":     { "type": "string", "description": "Contenu du message" },
                    "username":    { "type": "string", "description": "Nom d'affichage override (optionnel)" }
                },
                "required": ["webhook_url", "content"]
            }
        }),
        // ── Threads ─────────────────────────────────────────────────────────
        json!({
            "name": "discord_create_thread",
            "description": "DISCORD 💬 — Crée un fil de discussion (thread) dans un channel Discord, optionnellement attaché à un message existant. auto_archive_duration en minutes : 60, 1440, 4320, 10080.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel_id":            { "type": "string", "description": "ID du channel" },
                    "message_id":            { "type": "string", "description": "ID du message source pour créer le thread (optionnel — si absent, crée un thread standalone)" },
                    "name":                  { "type": "string", "description": "Nom du thread" },
                    "auto_archive_duration": { "type": "integer", "description": "Archivage automatique en minutes : 60, 1440 (défaut), 4320, 10080", "default": 1440 }
                },
                "required": ["channel_id", "name"]
            }
        }),
        json!({
            "name": "discord_list_active_threads",
            "description": "DISCORD 💬 — Liste tous les fils de discussion actifs d'un serveur Discord avec leur nom, channel parent et nombre de membres.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" }
                }
            }
        }),
        // ── Server overview & onboarding ────────────────────────────────────
        json!({
            "name": "discord_get_guild",
            "description": "DISCORD 🏠 — Vue complète d'un serveur Discord : nom, propriétaire, nombre de membres, niveau de boost, vérification, canaux système/règles, features activées.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" }
                }
            }
        }),
        json!({
            "name": "discord_get_onboarding",
            "description": "DISCORD 🎯 — Lit la configuration complète de l'onboarding d'un serveur : prompts (questions), options (réponses), rôles et canaux assignés, mode et statut activé/désactivé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" }
                }
            }
        }),
        json!({
            "name": "discord_update_onboarding",
            "description": "DISCORD 🎯 — Met à jour la configuration d'onboarding : activer/désactiver, changer le mode (0=DEFAULT, 1=ADVANCED), modifier les prompts et canaux par défaut.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id":            { "type": "string", "description": "ID du serveur" },
                    "enabled":             { "type": "boolean", "description": "Activer ou désactiver l'onboarding" },
                    "mode":                { "type": "integer", "description": "0 = DEFAULT, 1 = ADVANCED" },
                    "default_channel_ids": { "type": "array", "items": { "type": "string" }, "description": "IDs des canaux visibles par défaut" },
                    "prompts":             { "type": "array", "description": "Tableau complet des prompts (remplace l'existant)" }
                }
            }
        }),
        json!({
            "name": "discord_get_welcome_screen",
            "description": "DISCORD 👋 — Lit l'écran de bienvenue d'un serveur : description et canaux mis en avant avec leurs emojis.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" }
                }
            }
        }),
        json!({
            "name": "discord_update_welcome_screen",
            "description": "DISCORD 👋 — Met à jour l'écran de bienvenue : description et liste des canaux mis en avant (max 5) avec emoji optionnel.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id":        { "type": "string", "description": "ID du serveur" },
                    "description":     { "type": "string", "description": "Texte affiché sur l'écran de bienvenue" },
                    "welcome_channels": {
                        "type": "array",
                        "description": "Canaux à mettre en avant (max 5). Chaque entrée : {channel_id, description, emoji_name?}",
                        "items": { "type": "object" }
                    }
                },
                "required": ["guild_id"]
            }
        }),
        json!({
            "name": "discord_get_member_verification",
            "description": "DISCORD 📋 — Lit les règles d'adhésion du serveur (Member Screening) : description, règles à accepter avant d'accéder au serveur.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "guild_id": { "type": "string", "description": "ID du serveur (optionnel si configuré dans discord.toml)" }
                }
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = DiscordConfig::load()
        .ok_or_else(|| "Discord non configuré — créer ~/.osmozzz/discord.toml avec bot_token".to_string())?;

    match name {
        // ── Messages ────────────────────────────────────────────────────────
        "discord_send_message" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let content    = args["content"].as_str().ok_or("Paramètre 'content' requis")?;

            let mut body = json!({ "content": content });

            if let (Some(title), Some(desc)) = (
                args["embed_title"].as_str(),
                args["embed_description"].as_str(),
            ) {
                body["embeds"] = json!([{ "title": title, "description": desc }]);
            } else if let Some(title) = args["embed_title"].as_str() {
                body["embeds"] = json!([{ "title": title }]);
            } else if let Some(desc) = args["embed_description"].as_str() {
                body["embeds"] = json!([{ "description": desc }]);
            }

            let url  = cfg.api(&format!("channels/{channel_id}/messages"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id         = resp["id"].as_str().unwrap_or("—");
            let author     = resp["author"]["username"].as_str().unwrap_or("—");
            let timestamp  = resp["timestamp"].as_str().unwrap_or("—");
            Ok(format!(
                "Message envoyé.\nID      : {id}\nChannel : {channel_id}\nAuteur  : {author}\nDate    : {timestamp}"
            ))
        }

        "discord_edit_message" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let message_id = args["message_id"].as_str().ok_or("Paramètre 'message_id' requis")?;
            let content    = args["content"].as_str().ok_or("Paramètre 'content' requis")?;

            let body = json!({ "content": content });
            let url  = cfg.api(&format!("channels/{channel_id}/messages/{message_id}"));
            let resp = patch_json(&cfg, &url, &body).await?;

            let id        = resp["id"].as_str().unwrap_or(message_id);
            let edited_at = resp["edited_timestamp"].as_str().unwrap_or("—");
            Ok(format!("Message {id} modifié. Modifié le : {edited_at}"))
        }

        "discord_delete_message" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let message_id = args["message_id"].as_str().ok_or("Paramètre 'message_id' requis")?;

            let url = cfg.api(&format!("channels/{channel_id}/messages/{message_id}"));
            delete_req(&cfg, &url).await?;
            Ok(format!("Message {message_id} supprimé du channel {channel_id}."))
        }

        "discord_get_message" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let message_id = args["message_id"].as_str().ok_or("Paramètre 'message_id' requis")?;

            let url  = cfg.api(&format!("channels/{channel_id}/messages/{message_id}"));
            let resp = get(&cfg, &url).await?;

            let id        = resp["id"].as_str().unwrap_or(message_id);
            let author    = resp["author"]["username"].as_str().unwrap_or("—");
            let content   = resp["content"].as_str().unwrap_or("(vide)");
            let timestamp = resp["timestamp"].as_str().unwrap_or("—");

            let reactions: String = resp["reactions"].as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|r| {
                            let emoji = r["emoji"]["name"].as_str().unwrap_or("?");
                            let count = r["count"].as_u64().unwrap_or(0);
                            format!("{emoji}×{count}")
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();

            let mut out = format!(
                "Message {id}\nAuteur    : {author}\nDate      : {timestamp}\nContenu   : {content}"
            );
            if !reactions.is_empty() {
                out.push_str(&format!("\nRéactions : {reactions}"));
            }
            Ok(out)
        }

        "discord_list_messages" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let limit      = args["limit"].as_u64().unwrap_or(50).min(100);

            let url  = cfg.api(&format!("channels/{channel_id}/messages?limit={limit}"));
            let resp = get(&cfg, &url).await?;

            let messages = resp.as_array().cloned().unwrap_or_default();
            if messages.is_empty() {
                return Ok(format!("Aucun message dans le channel {channel_id}."));
            }

            let mut out = format!("{} message(s) dans #{channel_id} :\n", messages.len());
            for m in &messages {
                let id        = m["id"].as_str().unwrap_or("—");
                let author    = m["author"]["username"].as_str().unwrap_or("—");
                let content   = m["content"].as_str().unwrap_or("(embed/attachment)");
                let timestamp = m["timestamp"].as_str().unwrap_or("—");
                // Truncate long messages for readability
                let preview   = if content.len() > 120 { &content[..120] } else { content };
                out.push_str(&format!("[{timestamp}] {author} ({id}): {preview}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Channels ────────────────────────────────────────────────────────
        "discord_list_channels" => {
            let guild_id = cfg.resolve_guild(args)?;
            let url      = cfg.api(&format!("guilds/{guild_id}/channels"));
            let resp     = get(&cfg, &url).await?;

            let channels = resp.as_array().cloned().unwrap_or_default();
            if channels.is_empty() {
                return Ok(format!("Aucun channel dans le serveur {guild_id}."));
            }

            let mut out = format!("{} channel(s) :\n", channels.len());
            for c in &channels {
                let id       = c["id"].as_str().unwrap_or("—");
                let cname    = c["name"].as_str().unwrap_or("—");
                let ctype    = c["type"].as_u64().unwrap_or(0);
                let type_str = channel_type_name(ctype);
                let topic    = c["topic"].as_str().unwrap_or("");
                if topic.is_empty() {
                    out.push_str(&format!("• [{id}] #{cname} ({type_str})\n"));
                } else {
                    out.push_str(&format!("• [{id}] #{cname} ({type_str}) — {topic}\n"));
                }
            }
            Ok(out.trim_end().to_string())
        }

        "discord_get_channel" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let url  = cfg.api(&format!("channels/{channel_id}"));
            let resp = get(&cfg, &url).await?;

            let id       = resp["id"].as_str().unwrap_or(channel_id);
            let cname    = resp["name"].as_str().unwrap_or("—");
            let ctype    = resp["type"].as_u64().unwrap_or(0);
            let type_str = channel_type_name(ctype);
            let topic    = resp["topic"].as_str().unwrap_or("—");
            let position = resp["position"].as_u64().unwrap_or(0);

            Ok(format!(
                "Channel {id}\nNom      : #{cname}\nType     : {type_str}\nSujet    : {topic}\nPosition : {position}"
            ))
        }

        "discord_create_channel" => {
            let guild_id = cfg.resolve_guild(args)?;
            let cname    = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let ctype_str = args["type"].as_str().unwrap_or("text");
            let channel_type: u64 = match ctype_str {
                "voice"    => 2,
                "category" => 4,
                _          => 0, // text par défaut
            };

            let mut body = json!({ "name": cname, "type": channel_type });
            if let Some(topic) = args["topic"].as_str() {
                body["topic"] = json!(topic);
            }

            let url  = cfg.api(&format!("guilds/{guild_id}/channels"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id       = resp["id"].as_str().unwrap_or("—");
            let type_str = channel_type_name(channel_type);
            Ok(format!(
                "Channel créé.\nID   : {id}\nNom  : #{cname}\nType : {type_str}\nServeur : {guild_id}"
            ))
        }

        "discord_edit_channel" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let mut body   = json!({});

            if let Some(n) = args["name"].as_str()  { body["name"]  = json!(n); }
            if let Some(t) = args["topic"].as_str() { body["topic"] = json!(t); }

            if body.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                return Err("Au moins un paramètre parmi 'name' ou 'topic' est requis".to_string());
            }

            let url  = cfg.api(&format!("channels/{channel_id}"));
            let resp = patch_json(&cfg, &url, &body).await?;

            let id    = resp["id"].as_str().unwrap_or(channel_id);
            let cname = resp["name"].as_str().unwrap_or("—");
            Ok(format!("Channel {id} modifié. Nouveau nom : #{cname}"))
        }

        "discord_delete_channel" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let url = cfg.api(&format!("channels/{channel_id}"));
            delete_req(&cfg, &url).await?;
            Ok(format!("Channel {channel_id} supprimé."))
        }

        // ── Members ─────────────────────────────────────────────────────────
        "discord_list_members" => {
            let guild_id = cfg.resolve_guild(args)?;
            let limit    = args["limit"].as_u64().unwrap_or(100).min(1000);
            let url      = cfg.api(&format!("guilds/{guild_id}/members?limit={limit}"));
            let resp     = get(&cfg, &url).await?;

            let members = resp.as_array().cloned().unwrap_or_default();
            if members.is_empty() {
                return Ok(format!("Aucun membre dans le serveur {guild_id}."));
            }

            let mut out = format!("{} membre(s) :\n", members.len());
            for m in &members {
                let uid      = m["user"]["id"].as_str().unwrap_or("—");
                let username = m["user"]["username"].as_str().unwrap_or("—");
                let nickname = m["nick"].as_str().unwrap_or("");
                let joined   = m["joined_at"].as_str().unwrap_or("—");
                let roles_count = m["roles"].as_array().map(|r| r.len()).unwrap_or(0);
                if nickname.is_empty() {
                    out.push_str(&format!("• [{uid}] {username} — {roles_count} rôle(s) — arrivé: {joined}\n"));
                } else {
                    out.push_str(&format!("• [{uid}] {username} ({nickname}) — {roles_count} rôle(s) — arrivé: {joined}\n"));
                }
            }
            Ok(out.trim_end().to_string())
        }

        "discord_get_member" => {
            let guild_id = cfg.resolve_guild(args)?;
            let user_id  = args["user_id"].as_str().ok_or("Paramètre 'user_id' requis")?;
            let url      = cfg.api(&format!("guilds/{guild_id}/members/{user_id}"));
            let resp     = get(&cfg, &url).await?;

            let username = resp["user"]["username"].as_str().unwrap_or("—");
            let global_name = resp["user"]["global_name"].as_str().unwrap_or("—");
            let nickname = resp["nick"].as_str().unwrap_or("—");
            let joined   = resp["joined_at"].as_str().unwrap_or("—");
            let roles: Vec<&str> = resp["roles"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            Ok(format!(
                "Membre {user_id}\nUsername    : {username}\nNom global  : {global_name}\nSurnom      : {nickname}\nArrivé le   : {joined}\nRôles ({n}) : {roles}",
                n     = roles.len(),
                roles = if roles.is_empty() { "aucun".to_string() } else { roles.join(", ") }
            ))
        }

        "discord_kick_member" => {
            let guild_id = cfg.resolve_guild(args)?;
            let user_id  = args["user_id"].as_str().ok_or("Paramètre 'user_id' requis")?;
            let reason   = args["reason"].as_str().unwrap_or("Kicked via OSMOzzz");
            let url      = cfg.api(&format!("guilds/{guild_id}/members/{user_id}"));
            delete_with_reason(&cfg, &url, reason).await?;
            Ok(format!("Membre {user_id} kické du serveur {guild_id}. Raison : {reason}"))
        }

        // ── Roles ────────────────────────────────────────────────────────────
        "discord_list_roles" => {
            let guild_id = cfg.resolve_guild(args)?;
            let url      = cfg.api(&format!("guilds/{guild_id}/roles"));
            let resp     = get(&cfg, &url).await?;

            let roles = resp.as_array().cloned().unwrap_or_default();
            if roles.is_empty() {
                return Ok(format!("Aucun rôle dans le serveur {guild_id}."));
            }

            let mut out = format!("{} rôle(s) :\n", roles.len());
            for r in &roles {
                let id    = r["id"].as_str().unwrap_or("—");
                let rname = r["name"].as_str().unwrap_or("—");
                let color = r["color"].as_u64().unwrap_or(0);
                let color_str = if color > 0 { format!("#{:06X}", color) } else { "—".to_string() };
                let managed = r["managed"].as_bool().unwrap_or(false);
                let managed_str = if managed { " [bot]" } else { "" };
                out.push_str(&format!("• [{id}] {rname}{managed_str} — couleur: {color_str}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "discord_create_role" => {
            let guild_id = cfg.resolve_guild(args)?;
            let rname    = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let color    = args["color"].as_str()
                .map(hex_color_to_int)
                .unwrap_or(0);

            let body = json!({ "name": rname, "color": color });
            let url  = cfg.api(&format!("guilds/{guild_id}/roles"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id          = resp["id"].as_str().unwrap_or("—");
            let color_val   = resp["color"].as_u64().unwrap_or(0);
            let color_str   = if color_val > 0 { format!("#{:06X}", color_val) } else { "—".to_string() };
            Ok(format!(
                "Rôle créé.\nID      : {id}\nNom     : {rname}\nCouleur : {color_str}\nServeur : {guild_id}"
            ))
        }

        "discord_add_role_to_member" => {
            let guild_id = cfg.resolve_guild(args)?;
            let user_id  = args["user_id"].as_str().ok_or("Paramètre 'user_id' requis")?;
            let role_id  = args["role_id"].as_str().ok_or("Paramètre 'role_id' requis")?;
            let url      = cfg.api(&format!("guilds/{guild_id}/members/{user_id}/roles/{role_id}"));
            put_empty(&cfg, &url).await?;
            Ok(format!("Rôle {role_id} attribué à l'utilisateur {user_id} sur le serveur {guild_id}."))
        }

        "discord_remove_role_from_member" => {
            let guild_id = cfg.resolve_guild(args)?;
            let user_id  = args["user_id"].as_str().ok_or("Paramètre 'user_id' requis")?;
            let role_id  = args["role_id"].as_str().ok_or("Paramètre 'role_id' requis")?;
            let url      = cfg.api(&format!("guilds/{guild_id}/members/{user_id}/roles/{role_id}"));
            delete_req(&cfg, &url).await?;
            Ok(format!("Rôle {role_id} retiré de l'utilisateur {user_id} sur le serveur {guild_id}."))
        }

        // ── Webhooks ─────────────────────────────────────────────────────────
        "discord_list_webhooks" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let url  = cfg.api(&format!("channels/{channel_id}/webhooks"));
            let resp = get(&cfg, &url).await?;

            let webhooks = resp.as_array().cloned().unwrap_or_default();
            if webhooks.is_empty() {
                return Ok(format!("Aucun webhook dans le channel {channel_id}."));
            }

            let mut out = format!("{} webhook(s) dans #{channel_id} :\n", webhooks.len());
            for w in &webhooks {
                let id    = w["id"].as_str().unwrap_or("—");
                let wname = w["name"].as_str().unwrap_or("—");
                let token = w["token"].as_str().unwrap_or("");
                let wurl  = if token.is_empty() {
                    "—".to_string()
                } else {
                    format!("https://discord.com/api/webhooks/{id}/{token}")
                };
                out.push_str(&format!("• [{id}] {wname} — URL: {wurl}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "discord_create_webhook" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let wname      = args["name"].as_str().ok_or("Paramètre 'name' requis")?;

            let body = json!({ "name": wname });
            let url  = cfg.api(&format!("channels/{channel_id}/webhooks"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id    = resp["id"].as_str().unwrap_or("—");
            let token = resp["token"].as_str().unwrap_or("");
            let wurl  = if token.is_empty() {
                "—".to_string()
            } else {
                format!("https://discord.com/api/webhooks/{id}/{token}")
            };

            Ok(format!(
                "Webhook créé.\nID      : {id}\nNom     : {wname}\nChannel : {channel_id}\nURL     : {wurl}"
            ))
        }

        "discord_send_webhook_message" => {
            let webhook_url = args["webhook_url"].as_str().ok_or("Paramètre 'webhook_url' requis")?;
            let content     = args["content"].as_str().ok_or("Paramètre 'content' requis")?;

            let mut body = json!({ "content": content });
            if let Some(username) = args["username"].as_str() {
                body["username"] = json!(username);
            }

            let resp = post_webhook(webhook_url, &body).await?;

            let id        = resp["id"].as_str().unwrap_or("—");
            let author    = resp["author"]["username"].as_str().unwrap_or("webhook");
            let timestamp = resp["timestamp"].as_str().unwrap_or("—");
            Ok(format!(
                "Message envoyé via webhook.\nID     : {id}\nAuteur : {author}\nDate   : {timestamp}"
            ))
        }

        // ── Threads ─────────────────────────────────────────────────────────
        "discord_create_thread" => {
            let channel_id = args["channel_id"].as_str().ok_or("Paramètre 'channel_id' requis")?;
            let tname      = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let auto_arc   = args["auto_archive_duration"].as_u64().unwrap_or(1440);

            let (url, body) = if let Some(message_id) = args["message_id"].as_str() {
                // Thread attaché à un message existant
                let u = cfg.api(&format!("channels/{channel_id}/messages/{message_id}/threads"));
                let b = json!({ "name": tname, "auto_archive_duration": auto_arc });
                (u, b)
            } else {
                // Thread standalone (type 11 = GUILD_PUBLIC_THREAD)
                let u = cfg.api(&format!("channels/{channel_id}/threads"));
                let b = json!({ "name": tname, "auto_archive_duration": auto_arc, "type": 11 });
                (u, b)
            };

            let resp = post_json(&cfg, &url, &body).await?;

            let id         = resp["id"].as_str().unwrap_or("—");
            let owner      = resp["owner_id"].as_str().unwrap_or("—");
            let member_cnt = resp["member_count"].as_u64().unwrap_or(0);
            Ok(format!(
                "Thread créé.\nID              : {id}\nNom             : {tname}\nChannel parent  : {channel_id}\nOwner           : {owner}\nMembres         : {member_cnt}\nAuto-archive    : {auto_arc} min"
            ))
        }

        "discord_list_active_threads" => {
            let guild_id = cfg.resolve_guild(args)?;
            let url      = cfg.api(&format!("guilds/{guild_id}/threads/active"));
            let resp     = get(&cfg, &url).await?;

            let threads = resp["threads"].as_array().cloned().unwrap_or_default();
            if threads.is_empty() {
                return Ok(format!("Aucun thread actif dans le serveur {guild_id}."));
            }

            let mut out = format!("{} thread(s) actif(s) :\n", threads.len());
            for t in &threads {
                let id         = t["id"].as_str().unwrap_or("—");
                let tname      = t["name"].as_str().unwrap_or("—");
                let parent_id  = t["parent_id"].as_str().unwrap_or("—");
                let msg_count  = t["message_count"].as_u64().unwrap_or(0);
                let member_cnt = t["member_count"].as_u64().unwrap_or(0);
                out.push_str(&format!(
                    "• [{id}] {tname} — parent: #{parent_id} — {msg_count} msgs — {member_cnt} membres\n"
                ));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Server overview ──────────────────────────────────────────────────
        "discord_get_guild" => {
            let guild_id = cfg.resolve_guild(args)?;
            let url  = cfg.api(&format!("guilds/{guild_id}?with_counts=true"));
            let resp = get(&cfg, &url).await?;

            let name         = resp["name"].as_str().unwrap_or("—");
            let owner_id     = resp["owner_id"].as_str().unwrap_or("—");
            let members      = resp["approximate_member_count"].as_u64().unwrap_or(0);
            let online       = resp["approximate_presence_count"].as_u64().unwrap_or(0);
            let verif        = resp["verification_level"].as_u64().unwrap_or(0);
            let verif_str    = match verif { 0=>"Aucune", 1=>"Low", 2=>"Medium", 3=>"High", 4=>"Very High", _=>"—" };
            let boost_tier   = resp["premium_tier"].as_u64().unwrap_or(0);
            let boosts       = resp["premium_subscription_count"].as_u64().unwrap_or(0);
            let locale       = resp["preferred_locale"].as_str().unwrap_or("—");
            let features: Vec<&str> = resp["features"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let system_ch    = resp["system_channel_id"].as_str().unwrap_or("—");
            let rules_ch     = resp["rules_channel_id"].as_str().unwrap_or("—");
            let description  = resp["description"].as_str().unwrap_or("—");

            Ok(format!(
                "Serveur : {name} (ID: {guild_id})\n\
                Propriétaire   : {owner_id}\n\
                Membres        : {members} ({online} en ligne)\n\
                Vérification   : {verif_str}\n\
                Boost          : Tier {boost_tier} ({boosts} boosts)\n\
                Langue         : {locale}\n\
                Canal système  : {system_ch}\n\
                Canal règles   : {rules_ch}\n\
                Description    : {description}\n\
                Features       : {features}",
                features = if features.is_empty() { "aucune".to_string() } else { features.join(", ") }
            ))
        }

        // ── Onboarding ───────────────────────────────────────────────────────
        "discord_get_onboarding" => {
            let guild_id = cfg.resolve_guild(args)?;
            let url  = cfg.api(&format!("guilds/{guild_id}/onboarding"));
            let resp = get(&cfg, &url).await?;

            let enabled  = resp["enabled"].as_bool().unwrap_or(false);
            let mode     = resp["mode"].as_u64().unwrap_or(0);
            let mode_str = if mode == 0 { "DEFAULT (obligatoire)" } else { "ADVANCED (personnalisé)" };
            let default_channels: Vec<&str> = resp["default_channel_ids"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            let mut out = format!(
                "Onboarding du serveur {guild_id}\n\
                Activé : {enabled}\n\
                Mode   : {mode_str}\n\
                Canaux par défaut ({n}) : {channels}\n\n\
                Prompts :\n",
                n        = default_channels.len(),
                channels = if default_channels.is_empty() { "aucun".to_string() } else { default_channels.join(", ") }
            );

            if let Some(prompts) = resp["prompts"].as_array() {
                if prompts.is_empty() {
                    out.push_str("  (aucun prompt configuré)\n");
                }
                for (i, p) in prompts.iter().enumerate() {
                    let title    = p["title"].as_str().unwrap_or("—");
                    let required = p["required"].as_bool().unwrap_or(false);
                    let single   = p["single_select"].as_bool().unwrap_or(false);
                    let ptype    = if single { "choix unique" } else { "choix multiple" };
                    out.push_str(&format!("\n[{}] {} ({}{})\n", i+1, title, ptype, if required { ", obligatoire" } else { "" }));
                    if let Some(options) = p["options"].as_array() {
                        for opt in options {
                            let label       = opt["title"].as_str().unwrap_or("—");
                            let description = opt["description"].as_str().unwrap_or("");
                            let role_ids: Vec<&str> = opt["role_ids"].as_array()
                                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                                .unwrap_or_default();
                            let chan_ids: Vec<&str> = opt["channel_ids"].as_array()
                                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                                .unwrap_or_default();
                            out.push_str(&format!(
                                "  • {label}{desc} → rôles: [{roles}] canaux: [{chans}]\n",
                                desc  = if description.is_empty() { "".to_string() } else { format!(" — {}", description) },
                                roles = role_ids.join(", "),
                                chans = chan_ids.join(", "),
                            ));
                        }
                    }
                }
            }
            Ok(out.trim_end().to_string())
        }

        "discord_update_onboarding" => {
            let guild_id = cfg.resolve_guild(args)?;
            let url      = cfg.api(&format!("guilds/{guild_id}/onboarding"));

            // Lire la config actuelle pour merger
            let current  = get(&cfg, &url).await?;
            let mut body = current.clone();

            if let Some(enabled) = args["enabled"].as_bool() {
                body["enabled"] = json!(enabled);
            }
            if let Some(mode) = args["mode"].as_u64() {
                body["mode"] = json!(mode);
            }
            // prompts_json: tableau JSON des prompts (optionnel — remplace entièrement)
            if !args["prompts"].is_null() {
                body["prompts"] = args["prompts"].clone();
            }
            if let Some(channels) = args["default_channel_ids"].as_array() {
                body["default_channel_ids"] = json!(channels);
            }

            let resp = put_json(&cfg, &url, &body).await?;
            let enabled = resp["enabled"].as_bool().unwrap_or(false);
            Ok(format!("Onboarding mis à jour — activé: {enabled}"))
        }

        // ── Welcome screen ───────────────────────────────────────────────────
        "discord_get_welcome_screen" => {
            let guild_id    = cfg.resolve_guild(args)?;
            let url         = cfg.api(&format!("guilds/{guild_id}/welcome-screen"));
            let resp        = get(&cfg, &url).await?;

            let description = resp["description"].as_str().unwrap_or("(aucune description)");
            let mut out     = format!("Écran de bienvenue\nDescription : {description}\n\nCanaux mis en avant :\n");

            if let Some(channels) = resp["welcome_channels"].as_array() {
                for ch in channels {
                    let id          = ch["channel_id"].as_str().unwrap_or("—");
                    let desc        = ch["description"].as_str().unwrap_or("—");
                    let emoji_name  = ch["emoji_name"].as_str().unwrap_or("");
                    out.push_str(&format!("  • #{id} {emoji} — {desc}\n",
                        emoji = if emoji_name.is_empty() { "".to_string() } else { format!("({})", emoji_name) }
                    ));
                }
            }
            Ok(out.trim_end().to_string())
        }

        "discord_update_welcome_screen" => {
            let guild_id    = cfg.resolve_guild(args)?;
            let url         = cfg.api(&format!("guilds/{guild_id}/welcome-screen"));
            let description = args["description"].as_str().unwrap_or("");

            // welcome_channels: array of {channel_id, description, emoji_name?}
            let channels    = args["welcome_channels"].as_array().cloned().unwrap_or_default();

            let body = json!({
                "enabled":         true,
                "description":     description,
                "welcome_channels": channels
            });
            patch_json(&cfg, &url, &body).await?;
            Ok(format!("Écran de bienvenue mis à jour ({} canal(s) mis en avant)", channels.len()))
        }

        // ── Member verification (rules) ──────────────────────────────────────
        "discord_get_member_verification" => {
            let guild_id = cfg.resolve_guild(args)?;
            let url      = cfg.api(&format!("guilds/{guild_id}/member-verification"));
            let resp     = get(&cfg, &url).await?;

            let description = resp["description"].as_str().unwrap_or("(aucune)");
            let version     = resp["version"].as_str().unwrap_or("—");
            let mut out     = format!("Vérification membres (règles)\nVersion     : {version}\nDescription : {description}\n\nRègles :\n");

            if let Some(fields) = resp["form_fields"].as_array() {
                for (i, f) in fields.iter().enumerate() {
                    let ftype    = f["field_type"].as_str().unwrap_or("—");
                    let label    = f["label"].as_str().unwrap_or("—");
                    let required = f["required"].as_bool().unwrap_or(false);
                    let values: Vec<&str> = f["values"].as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    out.push_str(&format!("[{}] {} ({}{}) :\n", i+1, label, ftype, if required { ", requis" } else { "" }));
                    for v in &values {
                        out.push_str(&format!("  • {v}\n"));
                    }
                }
            }
            Ok(out.trim_end().to_string())
        }

        _ => Err(format!("Tool Discord inconnu : {name}")),
    }
}
