/// Trello Harvester — indexe les cartes de vos boards Trello.
///
/// Config : ~/.osmozzz/trello.toml
/// ```toml
/// api_key = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// token   = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// ```
///
/// Clé API : trello.com/power-ups/admin → "Manage your Power-Ups" → API key
/// Token : généré via https://trello.com/1/authorize?key=VOTRE_KEY&scope=read&expiration=never&response_type=token
use chrono::DateTime;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const TRELLO_API: &str = "https://api.trello.com/1";

#[derive(Debug, Deserialize)]
struct TrelloConfig {
    api_key: String,
    token: String,
}

impl TrelloConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/trello.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct TrelloHarvester;

impl TrelloHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TrelloHarvester {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct TrelloBoard {
    id: String,
    name: String,
    #[serde(default)]
    closed: bool,
}

#[derive(Debug, Deserialize)]
struct TrelloCard {
    id: String,
    name: String,
    #[serde(default)]
    desc: String,
    #[serde(rename = "idList")]
    id_list: String,
    #[serde(default)]
    closed: bool,
    #[serde(rename = "dateLastActivity")]
    date_last_activity: Option<String>,
    #[serde(rename = "shortUrl")]
    short_url: String,
    #[serde(default)]
    labels: Vec<TrelloLabel>,
}

#[derive(Debug, Deserialize)]
struct TrelloLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct TrelloList {
    id: String,
    name: String,
}

impl osmozzz_core::Harvester for TrelloHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match TrelloConfig::load() {
            Some(c) => c,
            None => {
                warn!("Trello: ~/.osmozzz/trello.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        let auth = format!("key={}&token={}", cfg.api_key, cfg.token);
        let client = reqwest::Client::new();
        let mut documents = Vec::new();

        // 1. Récupérer tous les boards de l'utilisateur
        let boards_url = format!("{}/members/me/boards?{}&fields=id,name,closed", TRELLO_API, auth);
        let boards: Vec<TrelloBoard> = match client.get(&boards_url).send().await {
            Ok(r) => r.json().await.unwrap_or_default(),
            Err(e) => {
                warn!("Trello boards API error: {}", e);
                return Ok(vec![]);
            }
        };

        if boards.is_empty() {
            warn!("Trello: aucun board trouvé — vérifiez votre token");
            return Ok(vec![]);
        }

        for board in boards.iter().filter(|b| !b.closed) {
            // 2. Récupérer les listes du board pour avoir les noms
            let lists_url = format!("{}/boards/{}/lists?{}", TRELLO_API, board.id, auth);
            let lists: Vec<TrelloList> = match client.get(&lists_url).send().await {
                Ok(r) => r.json().await.unwrap_or_default(),
                Err(_) => vec![],
            };

            let list_map: std::collections::HashMap<String, String> = lists
                .into_iter()
                .map(|l| (l.id, l.name))
                .collect();

            // 3. Récupérer les cartes du board
            let cards_url = format!(
                "{}/boards/{}/cards?{}&fields=id,name,desc,idList,closed,dateLastActivity,shortUrl,labels",
                TRELLO_API, board.id, auth
            );

            let cards: Vec<TrelloCard> = match client.get(&cards_url).send().await {
                Ok(r) => r.json().await.unwrap_or_default(),
                Err(e) => {
                    warn!("Trello cards error for board '{}': {}", board.name, e);
                    continue;
                }
            };

            for card in cards.iter().filter(|c| !c.closed) {
                let list_name = list_map
                    .get(&card.id_list)
                    .map(|s| s.as_str())
                    .unwrap_or("Unknown");

                let labels: Vec<&str> = card.labels.iter().map(|l| l.name.as_str()).collect();

                let content = format!(
                    "{}\nBoard: {} | Liste: {} | Labels: {}\n\n{}",
                    card.name,
                    board.name,
                    list_name,
                    labels.join(", "),
                    card.desc.trim()
                );

                let checksum = checksum::compute(&content);
                let mut doc =
                    Document::new(SourceType::Trello, &card.short_url, &content, &checksum)
                        .with_title(&card.name);

                if let Some(ref date) = card.date_last_activity {
                    if let Ok(ts) = DateTime::parse_from_rfc3339(date) {
                        doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                    }
                }

                documents.push(doc);
            }
        }

        info!("Trello harvester found {} cards", documents.len());
        Ok(documents)
    }
}
