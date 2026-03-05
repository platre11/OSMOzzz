/// Slack Harvester — indexe les messages des channels Slack.
///
/// Config : ~/.osmozzz/slack.toml
/// ```toml
/// token    = "xoxp-xxxxxxxxxxxx-xxxxxxxxxxxx-xxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// channels = ["general", "random", "dev"]   # noms ou IDs
/// ```
///
/// Token : api.slack.com/apps → Create App → OAuth & Permissions
/// Scopes nécessaires (User Token) : channels:history, channels:read,
///   groups:history, groups:read, im:history, mpim:history
use chrono::TimeZone;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const SLACK_API: &str = "https://slack.com/api";
const MAX_MESSAGES_PER_CHANNEL: usize = 500;

#[derive(Debug, Deserialize)]
struct SlackConfig {
    token: String,
    #[serde(default)]
    channels: Vec<String>,
}

impl SlackConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/slack.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct SlackHarvester;

impl SlackHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SlackHarvester {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Réponse API Slack ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ChannelListResponse {
    ok: bool,
    #[serde(default)]
    channels: Vec<SlackChannel>,
    #[serde(default)]
    response_metadata: Option<ResponseMetadata>,
}

#[derive(Debug, Deserialize)]
struct ResponseMetadata {
    #[serde(default)]
    next_cursor: String,
}

#[derive(Debug, Deserialize)]
struct SlackChannel {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct HistoryResponse {
    ok: bool,
    #[serde(default)]
    messages: Vec<SlackMessage>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    response_metadata: Option<ResponseMetadata>,
}

#[derive(Debug, Deserialize)]
struct SlackMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    ts: String,
    #[serde(default)]
    username: Option<String>,
}

async fn get_channel_id(
    client: &reqwest::Client,
    token: &str,
    name_or_id: &str,
) -> Option<(String, String)> {
    // Si c'est déjà un ID (commence par C, G, D)
    if name_or_id.starts_with('C') || name_or_id.starts_with('G') || name_or_id.starts_with('D') {
        return Some((name_or_id.to_string(), name_or_id.to_string()));
    }

    let mut cursor = String::new();
    loop {
        let mut url = format!(
            "{}/conversations.list?types=public_channel,private_channel&limit=200",
            SLACK_API
        );
        if !cursor.is_empty() {
            url.push_str(&format!("&cursor={}", cursor));
        }

        let resp = client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .ok()?;

        let list: ChannelListResponse = resp.json().await.ok()?;
        if !list.ok {
            return None;
        }

        for ch in &list.channels {
            if ch.name == name_or_id {
                return Some((ch.id.clone(), ch.name.clone()));
            }
        }

        let next = list
            .response_metadata
            .as_ref()
            .map(|m| m.next_cursor.as_str())
            .unwrap_or("");
        if next.is_empty() {
            break;
        }
        cursor = next.to_string();
    }

    None
}

impl osmozzz_core::Harvester for SlackHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match SlackConfig::load() {
            Some(c) => c,
            None => {
                warn!("Slack: ~/.osmozzz/slack.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        if cfg.channels.is_empty() {
            warn!("Slack: aucun channel configuré dans slack.toml (channels = [...])");
            return Ok(vec![]);
        }

        let client = reqwest::Client::new();
        let mut documents = Vec::new();

        for channel_name in &cfg.channels {
            let (channel_id, channel_display) =
                match get_channel_id(&client, &cfg.token, channel_name).await {
                    Some(c) => c,
                    None => {
                        warn!("Slack: channel '{}' introuvable ou accès refusé", channel_name);
                        continue;
                    }
                };

            let mut cursor = String::new();
            let mut count = 0;

            loop {
                let mut url = format!(
                    "{}/conversations.history?channel={}&limit=200",
                    SLACK_API, channel_id
                );
                if !cursor.is_empty() {
                    url.push_str(&format!("&cursor={}", cursor));
                }

                let resp = match client.get(&url).bearer_auth(&cfg.token).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Slack API error: {}", e);
                        break;
                    }
                };

                let history: HistoryResponse = match resp.json().await {
                    Ok(h) => h,
                    Err(e) => {
                        warn!("Slack JSON parse error: {}", e);
                        break;
                    }
                };

                if !history.ok {
                    warn!("Slack: accès refusé au channel '{}'", channel_display);
                    break;
                }

                for msg in &history.messages {
                    if msg.msg_type != "message" || msg.text.trim().is_empty() {
                        continue;
                    }

                    let author = msg
                        .username
                        .as_deref()
                        .or(msg.user.as_deref())
                        .unwrap_or("unknown");

                    let ts_secs: f64 = msg.ts.parse().unwrap_or(0.0);
                    let ts_unix = ts_secs as i64;

                    let content = format!(
                        "[#{}] {}: {}",
                        channel_display, author, msg.text
                    );

                    let url = format!(
                        "slack://channel/{}/{}",
                        channel_id,
                        msg.ts.replace('.', "")
                    );

                    let checksum = checksum::compute(&content);
                    let mut doc =
                        Document::new(SourceType::Slack, &url, &content, &checksum);

                    if ts_unix > 0 {
                        if let Some(ts) = chrono::Utc.timestamp_opt(ts_unix, 0).single() {
                            doc = doc.with_source_ts(ts);
                        }
                    }

                    documents.push(doc);
                    count += 1;
                }

                if !history.has_more || count >= MAX_MESSAGES_PER_CHANNEL {
                    break;
                }

                let next = history
                    .response_metadata
                    .as_ref()
                    .map(|m| m.next_cursor.as_str())
                    .unwrap_or("");
                if next.is_empty() {
                    break;
                }
                cursor = next.to_string();
            }
        }

        info!("Slack harvester found {} messages", documents.len());
        Ok(documents)
    }
}
