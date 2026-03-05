/// Airtable Harvester — indexe les records de vos bases Airtable.
///
/// Config : ~/.osmozzz/airtable.toml
/// ```toml
/// token    = "patXXXXXXXXXXXXXXXX.xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// bases    = ["appXXXXXXXXXXXXXX"]   # IDs des bases à indexer
/// ```
///
/// Token : airtable.com/create/tokens → Personal access tokens → Create token
/// Scopes nécessaires : data.records:read, schema.bases:read
/// ID de base : visible dans l'URL de votre base (https://airtable.com/appXXXX/...)
use chrono::DateTime;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const AIRTABLE_API: &str = "https://api.airtable.com/v0";
const MAX_RECORDS_PER_TABLE: usize = 1000;

#[derive(Debug, Deserialize)]
struct AirtableConfig {
    token: String,
    #[serde(default)]
    bases: Vec<String>,
}

impl AirtableConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/airtable.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct AirtableHarvester;

impl AirtableHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AirtableHarvester {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct BaseTablesResponse {
    tables: Vec<AirtableTable>,
}

#[derive(Debug, Deserialize)]
struct AirtableTable {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct RecordsResponse {
    records: Vec<AirtableRecord>,
    #[serde(default)]
    offset: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AirtableRecord {
    id: String,
    fields: serde_json::Value,
    #[serde(rename = "createdTime")]
    created_time: Option<String>,
}

/// Convertit les fields Airtable en texte lisible
fn fields_to_text(fields: &serde_json::Value, table_name: &str, record_id: &str) -> String {
    let mut parts = vec![format!("[{}] {}", table_name, record_id)];

    if let Some(obj) = fields.as_object() {
        for (key, val) in obj {
            let text = match val {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| v.as_str().or_else(|| v.get("name").and_then(|n| n.as_str())))
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => continue,
            };
            if !text.is_empty() {
                parts.push(format!("{}: {}", key, text));
            }
        }
    }

    parts.join("\n")
}

impl osmozzz_core::Harvester for AirtableHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match AirtableConfig::load() {
            Some(c) => c,
            None => {
                warn!("Airtable: ~/.osmozzz/airtable.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        if cfg.bases.is_empty() {
            warn!("Airtable: aucune base configurée dans airtable.toml (bases = [...])");
            return Ok(vec![]);
        }

        let client = reqwest::Client::new();
        let mut documents = Vec::new();

        for base_id in &cfg.bases {
            // Récupérer le schéma (liste des tables)
            let schema_url = format!("https://api.airtable.com/v0/meta/bases/{}/tables", base_id);
            let schema_resp = match client
                .get(&schema_url)
                .bearer_auth(&cfg.token)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("Airtable schema error for {}: {}", base_id, e);
                    continue;
                }
            };

            if schema_resp.status() == 401 || schema_resp.status() == 403 {
                warn!("Airtable: accès refusé à la base {} ({})", base_id, schema_resp.status());
                continue;
            }

            let schema: BaseTablesResponse = match schema_resp.json().await {
                Ok(s) => s,
                Err(e) => {
                    warn!("Airtable schema parse error: {}", e);
                    continue;
                }
            };

            for table in &schema.tables {
                let mut offset: Option<String> = None;
                let mut count = 0;

                loop {
                    let mut url = format!(
                        "{}/{}/{}?pageSize=100",
                        AIRTABLE_API, base_id, table.id
                    );
                    if let Some(ref o) = offset {
                        url.push_str(&format!("&offset={}", o));
                    }

                    let resp = match client.get(&url).bearer_auth(&cfg.token).send().await {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("Airtable records error: {}", e);
                            break;
                        }
                    };

                    if !resp.status().is_success() {
                        warn!("Airtable status {} for table {}", resp.status(), table.name);
                        break;
                    }

                    let records_resp: RecordsResponse = match resp.json().await {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("Airtable records parse error: {}", e);
                            break;
                        }
                    };

                    for record in &records_resp.records {
                        let content = fields_to_text(&record.fields, &table.name, &record.id);
                        let record_url = format!(
                            "https://airtable.com/{}/{}/{}",
                            base_id, table.id, record.id
                        );

                        let checksum = checksum::compute(&content);
                        let mut doc = Document::new(
                            SourceType::Airtable,
                            &record_url,
                            &content,
                            &checksum,
                        );

                        if let Some(ref created) = record.created_time {
                            if let Ok(ts) = DateTime::parse_from_rfc3339(created) {
                                doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                            }
                        }

                        documents.push(doc);
                        count += 1;
                    }

                    if records_resp.offset.is_none() || count >= MAX_RECORDS_PER_TABLE {
                        break;
                    }
                    offset = records_resp.offset;
                }
            }
        }

        info!("Airtable harvester found {} records", documents.len());
        Ok(documents)
    }
}
