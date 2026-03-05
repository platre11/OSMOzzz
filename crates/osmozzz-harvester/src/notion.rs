/// Notion Harvester — indexe les pages Notion via l'API REST v1.
///
/// Config : ~/.osmozzz/notion.toml
/// ```toml
/// token = "secret_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// ```
///
/// Obtenir le token : notion.com/my-integrations → Créer une intégration → copier le secret
/// Puis partager vos pages/databases avec l'intégration dans Notion.
use chrono::DateTime;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const NOTION_API: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2022-06-28";
const MAX_PAGES: usize = 1000;

#[derive(Debug, Deserialize)]
struct NotionConfig {
    token: String,
}

impl NotionConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/notion.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct NotionHarvester;

impl NotionHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NotionHarvester {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Réponse API Notion ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: Vec<NotionObject>,
    #[serde(default)]
    next_cursor: Option<String>,
    #[serde(default)]
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct NotionObject {
    id: String,
    object: String, // "page" | "database"
    #[serde(default)]
    last_edited_time: Option<String>,
    #[serde(default)]
    properties: serde_json::Value,
    #[serde(default)]
    title: Vec<RichText>, // pour les databases
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RichText {
    #[serde(default)]
    plain_text: String,
}

#[derive(Debug, Deserialize)]
struct BlocksResponse {
    results: Vec<Block>,
    #[serde(default)]
    next_cursor: Option<String>,
    #[serde(default)]
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct Block {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    paragraph: Option<BlockContent>,
    #[serde(default)]
    heading_1: Option<BlockContent>,
    #[serde(default)]
    heading_2: Option<BlockContent>,
    #[serde(default)]
    heading_3: Option<BlockContent>,
    #[serde(default)]
    bulleted_list_item: Option<BlockContent>,
    #[serde(default)]
    numbered_list_item: Option<BlockContent>,
    #[serde(default)]
    to_do: Option<BlockContent>,
    #[serde(default)]
    toggle: Option<BlockContent>,
    #[serde(default)]
    quote: Option<BlockContent>,
    #[serde(default)]
    callout: Option<BlockContent>,
    #[serde(default)]
    code: Option<BlockContent>,
}

#[derive(Debug, Deserialize)]
struct BlockContent {
    #[serde(default)]
    rich_text: Vec<RichText>,
}

impl Block {
    fn to_text(&self) -> String {
        let content = match self.block_type.as_str() {
            "paragraph" => self.paragraph.as_ref(),
            "heading_1" => self.heading_1.as_ref(),
            "heading_2" => self.heading_2.as_ref(),
            "heading_3" => self.heading_3.as_ref(),
            "bulleted_list_item" => self.bulleted_list_item.as_ref(),
            "numbered_list_item" => self.numbered_list_item.as_ref(),
            "to_do" => self.to_do.as_ref(),
            "toggle" => self.toggle.as_ref(),
            "quote" => self.quote.as_ref(),
            "callout" => self.callout.as_ref(),
            "code" => self.code.as_ref(),
            _ => None,
        };
        content
            .map(|c| {
                c.rich_text
                    .iter()
                    .map(|rt| rt.plain_text.as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default()
    }
}

// ─── Implémentation ───────────────────────────────────────────────────────────

fn extract_page_title(obj: &NotionObject) -> String {
    // Essayer la propriété "title" (type database title)
    if !obj.title.is_empty() {
        return obj
            .title
            .iter()
            .map(|rt| rt.plain_text.as_str())
            .collect::<Vec<_>>()
            .join("");
    }

    // Propriétés de page : chercher un champ de type "title"
    if let Some(props) = obj.properties.as_object() {
        for val in props.values() {
            if val.get("type").and_then(|t| t.as_str()) == Some("title") {
                if let Some(arr) = val.get("title").and_then(|t| t.as_array()) {
                    let title: String = arr
                        .iter()
                        .filter_map(|rt| rt.get("plain_text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("");
                    if !title.is_empty() {
                        return title;
                    }
                }
            }
        }
    }

    "(sans titre)".to_string()
}

async fn fetch_blocks(
    client: &reqwest::Client,
    token: &str,
    block_id: &str,
) -> Vec<String> {
    let mut texts = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut url = format!("{}/blocks/{}/children?page_size=100", NOTION_API, block_id);
        if let Some(ref c) = cursor {
            url.push_str(&format!("&start_cursor={}", c));
        }

        let resp = match client
            .get(&url)
            .bearer_auth(token)
            .header("Notion-Version", NOTION_VERSION)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => break,
        };

        if !resp.status().is_success() {
            break;
        }

        let blocks_resp: BlocksResponse = match resp.json().await {
            Ok(r) => r,
            Err(_) => break,
        };

        for block in &blocks_resp.results {
            let text = block.to_text();
            if !text.is_empty() {
                texts.push(text);
            }
        }

        if blocks_resp.has_more {
            cursor = blocks_resp.next_cursor;
        } else {
            break;
        }
    }

    texts
}

impl osmozzz_core::Harvester for NotionHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match NotionConfig::load() {
            Some(c) => c,
            None => {
                warn!("Notion: ~/.osmozzz/notion.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        let client = reqwest::Client::new();
        let mut documents = Vec::new();
        let mut cursor: Option<String> = None;
        let mut total_fetched = 0;

        loop {
            let body = if let Some(ref c) = cursor {
                serde_json::json!({ "start_cursor": c, "page_size": 100 })
            } else {
                serde_json::json!({ "page_size": 100 })
            };

            let resp = match client
                .post(format!("{}/search", NOTION_API))
                .bearer_auth(&cfg.token)
                .header("Notion-Version", NOTION_VERSION)
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("Notion API error: {}", e);
                    break;
                }
            };

            if resp.status() == 401 {
                warn!("Notion: token invalide (401) — vérifiez ~/.osmozzz/notion.toml");
                break;
            }

            if !resp.status().is_success() {
                warn!("Notion API status: {}", resp.status());
                break;
            }

            let search_resp: SearchResponse = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Notion JSON parse error: {}", e);
                    break;
                }
            };

            for obj in &search_resp.results {
                if obj.object != "page" {
                    continue;
                }

                let title = extract_page_title(obj);
                let notion_url = obj
                    .url
                    .clone()
                    .unwrap_or_else(|| format!("https://notion.so/{}", obj.id.replace('-', "")));

                // Récupérer le contenu des blocs
                let block_texts = fetch_blocks(&client, &cfg.token, &obj.id).await;
                let body_text = block_texts.join("\n");

                let content = if body_text.is_empty() {
                    title.clone()
                } else {
                    format!("{}\n{}", title, body_text)
                };

                let checksum = checksum::compute(&content);
                let mut doc =
                    Document::new(SourceType::Notion, &notion_url, &content, &checksum)
                        .with_title(&title);

                if let Some(ref ts_str) = obj.last_edited_time {
                    if let Ok(ts) = DateTime::parse_from_rfc3339(ts_str) {
                        doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                    }
                }

                documents.push(doc);
                total_fetched += 1;
            }

            if !search_resp.has_more || total_fetched >= MAX_PAGES {
                break;
            }
            cursor = search_resp.next_cursor;
        }

        info!("Notion harvester found {} pages", documents.len());
        Ok(documents)
    }
}
