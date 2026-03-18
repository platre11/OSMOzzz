use osmozzz_core::{Document, Result, SourceType};
use tracing::{info, warn};

use crate::checksum;

pub struct ContactsHarvester;

impl ContactsHarvester {
    pub fn new() -> Self { Self }
}

impl Default for ContactsHarvester {
    fn default() -> Self { Self::new() }
}

impl osmozzz_core::Harvester for ContactsHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let script = r#"tell application "Contacts"
            set sep to "|||OSMOZZZ|||"
            set rec to "~~~OSMOZZZ~~~"
            set output to ""
            repeat with p in every person
                try
                    set pId to id of p
                    set pName to name of p
                    set pCompany to ""
                    try
                        if organization of p is not missing value then
                            set pCompany to organization of p
                        end if
                    end try
                    set pPhones to ""
                    repeat with ph in phones of p
                        set pPhones to pPhones & value of ph & ","
                    end repeat
                    set pEmails to ""
                    repeat with em in emails of p
                        set pEmails to pEmails & value of em & ","
                    end repeat
                    set pNotes to ""
                    try
                        if note of p is not missing value then
                            set pNotes to note of p
                        end if
                    end try
                    set output to output & pId & sep & pName & sep & pCompany & sep & pPhones & sep & pEmails & sep & pNotes & rec
                end try
            end repeat
            return output
        end tell"#;

        let raw = match run_osascript(script).await {
            Ok(r) => r,
            Err(e) => {
                warn!("Contacts AppleScript failed: {e}");
                return Ok(vec![]);
            }
        };

        let mut documents = Vec::new();

        for record in raw.split("~~~OSMOZZZ~~~") {
            let parts: Vec<&str> = record.splitn(6, "|||OSMOZZZ|||").collect();
            if parts.len() < 2 { continue; }

            let contact_id = parts[0].trim();
            let name       = parts[1].trim();
            let company    = if parts.len() > 2 { parts[2].trim() } else { "" };
            let phones     = if parts.len() > 3 { parts[3].trim() } else { "" };
            let emails     = if parts.len() > 4 { parts[4].trim() } else { "" };
            let notes      = if parts.len() > 5 { parts[5].trim() } else { "" };

            if contact_id.is_empty() || name.is_empty() { continue; }

            let mut content = name.to_string();
            if !company.is_empty() { content.push_str(&format!("\nEntreprise: {company}")); }
            if !phones.is_empty()  { content.push_str(&format!("\nTéléphones: {}", phones.trim_end_matches(','))); }
            if !emails.is_empty()  { content.push_str(&format!("\nEmails: {}", emails.trim_end_matches(','))); }
            if !notes.is_empty()   { content.push_str(&format!("\nNotes: {notes}")); }

            let chk = checksum::compute(&content);
            let url = format!("contacts://person/{}", contact_id.replace(' ', "_"));

            let doc = Document::new(SourceType::Contacts, &url, &content, &chk)
                .with_title(name);

            documents.push(doc);
        }

        info!("Contacts harvester found {} contacts", documents.len());
        Ok(documents)
    }
}

async fn run_osascript(script: &str) -> std::result::Result<String, String> {
    let output = tokio::process::Command::new("osascript")
        .arg("-e").arg(script)
        .output().await
        .map_err(|e| format!("osascript spawn: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Construit une map téléphone → nom depuis Apple Contacts (pour enrichir iMessage).
/// Normalise les numéros : retire espaces, tirets, parenthèses.
pub async fn build_phone_name_map() -> std::collections::HashMap<String, String> {
    let script = r#"tell application "Contacts"
        set sep to "|||"
        set rec to "~~~"
        set output to ""
        repeat with p in every person
            try
                set pName to name of p
                repeat with ph in phones of p
                    set output to output & (value of ph) & sep & pName & rec
                end repeat
            end try
        end repeat
        return output
    end tell"#;

    let mut map = std::collections::HashMap::new();

    let raw = match run_osascript(script).await {
        Ok(r) => r,
        Err(_) => return map,
    };

    for record in raw.split("~~~") {
        let parts: Vec<&str> = record.splitn(2, "|||").collect();
        if parts.len() != 2 { continue; }
        let phone = normalize_phone(parts[0].trim());
        let name  = parts[1].trim().to_string();
        if phone.is_empty() || name.is_empty() { continue; }
        map.insert(phone, name);
    }

    map
}

/// Normalise un numéro vers un format canonique (chiffres seulement, sans + ni indicatif).
/// Exemples :
///   "+33766300049" → "766300049"   (retire +33 puis le 0 initial)
///   "07 66 30 00 49" → "766300049" (retire espaces, 0 initial)
///   "+1-800-555-0100" → "18005550100" (autres pays : conserve tel quel)
pub fn normalize_phone(phone: &str) -> String {
    // 1. Garde uniquement chiffres et +
    let digits: String = phone.chars()
        .filter(|c| c.is_ascii_digit() || *c == '+')
        .collect();

    // 2. Retire le + initial
    let digits = digits.trim_start_matches('+');

    // 3. Cas France : commence par 33 suivi d'un chiffre non-0 → retire le 33
    if digits.starts_with("33") && digits.len() == 11 {
        return digits[2..].to_string();
    }

    // 4. Numéro local français : commence par 0 avec 10 chiffres → retire le 0
    if digits.starts_with('0') && digits.len() == 10 {
        return digits[1..].to_string();
    }

    digits.to_string()
}
