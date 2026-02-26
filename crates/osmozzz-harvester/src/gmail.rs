/// Gmail IMAP Harvester
///
/// Connexion : imap.gmail.com:993 (TLS)
/// Auth      : adresse Gmail + mot de passe d'application Google
/// Config    : ~/.osmozzz/gmail.toml  OU  variables d'env OSMOZZZ_GMAIL_USER / OSMOZZZ_GMAIL_PASSWORD
///
/// Stratégie :
/// - Récupère les N derniers emails (défaut : 500) triés par date décroissante
/// - Extrait : expéditeur, objet, date, corps texte (HTML strippé si nécessaire)
/// - Déduplique par Message-ID (checksum SHA-256)
/// - Corps tronqué à 4000 chars pour économiser de l'espace vectoriel
use std::collections::HashSet;

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use mailparse::{MailHeaderMap, parse_mail};
use osmozzz_core::{Document, OsmozzError, Result, SourceType};
use tokio::task;
use tracing::{debug, info, warn};

use crate::checksum;

const IMAP_SERVER: &str = "imap.gmail.com";
const IMAP_PORT: u16 = 993;
const MAX_BODY_CHARS: usize = 20_000;

// ─── Config ──────────────────────────────────────────────────────────────────

pub struct GmailConfig {
    pub username: String,
    /// Mot de passe d'application Google (pas le mot de passe principal)
    pub password: String,
}

impl GmailConfig {
    /// Charge depuis les variables d'environnement.
    pub fn from_env() -> Option<Self> {
        let username = std::env::var("OSMOZZZ_GMAIL_USER").ok()?;
        let password = std::env::var("OSMOZZZ_GMAIL_PASSWORD").ok()?;
        Some(Self { username, password })
    }

    /// Charge depuis ~/.osmozzz/gmail.toml
    /// Format attendu :
    ///   username = "you@gmail.com"
    ///   password = "xxxx xxxx xxxx xxxx"
    pub fn from_file() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/gmail.toml");
        let content = std::fs::read_to_string(path).ok()?;

        let mut username = None;
        let mut password = None;

        for line in content.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("username") {
                if let Some(val) = rest.trim_start_matches(|c| c == ' ' || c == '=').strip_prefix('"') {
                    username = val.strip_suffix('"').map(String::from);
                }
            } else if let Some(rest) = line.strip_prefix("password") {
                if let Some(val) = rest.trim_start_matches(|c| c == ' ' || c == '=').strip_prefix('"') {
                    password = val.strip_suffix('"').map(String::from);
                }
            }
        }

        Some(Self {
            username: username?,
            password: password?,
        })
    }

    /// Essaie les variables d'env puis le fichier config.
    pub fn load() -> Option<Self> {
        Self::from_env().or_else(Self::from_file)
    }
}

// ─── Harvester ───────────────────────────────────────────────────────────────

pub struct GmailHarvester {
    config: GmailConfig,
    max_emails: usize,
    known_checksums: HashSet<String>,
    /// If set, use IMAP SEARCH SINCE to fetch only emails newer than this date.
    /// More efficient than fetching last N emails by sequence number.
    since_date: Option<NaiveDate>,
}

impl GmailHarvester {
    pub fn new(config: GmailConfig) -> Self {
        Self {
            config,
            max_emails: 5000,
            known_checksums: HashSet::new(),
            since_date: None,
        }
    }

    pub fn with_max(mut self, max: usize) -> Self {
        self.max_emails = max;
        self
    }

    pub fn with_known_checksums(mut self, checksums: HashSet<String>) -> Self {
        self.known_checksums = checksums;
        self
    }

    /// Only fetch emails received on or after this date (uses IMAP SEARCH SINCE).
    /// Much faster than fetching last N emails when the index is already up to date.
    pub fn with_since(mut self, date: NaiveDate) -> Self {
        self.since_date = Some(date);
        self
    }
}

impl osmozzz_core::Harvester for GmailHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let username = self.config.username.clone();
        let password = self.config.password.clone();
        let max_emails = self.max_emails;
        let known = self.known_checksums.clone();
        let since_date = self.since_date;

        // IMAP est synchrone → on l'isole dans spawn_blocking
        let docs = task::spawn_blocking(move || {
            harvest_imap(&username, &password, max_emails, &known, since_date)
        })
        .await
        .map_err(|e| OsmozzError::Harvester(format!("Task join error: {}", e)))??;

        Ok(docs)
    }
}

// ─── Logique IMAP (sync) ──────────────────────────────────────────────────────

fn harvest_imap(
    username: &str,
    password: &str,
    max_emails: usize,
    known_checksums: &HashSet<String>,
    since_date: Option<NaiveDate>,
) -> Result<Vec<Document>> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| OsmozzError::Harvester(format!("TLS init échoué : {}", e)))?;

    let client = imap::connect((IMAP_SERVER, IMAP_PORT), IMAP_SERVER, &tls)
        .map_err(|e| OsmozzError::Harvester(format!(
            "Connexion imap.gmail.com échouée : {}. Vérifiez votre connexion internet.", e
        )))?;

    let mut session = client
        .login(username, password)
        .map_err(|(e, _)| OsmozzError::Harvester(format!(
            "Authentification Gmail échouée : {}.\n\
             → Assurez-vous d'utiliser un mot de passe d'application Google,\n\
             → pas votre mot de passe principal.\n\
             → Générez-en un sur : myaccount.google.com/apppasswords", e
        )))?;

    let mailbox = session
        .select("[Gmail]/All Mail")
        .map_err(|e| OsmozzError::Harvester(format!("Impossible d'ouvrir [Gmail]/All Mail : {}", e)))?;

    let total = mailbox.exists as usize;
    info!("Gmail INBOX : {} messages au total", total);

    if total == 0 {
        session.logout().ok();
        return Ok(vec![]);
    }

    // Choisir la stratégie de sélection des messages
    let range = if let Some(since) = since_date {
        // Stratégie incrémentale : SEARCH SINCE pour ne récupérer que les nouveaux
        let since_str = format_imap_date(since);
        info!("Gmail : SEARCH SINCE {} (indexation incrémentale)", since_str);

        let seq_nums = session
            .search(format!("SINCE {}", since_str))
            .map_err(|e| OsmozzError::Harvester(format!("SEARCH SINCE échoué : {}", e)))?;

        if seq_nums.is_empty() {
            session.logout().ok();
            info!("Gmail SEARCH SINCE {} : aucun nouveau message", since_str);
            return Ok(vec![]);
        }

        let mut nums: Vec<u32> = seq_nums.into_iter().collect();
        nums.sort_unstable();
        info!("Gmail SEARCH SINCE {} : {} messages à vérifier", since_str, nums.len());
        nums_to_sequence_set(&nums)
    } else {
        // Stratégie initiale : N derniers messages par numéro de séquence
        let start = if total > max_emails { total - max_emails + 1 } else { 1 };
        info!("Gmail : récupération des messages {} → {} (indexation initiale)", start, total);
        format!("{}:{}", start, total)
    };

    let messages = session
        .fetch(&range, "RFC822")
        .map_err(|e| OsmozzError::Harvester(format!("FETCH échoué : {}", e)))?;

    let mut documents = Vec::new();

    for msg in messages.iter() {
        let raw = match msg.body() {
            Some(b) => b,
            None => continue,
        };

        let parsed = match parse_mail(raw) {
            Ok(p) => p,
            Err(e) => {
                warn!("Impossible de parser un email : {}", e);
                continue;
            }
        };

        // Extraire les headers
        let headers = &parsed.headers;
        let from    = headers.get_first_value("From").unwrap_or_default();
        let subject = headers.get_first_value("Subject").unwrap_or_else(|| "(sans objet)".into());
        let date_str = headers.get_first_value("Date").unwrap_or_default();
        let msg_id   = headers.get_first_value("Message-ID")
            .unwrap_or_else(|| format!("seq-{}", msg.message));

        // Extraire le corps texte (préfère text/plain, fallback text/html)
        let body = extract_body(&parsed);

        // Checksum basé sur Message-ID (stable, ne change pas si le mail est re-fetché)
        let ck = checksum::compute(msg_id.trim());

        if known_checksums.contains(&ck) {
            debug!("Skip email déjà indexé : {}", subject.trim());
            continue;
        }

        // Contenu indexable : From + Subject + Body
        let content = format!(
            "De : {}\nObjet : {}\n\n{}",
            from.trim(),
            subject.trim(),
            body.trim()
        );

        let source_ts = parse_email_date(date_str.trim());

        // URL interne pour retrouver l'email
        let clean_id = msg_id.trim().trim_matches(|c| c == '<' || c == '>');
        let url = format!("gmail://message/{}", clean_id);

        let mut doc = Document::new(SourceType::Email, &url, &content, &ck)
            .with_title(subject.trim());

        if let Some(ts) = source_ts {
            doc = doc.with_source_ts(ts);
        }

        documents.push(doc);
    }

    session.logout().ok();

    info!("Gmail harvester : {} nouveaux emails à indexer", documents.len());
    Ok(documents)
}

// ─── Helpers IMAP ────────────────────────────────────────────────────────────

/// Formats a NaiveDate into IMAP date format: "d-Mon-yyyy" (e.g., "1-Feb-2026").
fn format_imap_date(date: NaiveDate) -> String {
    let months = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
    let m = months[(date.month() as usize) - 1];
    format!("{}-{}-{}", date.day(), m, date.year())
}

/// Converts a sorted slice of IMAP sequence numbers into a compact sequence set string.
/// Example: [1, 2, 3, 5, 6] → "1:3,5:6"
fn nums_to_sequence_set(nums: &[u32]) -> String {
    if nums.is_empty() { return String::new(); }
    let mut ranges: Vec<String> = Vec::new();
    let mut start = nums[0];
    let mut end = nums[0];
    for &n in &nums[1..] {
        if n == end + 1 {
            end = n;
        } else {
            if start == end { ranges.push(format!("{}", start)); }
            else { ranges.push(format!("{}:{}", start, end)); }
            start = n;
            end = n;
        }
    }
    if start == end { ranges.push(format!("{}", start)); }
    else { ranges.push(format!("{}:{}", start, end)); }
    ranges.join(",")
}

// ─── Extraction du corps ──────────────────────────────────────────────────────

/// Extrait le texte d'un email parsé.
/// Préfère text/plain. Si absent, utilise text/html (strippé).
fn extract_body(mail: &mailparse::ParsedMail) -> String {
    // Email simple (non multipart)
    if mail.subparts.is_empty() {
        let ct = mail.ctype.mimetype.to_lowercase();
        if ct.contains("text/plain") {
            let body = mail.get_body().unwrap_or_default();
            return truncate(&decode_html_entities(&body), MAX_BODY_CHARS);
        }
        if ct.contains("text/html") {
            let html = mail.get_body().unwrap_or_default();
            return truncate(&decode_html_entities(&strip_html(&html)), MAX_BODY_CHARS);
        }
        return String::new();
    }

    // Multipart : chercher text/plain d'abord
    for part in &mail.subparts {
        let ct = part.ctype.mimetype.to_lowercase();
        if ct.contains("text/plain") {
            let body = part.get_body().unwrap_or_default();
            if !body.trim().is_empty() {
                return truncate(&decode_html_entities(&body), MAX_BODY_CHARS);
            }
        }
    }

    // Fallback : text/html
    for part in &mail.subparts {
        let ct = part.ctype.mimetype.to_lowercase();
        if ct.contains("text/html") {
            let html = part.get_body().unwrap_or_default();
            let text = decode_html_entities(&strip_html(&html));
            if !text.trim().is_empty() {
                return truncate(&text, MAX_BODY_CHARS);
            }
        }
    }

    // Récursif pour multipart imbriqués
    for part in &mail.subparts {
        let body = extract_body(part);
        if !body.is_empty() {
            return body;
        }
    }

    String::new()
}

/// Decode common HTML entities to plain text.
fn decode_html_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '&' {
            // Find the closing semicolon
            let rest = &s[i..];
            if let Some(end) = rest.find(';') {
                let entity = &rest[1..end]; // between & and ;
                let replacement = match entity {
                    "nbsp" | "#160" => " ",
                    "amp"  => "&",
                    "lt"   => "<",
                    "gt"   => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    "eacute" | "#233" | "#xE9" | "#xe9" => "é",
                    "egrave" | "#232" | "#xE8" | "#xe8" => "è",
                    "ecirc"  | "#234" | "#xEA" | "#xea" => "ê",
                    "euml"   | "#235" | "#xEB" | "#xeb" => "ë",
                    "agrave" | "#224" | "#xE0" | "#xe0" => "à",
                    "acirc"  | "#226" | "#xE2" | "#xe2" => "â",
                    "auml"   | "#228" | "#xE4" | "#xe4" => "ä",
                    "ugrave" | "#249" | "#xF9" | "#xf9" => "ù",
                    "ucirc"  | "#251" | "#xFB" | "#xfb" => "û",
                    "uuml"   | "#252" | "#xFC" | "#xfc" => "ü",
                    "ocirc"  | "#244" | "#xF4" | "#xf4" => "ô",
                    "iuml"   | "#239" | "#xEF" | "#xef" => "ï",
                    "ccedil" | "#231" | "#xE7" | "#xe7" => "ç",
                    "ntilde" | "#241" | "#xF1" | "#xf1" => "ñ",
                    "aelig"  | "#230" | "#xE6" | "#xe6" => "æ",
                    "oslash" | "#248" | "#xF8" | "#xf8" => "ø",
                    "szlig"  | "#223" | "#xDF" | "#xdf" => "ß",
                    "mdash"  | "#8212"| "#x2014" => "—",
                    "ndash"  | "#8211"| "#x2013" => "–",
                    "laquo"  | "#171" | "#xAB"   => "«",
                    "raquo"  | "#187" | "#xBB"   => "»",
                    "hellip" | "#8230"| "#x2026" => "…",
                    "euro"   | "#8364"| "#x20AC" => "€",
                    _ => {
                        // Numeric entity: &#NNN; or &#xHH;
                        if let Some(n) = entity.strip_prefix('#') {
                            let code: Option<u32> = if let Some(h) = n.strip_prefix('x').or_else(|| n.strip_prefix('X')) {
                                u32::from_str_radix(h, 16).ok()
                            } else {
                                n.parse().ok()
                            };
                            if let Some(c) = code.and_then(char::from_u32) {
                                out.push(c);
                                // skip over &entity;
                                for _ in 0..(end) { chars.next(); }
                                continue;
                            }
                        }
                        // Unknown entity — keep as-is
                        out.push('&');
                        continue;
                    }
                };
                out.push_str(replacement);
                for _ in 0..(end) { chars.next(); }
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// Supprime les balises HTML d'un corps d'email.
fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut skip_depth: u32 = 0;
    let mut tag_buf = String::new();

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
            tag_buf.clear();
        } else if c == '>' && in_tag {
            in_tag = false;
            let tag = tag_buf.trim().to_lowercase();
            let is_close = tag.starts_with('/');
            let name = tag.trim_start_matches('/').split_whitespace().next().unwrap_or("");

            if matches!(name, "script" | "style" | "head") {
                if is_close && skip_depth > 0 { skip_depth -= 1; }
                else if !is_close { skip_depth += 1; }
            } else if skip_depth == 0 && matches!(name, "p" | "div" | "br" | "h1" | "h2" | "h3" | "li" | "tr") {
                result.push('\n');
            }
        } else if in_tag {
            tag_buf.push(c);
        } else if skip_depth == 0 {
            result.push(c);
        }
    }

    // Nettoyer les lignes vides
    let mut cleaned = String::new();
    let mut prev_blank = false;
    for line in result.lines() {
        let t = line.trim();
        if t.is_empty() {
            if !prev_blank { cleaned.push('\n'); }
            prev_blank = true;
        } else {
            cleaned.push_str(t);
            cleaned.push('\n');
            prev_blank = false;
        }
    }
    cleaned.trim().to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{}\n[... email tronqué]", t)
    }
}

// ─── Parse date RFC 2822 ──────────────────────────────────────────────────────

fn parse_email_date(date_str: &str) -> Option<DateTime<Utc>> {
    // Strip "Day, " prefix if present (e.g. "Mon, ")
    let s = if let Some(comma_pos) = date_str.find(',') {
        date_str[comma_pos + 1..].trim()
    } else {
        date_str.trim()
    };

    // Strip trailing comment like "(UTC)", "(CET)", "(PDT)" etc.
    let s = if let Some(paren_pos) = s.rfind('(') {
        s[..paren_pos].trim()
    } else {
        s
    };

    // "22 Feb 2026 15:30:00 +0100"
    if let Ok(dt) = DateTime::parse_from_str(s, "%d %b %Y %H:%M:%S %z") {
        return Some(dt.with_timezone(&Utc));
    }
    // Without seconds: "22 Feb 2026 15:30 +0100"
    if let Ok(dt) = DateTime::parse_from_str(s, "%d %b %Y %H:%M %z") {
        return Some(dt.with_timezone(&Utc));
    }
    // Text timezone (GMT, UTC): replace last word with +0000
    if let Some((left, _tz)) = s.rsplit_once(' ') {
        let with_offset = format!("{} +0000", left);
        if let Ok(dt) = DateTime::parse_from_str(&with_offset, "%d %b %Y %H:%M:%S %z") {
            return Some(dt.with_timezone(&Utc));
        }
        if let Ok(dt) = DateTime::parse_from_str(&with_offset, "%d %b %Y %H:%M %z") {
            return Some(dt.with_timezone(&Utc));
        }
    }

    None
}
