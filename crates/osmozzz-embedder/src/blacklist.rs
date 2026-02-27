use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Persistent blacklist stored at ~/.osmozzz/blacklist.toml
/// Two levels:
///   - urls: specific document URLs (ban one precise document)
///   - per-source sets: ban everything from a sender/phone/domain/path
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Blacklist {
    /// Specific document URLs banned (any source)
    #[serde(default)]
    pub urls: HashSet<String>,
    /// Banned Gmail sender addresses
    #[serde(default)]
    pub gmail: HashSet<String>,
    /// Banned iMessage phone numbers
    #[serde(default)]
    pub imessage: HashSet<String>,
    /// Banned Chrome domains (e.g. "facebook.com")
    #[serde(default)]
    pub chrome: HashSet<String>,
    /// Banned Safari domains
    #[serde(default)]
    pub safari: HashSet<String>,
    /// Banned file/folder paths (prefix match)
    #[serde(default)]
    pub files: HashSet<String>,
}

impl Blacklist {
    pub fn path() -> PathBuf {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".osmozzz/blacklist.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        if !path.exists() { return Self::default(); }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Ban one specific document by URL.
    pub fn ban_url(&mut self, url: &str) {
        self.urls.insert(url.to_string());
    }

    /// Ban all documents from a source identifier (sender, phone, domain, path).
    pub fn ban_source_item(&mut self, source: &str, identifier: &str) {
        let set = match source {
            "email"    => &mut self.gmail,
            "imessage" => &mut self.imessage,
            "chrome"   => &mut self.chrome,
            "safari"   => &mut self.safari,
            "file"     => &mut self.files,
            _ => return,
        };
        set.insert(identifier.to_string());
    }

    /// Returns true if this document must NOT be indexed (daemon-side check).
    pub fn is_banned(&self, source: &str, url: &str, title: &str) -> bool {
        self.is_result_banned(source, url, title, "")
    }

    /// Returns true if a document (with content) is banned — used for result filtering.
    pub fn is_result_banned(&self, source: &str, url: &str, title: &str, content: &str) -> bool {
        if self.urls.contains(url) { return true; }
        match source {
            "email" => {
                // Extract sender from "De : sender@..." in content
                let sender = content.lines()
                    .find(|l| l.starts_with("De : ") || l.starts_with("De: "))
                    .and_then(|l| l.splitn(2, ':').nth(1))
                    .map(|s| s.trim())
                    .unwrap_or("");
                self.gmail.iter().any(|s| {
                    (!sender.is_empty() && sender.contains(s.as_str()))
                    || url.contains(s.as_str())
                })
            }
            "imessage" => {
                let phone = title.split_whitespace().last().unwrap_or("");
                self.imessage.contains(phone)
            }
            "chrome" => {
                let domain = extract_domain(url).unwrap_or("");
                self.chrome.iter().any(|d| domain_matches(domain, d))
            }
            "safari" => {
                let domain = extract_domain(url).unwrap_or("");
                self.safari.iter().any(|d| domain_matches(domain, d))
            }
            "file" => {
                self.files.iter().any(|p| url.starts_with(p.as_str()))
            }
            _ => false,
        }
    }

    pub fn unban_url(&mut self, url: &str) {
        self.urls.remove(url);
    }

    pub fn unban_source_item(&mut self, source: &str, identifier: &str) {
        let set = match source {
            "email"    => &mut self.gmail,
            "imessage" => &mut self.imessage,
            "chrome"   => &mut self.chrome,
            "safari"   => &mut self.safari,
            "file"     => &mut self.files,
            _ => return,
        };
        set.remove(identifier);
    }

    /// Returns all banned entries as (kind, source, identifier).
    /// kind = "url" for URL bans, "source" for source-level bans.
    pub fn get_all_entries(&self) -> Vec<(String, String, String)> {
        let mut entries = Vec::new();
        for url in &self.urls {
            entries.push(("url".to_string(), "any".to_string(), url.clone()));
        }
        for id in &self.gmail    { entries.push(("source".to_string(), "email".to_string(),    id.clone())); }
        for id in &self.imessage { entries.push(("source".to_string(), "imessage".to_string(), id.clone())); }
        for id in &self.chrome   { entries.push(("source".to_string(), "chrome".to_string(),   id.clone())); }
        for id in &self.safari   { entries.push(("source".to_string(), "safari".to_string(),   id.clone())); }
        for id in &self.files    { entries.push(("source".to_string(), "file".to_string(),      id.clone())); }
        entries
    }
}

fn extract_domain(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    Some(rest.split('/').next().unwrap_or(rest).split(':').next().unwrap_or(rest))
}

fn domain_matches(host: &str, banned: &str) -> bool {
    host == banned || host.ends_with(&format!(".{}", banned))
}
