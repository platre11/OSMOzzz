/// Connecteur Google Calendar — protocole CalDAV avec App Password.
/// Même approche que Gmail IMAP : pas d'OAuth2, utilise un App Password Google.
/// Config : ~/.osmozzz/google.toml (username + app_password)
///
/// CalDAV sur Google :
///   - Principal : https://apidata.googleusercontent.com/caldav/v2/{email}/user/
///   - Events    : https://apidata.googleusercontent.com/caldav/v2/{email}/events/
///   - Auth      : Basic base64(email:app_password)
use serde_json::{json, Value};
use base64::Engine;

// ─── Config ──────────────────────────────────────────────────────────────────

struct GoogleConfig {
    username:     String,
    app_password: String,
}

impl GoogleConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/google.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let t: toml::Value = content.parse().ok()?;
        Some(Self {
            username:     t.get("username")?.as_str()?.to_string(),
            app_password: t.get("app_password")?.as_str()?.to_string(),
        })
    }

    fn basic_auth(&self) -> String {
        let creds = format!("{}:{}", self.username, self.app_password);
        base64::engine::general_purpose::STANDARD.encode(creds.as_bytes())
    }

    fn caldav_base(&self) -> String {
        format!(
            "https://apidata.googleusercontent.com/caldav/v2/{}/",
            urlencoding::encode(&self.username)
        )
    }
}

// ─── CalDAV HTTP helpers ──────────────────────────────────────────────────────

async fn caldav_report(cfg: &GoogleConfig, calendar_path: &str, body: &str) -> Result<String, String> {
    let url = format!("{}{}", cfg.caldav_base(), calendar_path);
    let resp = reqwest::Client::new()
        .request(reqwest::Method::from_bytes(b"REPORT").unwrap(), &url)
        .header("Authorization", format!("Basic {}", cfg.basic_auth()))
        .header("Content-Type", "application/xml; charset=utf-8")
        .header("Depth", "1")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("Erreur réseau CalDAV : {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("CalDAV erreur {status} : {}", &body[..body.len().min(300)]));
    }
    resp.text().await.map_err(|e| format!("Erreur lecture CalDAV : {e}"))
}

async fn caldav_propfind(cfg: &GoogleConfig, path: &str) -> Result<String, String> {
    let url = format!("{}{}", cfg.caldav_base(), path);
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:displayname/>
    <D:resourcetype/>
  </D:prop>
</D:propfind>"#;

    let resp = reqwest::Client::new()
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
        .header("Authorization", format!("Basic {}", cfg.basic_auth()))
        .header("Content-Type", "application/xml; charset=utf-8")
        .header("Depth", "1")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("Erreur réseau CalDAV : {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("CalDAV PROPFIND erreur {status} : {}", &text[..text.len().min(300)]));
    }
    resp.text().await.map_err(|e| e.to_string())
}

async fn caldav_put(cfg: &GoogleConfig, path: &str, ical: &str) -> Result<(), String> {
    let url = format!("{}{}", cfg.caldav_base(), path);
    let resp = reqwest::Client::new()
        .put(&url)
        .header("Authorization", format!("Basic {}", cfg.basic_auth()))
        .header("Content-Type", "text/calendar; charset=utf-8")
        .body(ical.to_string())
        .send()
        .await
        .map_err(|e| format!("Erreur réseau CalDAV PUT : {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("CalDAV PUT erreur {status} : {}", &body[..body.len().min(300)]));
    }
    Ok(())
}

async fn caldav_delete(cfg: &GoogleConfig, path: &str) -> Result<(), String> {
    let url = format!("{}{}", cfg.caldav_base(), path);
    let resp = reqwest::Client::new()
        .delete(&url)
        .header("Authorization", format!("Basic {}", cfg.basic_auth()))
        .send()
        .await
        .map_err(|e| format!("Erreur réseau CalDAV DELETE : {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("CalDAV DELETE erreur {status} : {}", &body[..body.len().min(300)]));
    }
    Ok(())
}

// ─── iCalendar parser ─────────────────────────────────────────────────────────

#[derive(Debug)]
struct CalEvent {
    uid:         String,
    summary:     String,
    dtstart:     String,
    dtend:       String,
    description: String,
    location:    String,
}

/// Extrait tous les VEVENT d'un texte iCalendar
fn parse_ical_events(ical: &str) -> Vec<CalEvent> {
    let mut events = Vec::new();
    let mut in_event = false;
    let mut current: Option<std::collections::HashMap<String, String>> = None;

    for raw_line in ical.lines() {
        // iCal line folding: lines starting with space/tab are continuation
        let line = raw_line.trim_end();

        if line == "BEGIN:VEVENT" {
            in_event = true;
            current = Some(std::collections::HashMap::new());
            continue;
        }
        if line == "END:VEVENT" {
            if let Some(map) = current.take() {
                events.push(CalEvent {
                    uid:         map.get("UID").cloned().unwrap_or_default(),
                    summary:     map.get("SUMMARY").cloned().unwrap_or_else(|| "(sans titre)".to_string()),
                    dtstart:     map.get("DTSTART").or_else(|| map.get("DTSTART;TZID")).cloned().unwrap_or_default(),
                    dtend:       map.get("DTEND").or_else(|| map.get("DTEND;TZID")).cloned().unwrap_or_default(),
                    description: map.get("DESCRIPTION").cloned().unwrap_or_default(),
                    location:    map.get("LOCATION").cloned().unwrap_or_default(),
                });
            }
            in_event = false;
            continue;
        }

        if in_event {
            if let Some(map) = current.as_mut() {
                // Handle property parameters: DTSTART;TZID=Europe/Paris:20260401T090000
                if let Some(colon_pos) = line.find(':') {
                    let key_part = &line[..colon_pos];
                    let value    = &line[colon_pos + 1..];
                    // Normalize key: strip params after semicolon for simple keys
                    let base_key = key_part.split(';').next().unwrap_or(key_part);
                    map.insert(base_key.to_string(), value.to_string());
                }
            }
        }
    }
    events
}

/// Extrait tous les blocs calendar-data d'une réponse XML CalDAV
fn extract_calendar_data(xml: &str) -> Vec<String> {
    let mut results = Vec::new();
    let lower = xml.to_lowercase();

    // Cherche <cal:calendar-data>, <C:calendar-data>, <calendar-data> (variantes de namespace)
    let patterns = ["<cal:calendar-data>", "<c:calendar-data>", "<calendar-data>"];
    let end_patterns = ["</cal:calendar-data>", "</c:calendar-data>", "</calendar-data>"];

    for (start_tag, end_tag) in patterns.iter().zip(end_patterns.iter()) {
        let mut pos = 0;
        while let Some(start) = lower[pos..].find(start_tag) {
            let abs_start = pos + start + start_tag.len();
            if let Some(end) = lower[abs_start..].find(end_tag) {
                let ical = xml[abs_start..abs_start + end].trim().to_string();
                if !ical.is_empty() {
                    results.push(ical);
                }
                pos = abs_start + end + end_tag.len();
            } else {
                break;
            }
        }
    }
    results
}

/// Extrait les display names des calendriers d'une réponse PROPFIND
fn extract_calendar_names(xml: &str) -> Vec<String> {
    let lower = xml.to_lowercase();
    let mut names = Vec::new();
    let start_tag = "<d:displayname>";
    let end_tag   = "</d:displayname>";
    let mut pos = 0;
    while let Some(start) = lower[pos..].find(start_tag) {
        let abs_start = pos + start + start_tag.len();
        if let Some(end) = lower[abs_start..].find(end_tag) {
            let name = xml[abs_start..abs_start + end].trim().to_string();
            if !name.is_empty() {
                names.push(name);
            }
            pos = abs_start + end + end_tag.len();
        } else {
            break;
        }
    }
    names
}

/// Formate une date iCal (20260401T090000Z ou 20260401) en lisible
fn fmt_ical_date(dt: &str) -> String {
    if dt.len() >= 15 {
        // Format: 20260401T090000Z
        let y = &dt[0..4];
        let m = &dt[4..6];
        let d = &dt[6..8];
        let h = &dt[9..11];
        let mn = &dt[11..13];
        format!("{}/{}/{} {}:{}", d, m, y, h, mn)
    } else if dt.len() == 8 {
        // Format: 20260401 (date seule)
        let y = &dt[0..4];
        let m = &dt[4..6];
        let d = &dt[6..8];
        format!("{}/{}/{}", d, m, y)
    } else {
        dt.to_string()
    }
}

/// Formate une liste d'événements
fn fmt_events(events: &[CalEvent]) -> String {
    if events.is_empty() {
        return "Aucun événement trouvé.".to_string();
    }
    events.iter().enumerate().map(|(i, e)| {
        let mut s = format!(
            "{}. 📅 {}\n   Début  : {}\n   Fin    : {}\n   UID    : {}",
            i + 1,
            e.summary,
            fmt_ical_date(&e.dtstart),
            fmt_ical_date(&e.dtend),
            if e.uid.len() > 40 { &e.uid[..40] } else { &e.uid }
        );
        if !e.location.is_empty() {
            s.push_str(&format!("\n   Lieu   : {}", e.location));
        }
        if !e.description.is_empty() {
            let desc = e.description.replace("\\n", " ").replace("\\,", ",");
            let preview = if desc.len() > 100 {
                let mut b = 100;
                while b > 0 && !desc.is_char_boundary(b) { b -= 1; }
                format!("{}…", &desc[..b])
            } else {
                desc
            };
            s.push_str(&format!("\n   Note   : {}", preview));
        }
        s
    }).collect::<Vec<_>>().join("\n\n")
}

/// Génère une plage de dates CalDAV pour une fenêtre temporelle
fn date_range_xml(days_from_now: i64, days_window: i64) -> (String, String) {
    let now = chrono::Utc::now();
    let start = now + chrono::Duration::days(days_from_now);
    let end   = now + chrono::Duration::days(days_from_now + days_window);
    let fmt   = |dt: chrono::DateTime<chrono::Utc>| dt.format("%Y%m%dT%H%M%SZ").to_string();
    (fmt(start), fmt(end))
}

// ─── CalDAV queries ───────────────────────────────────────────────────────────

async fn fetch_events_in_range(cfg: &GoogleConfig, start_ical: &str, end_ical: &str, calendar_path: &str) -> Result<Vec<CalEvent>, String> {
    let body = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<C:calendar-query xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:D="DAV:">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VEVENT">
        <C:time-range start="{start_ical}" end="{end_ical}"/>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#);

    let xml = caldav_report(cfg, calendar_path, &body).await?;
    let ical_blocks = extract_calendar_data(&xml);
    let mut all_events: Vec<CalEvent> = ical_blocks.iter()
        .flat_map(|block| parse_ical_events(block))
        .collect();

    // Tri par date de début
    all_events.sort_by(|a, b| a.dtstart.cmp(&b.dtstart));
    Ok(all_events)
}

/// Récupère un événement par UID via CalDAV REPORT avec filtre UID
async fn fetch_event_by_uid(cfg: &GoogleConfig, uid: &str) -> Result<(CalEvent, String), String> {
    let full_uid = if uid.contains('@') {
        uid.to_string()
    } else {
        format!("{uid}@osmozzz")
    };

    let body = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop><C:calendar-data/></D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VEVENT">
        <C:prop-filter name="UID">
          <C:text-match>{full_uid}</C:text-match>
        </C:prop-filter>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#);

    let xml = caldav_report(cfg, "events/", &body).await?;
    let ical_blocks = extract_calendar_data(&xml);

    // Also try without @osmozzz suffix if not found
    let ical_text = if ical_blocks.is_empty() && full_uid.ends_with("@osmozzz") {
        let bare_uid = uid;
        let body2 = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop><C:calendar-data/></D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VEVENT">
        <C:prop-filter name="UID">
          <C:text-match>{bare_uid}</C:text-match>
        </C:prop-filter>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#);
        let xml2 = caldav_report(cfg, "events/", &body2).await?;
        let blocks2 = extract_calendar_data(&xml2);
        blocks2.into_iter().next()
    } else {
        ical_blocks.into_iter().next()
    };

    let ical_text = ical_text.ok_or_else(|| format!("Événement introuvable avec UID : {uid}"))?;
    let events = parse_ical_events(&ical_text);
    let event = events.into_iter().next()
        .ok_or_else(|| format!("Impossible de parser l'événement avec UID : {uid}"))?;

    Ok((event, ical_text))
}

// ─── Tool definitions ─────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "gcal_upcoming",
            "description": "GOOGLE CALENDAR — Prochains événements à venir. QUAND L'UTILISER : 'mes prochains rendez-vous', 'qu'est-ce que j'ai cette semaine'. Retourne les N prochains événements triés par date.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "days": { "type": "integer", "description": "Fenêtre en jours (défaut: 7, max: 30)", "default": 7, "minimum": 1, "maximum": 30 },
                    "limit": { "type": "integer", "description": "Nombre max d'événements (défaut: 10, max: 30)", "default": 10, "minimum": 1, "maximum": 30 }
                }
            }
        }),
        json!({
            "name": "gcal_today",
            "description": "GOOGLE CALENDAR — Événements d'aujourd'hui uniquement. QUAND L'UTILISER : 'qu'est-ce que j'ai aujourd'hui', 'mon agenda du jour'.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gcal_this_week",
            "description": "GOOGLE CALENDAR — Événements de la semaine en cours (7 prochains jours).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gcal_search",
            "description": "GOOGLE CALENDAR — Recherche un événement par mot-clé dans le titre ou la description.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "Mot-clé à chercher (ex: 'dentiste', 'réunion', 'Thomas')" },
                    "days_back": { "type": "integer", "description": "Cherche aussi dans les N jours passés (défaut: 30)", "default": 30 },
                    "days_ahead": { "type": "integer", "description": "Cherche dans les N jours futurs (défaut: 90)", "default": 90 }
                },
                "required": ["keyword"]
            }
        }),
        json!({
            "name": "gcal_list_calendars",
            "description": "GOOGLE CALENDAR — Liste tous les calendriers disponibles sur le compte Google.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "gcal_create_event",
            "description": "GOOGLE CALENDAR — Crée un nouvel événement dans Google Calendar. Format date : YYYYMMDDTHHMMSSZ (ex: 20260415T090000Z) ou YYYYMMDD pour journée entière.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title":       { "type": "string", "description": "Titre de l'événement" },
                    "start":       { "type": "string", "description": "Début : 20260415T090000Z ou 20260415" },
                    "end":         { "type": "string", "description": "Fin   : 20260415T100000Z ou 20260416" },
                    "description": { "type": "string", "description": "Description ou notes (optionnel)" },
                    "location":    { "type": "string", "description": "Lieu (optionnel)" }
                },
                "required": ["title", "start", "end"]
            }
        }),
        json!({
            "name": "gcal_delete_event",
            "description": "GOOGLE CALENDAR — Supprime un événement par son UID (obtenu depuis gcal_upcoming, gcal_search, etc.). Action irréversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uid": { "type": "string", "description": "UID de l'événement à supprimer (ex: abc123@google.com)" }
                },
                "required": ["uid"]
            }
        }),
        json!({
            "name": "gcal_update_event",
            "description": "GCAL 📅 — Met à jour un événement existant par son UID. Modifie uniquement les champs fournis (titre, début, fin, description, lieu). QUAND L'UTILISER : 'déplace mon rendez-vous', 'change le titre de cet événement', 'ajoute une description'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uid":         { "type": "string", "description": "UID de l'événement (sans suffixe @osmozzz, ex: abc123)" },
                    "title":       { "type": "string", "description": "Nouveau titre (optionnel)" },
                    "start":       { "type": "string", "description": "Nouvelle date de début : 20260415T090000Z ou 20260415 (optionnel)" },
                    "end":         { "type": "string", "description": "Nouvelle date de fin : 20260415T100000Z ou 20260416 (optionnel)" },
                    "description": { "type": "string", "description": "Nouvelle description (optionnel)" },
                    "location":    { "type": "string", "description": "Nouveau lieu (optionnel)" }
                },
                "required": ["uid"]
            }
        }),
        json!({
            "name": "gcal_get_event",
            "description": "GCAL 📅 — Récupère les détails complets d'un événement par son UID. QUAND L'UTILISER : 'montre-moi les détails de ce rendez-vous', 'quel est le lieu de cet événement'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uid": { "type": "string", "description": "UID de l'événement (sans suffixe @osmozzz, ex: abc123)" }
                },
                "required": ["uid"]
            }
        }),
        json!({
            "name": "gcal_get_free_busy",
            "description": "GCAL 📅 — Vérifie les disponibilités (créneaux occupés) sur une plage de dates. QUAND L'UTILISER : 'suis-je libre mardi prochain ?', 'quels créneaux sont libres cette semaine ?'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "start":         { "type": "string", "description": "Début de la plage : YYYYMMDD ou YYYYMMDDTHHmmssZ (ex: 20260415 ou 20260415T090000Z)" },
                    "end":           { "type": "string", "description": "Fin de la plage : YYYYMMDD ou YYYYMMDDTHHmmssZ (ex: 20260422 ou 20260415T180000Z)" },
                    "calendar_path": { "type": "string", "description": "Chemin du calendrier (défaut: 'events/')", "default": "events/" }
                },
                "required": ["start", "end"]
            }
        }),
        json!({
            "name": "gcal_add_attendee",
            "description": "GCAL 📅 — Ajoute un participant (attendee) à un événement existant. QUAND L'UTILISER : 'invite Thomas à cette réunion', 'ajoute un participant à cet événement'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uid":   { "type": "string", "description": "UID de l'événement (sans suffixe @osmozzz)" },
                    "email": { "type": "string", "description": "Adresse email du participant (ex: thomas@example.com)" },
                    "name":  { "type": "string", "description": "Nom affiché du participant (optionnel, utilisé comme CN=)" }
                },
                "required": ["uid", "email"]
            }
        }),
        json!({
            "name": "gcal_list_upcoming_for_calendar",
            "description": "GCAL 📅 — Liste les prochains événements d'un calendrier spécifique (autre que le calendrier principal). QUAND L'UTILISER : 'montre mes événements du calendrier travail', 'prochains événements du calendrier partagé'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "calendar_path": { "type": "string", "description": "Chemin du calendrier CalDAV (ex: 'cal_id/', 'work/', 'family/')" },
                    "days":          { "type": "integer", "description": "Nombre de jours à venir (défaut: 7, max: 30)", "default": 7, "minimum": 1, "maximum": 30 }
                },
                "required": ["calendar_path"]
            }
        }),
    ]
}

// ─── Handler ──────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = GoogleConfig::load()
        .ok_or_else(|| "google.toml introuvable — configure Google Calendar dans le dashboard OSMOzzz (username + app_password Google)".to_string())?;

    match name {
        "gcal_upcoming" => {
            let days  = args["days"].as_i64().unwrap_or(7).clamp(1, 30);
            let limit = args["limit"].as_i64().unwrap_or(10).clamp(1, 30) as usize;
            let (start, end) = date_range_xml(0, days);
            let mut events = fetch_events_in_range(&cfg, &start, &end, "events/").await?;
            events.truncate(limit);
            Ok(format!("📅 Prochains {} événements ({} jours) :\n\n{}", events.len(), days, fmt_events(&events)))
        }

        "gcal_today" => {
            let now   = chrono::Utc::now();
            let start = now.format("%Y%m%dT000000Z").to_string();
            let end   = now.format("%Y%m%dT235959Z").to_string();
            let events = fetch_events_in_range(&cfg, &start, &end, "events/").await?;
            Ok(format!("📅 Agenda du {} :\n\n{}",
                now.format("%d/%m/%Y"),
                fmt_events(&events)))
        }

        "gcal_this_week" => {
            let (start, end) = date_range_xml(0, 7);
            let events = fetch_events_in_range(&cfg, &start, &end, "events/").await?;
            Ok(format!("📅 Événements cette semaine ({}) :\n\n{}", events.len(), fmt_events(&events)))
        }

        "gcal_search" => {
            let keyword = args["keyword"].as_str()
                .ok_or_else(|| "Paramètre 'keyword' requis".to_string())?;
            let days_back  = args["days_back"].as_i64().unwrap_or(30);
            let days_ahead = args["days_ahead"].as_i64().unwrap_or(90);
            let (start, _) = date_range_xml(-days_back, days_back + days_ahead);
            let (_, end)   = date_range_xml(0, days_ahead);
            let events = fetch_events_in_range(&cfg, &start, &end, "events/").await?;
            let kw_lower = keyword.to_lowercase();
            let matched: Vec<&CalEvent> = events.iter()
                .filter(|e|
                    e.summary.to_lowercase().contains(&kw_lower) ||
                    e.description.to_lowercase().contains(&kw_lower) ||
                    e.location.to_lowercase().contains(&kw_lower)
                )
                .collect();
            if matched.is_empty() {
                Ok(format!("Aucun événement trouvé pour \"{}\" dans les {} jours passés et {} jours à venir.", keyword, days_back, days_ahead))
            } else {
                let owned: Vec<CalEvent> = matched.iter().map(|e| CalEvent {
                    uid:         e.uid.clone(),
                    summary:     e.summary.clone(),
                    dtstart:     e.dtstart.clone(),
                    dtend:       e.dtend.clone(),
                    description: e.description.clone(),
                    location:    e.location.clone(),
                }).collect();
                Ok(format!("🔍 {} événement(s) trouvé(s) pour \"{}\" :\n\n{}", owned.len(), keyword, fmt_events(&owned)))
            }
        }

        "gcal_list_calendars" => {
            let xml   = caldav_propfind(&cfg, "user/").await?;
            let names = extract_calendar_names(&xml);
            if names.is_empty() {
                Ok(format!("Calendriers du compte {} : (aucun nom trouvé — CalDAV PROPFIND OK)", cfg.username))
            } else {
                let list = names.iter().enumerate()
                    .map(|(i, n)| format!("{}. 📅 {}", i + 1, n))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(format!("📅 Calendriers de {} :\n\n{}", cfg.username, list))
            }
        }

        "gcal_create_event" => {
            let title = args["title"].as_str()
                .ok_or_else(|| "Paramètre 'title' requis".to_string())?;
            let start = args["start"].as_str()
                .ok_or_else(|| "Paramètre 'start' requis".to_string())?;
            let end = args["end"].as_str()
                .ok_or_else(|| "Paramètre 'end' requis".to_string())?;
            let description = args["description"].as_str().unwrap_or("");
            let location    = args["location"].as_str().unwrap_or("");

            let uid = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

            // Détermine si c'est une date entière (8 chars) ou datetime
            let (dtstart, dtend) = if start.len() == 8 {
                (format!("DTSTART;VALUE=DATE:{start}"), format!("DTEND;VALUE=DATE:{end}"))
            } else {
                (format!("DTSTART:{start}"), format!("DTEND:{end}"))
            };

            let ical = format!(
                "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//OSMOzzz//EN\r\nBEGIN:VEVENT\r\nUID:{uid}@osmozzz\r\nDTSTAMP:{now}\r\n{dtstart}\r\n{dtend}\r\nSUMMARY:{title}\r\nDESCRIPTION:{description}\r\nLOCATION:{location}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
            );

            let event_path = format!("events/{uid}.ics");
            caldav_put(&cfg, &event_path, &ical).await?;
            Ok(format!("✅ Événement créé dans Google Calendar\nTitre : {title}\nDébut : {start}\nFin   : {end}\nUID   : {uid}@osmozzz"))
        }

        "gcal_delete_event" => {
            let uid = args["uid"].as_str()
                .ok_or_else(|| "Paramètre 'uid' requis".to_string())?;

            // Cherche d'abord l'événement pour confirmer qu'il existe
            let (start, end) = date_range_xml(-365, 730); // cherche dans une large fenêtre
            let events = fetch_events_in_range(&cfg, &start, &end, "events/").await?;
            let event = events.iter().find(|e| e.uid == uid || e.uid.starts_with(uid));

            let event_uid = match event {
                Some(e) => e.uid.clone(),
                None => return Err(format!("Événement introuvable avec UID : {uid}")),
            };

            // Encode l'UID pour le chemin URL
            let event_path = format!("events/{}.ics", urlencoding::encode(&event_uid));
            caldav_delete(&cfg, &event_path).await?;
            Ok(format!("🗑️ Événement supprimé de Google Calendar\nUID : {event_uid}"))
        }

        "gcal_update_event" => {
            let uid = args["uid"].as_str()
                .ok_or_else(|| "Paramètre 'uid' requis".to_string())?;

            // 1. Récupère le .ics actuel via REPORT avec filtre UID
            let (event, ical_text) = fetch_event_by_uid(&cfg, uid).await?;

            // 2. Remplace les champs demandés dans le texte .ics
            let mut updated = ical_text.clone();

            if let Some(new_title) = args["title"].as_str() {
                // Remplace la ligne SUMMARY
                let new_line = format!("SUMMARY:{new_title}");
                if let Some(pos) = updated.find("SUMMARY:") {
                    let end = updated[pos..].find('\n').map(|n| pos + n).unwrap_or(updated.len());
                    updated.replace_range(pos..end, &new_line);
                } else {
                    // Insère avant END:VEVENT
                    updated = updated.replace("END:VEVENT", &format!("{new_line}\r\nEND:VEVENT"));
                }
            }

            if let Some(new_start) = args["start"].as_str() {
                let (dtstart_line, _) = if new_start.len() == 8 {
                    (format!("DTSTART;VALUE=DATE:{new_start}"), "")
                } else {
                    (format!("DTSTART:{new_start}"), "")
                };
                // Remplace DTSTART (avec ou sans paramètres comme TZID)
                let start_pos = updated.find("DTSTART");
                if let Some(pos) = start_pos {
                    let end = updated[pos..].find('\n').map(|n| pos + n).unwrap_or(updated.len());
                    updated.replace_range(pos..end, &dtstart_line);
                } else {
                    updated = updated.replace("END:VEVENT", &format!("{dtstart_line}\r\nEND:VEVENT"));
                }
            }

            if let Some(new_end) = args["end"].as_str() {
                let (dtend_line, _) = if new_end.len() == 8 {
                    (format!("DTEND;VALUE=DATE:{new_end}"), "")
                } else {
                    (format!("DTEND:{new_end}"), "")
                };
                let end_pos = updated.find("DTEND");
                if let Some(pos) = end_pos {
                    let line_end = updated[pos..].find('\n').map(|n| pos + n).unwrap_or(updated.len());
                    updated.replace_range(pos..line_end, &dtend_line);
                } else {
                    updated = updated.replace("END:VEVENT", &format!("{dtend_line}\r\nEND:VEVENT"));
                }
            }

            if let Some(new_desc) = args["description"].as_str() {
                let new_line = format!("DESCRIPTION:{new_desc}");
                if let Some(pos) = updated.find("DESCRIPTION:") {
                    let line_end = updated[pos..].find('\n').map(|n| pos + n).unwrap_or(updated.len());
                    updated.replace_range(pos..line_end, &new_line);
                } else {
                    updated = updated.replace("END:VEVENT", &format!("{new_line}\r\nEND:VEVENT"));
                }
            }

            if let Some(new_loc) = args["location"].as_str() {
                let new_line = format!("LOCATION:{new_loc}");
                if let Some(pos) = updated.find("LOCATION:") {
                    let line_end = updated[pos..].find('\n').map(|n| pos + n).unwrap_or(updated.len());
                    updated.replace_range(pos..line_end, &new_line);
                } else {
                    updated = updated.replace("END:VEVENT", &format!("{new_line}\r\nEND:VEVENT"));
                }
            }

            // Met à jour DTSTAMP
            let now_stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
            let new_stamp = format!("DTSTAMP:{now_stamp}");
            if let Some(pos) = updated.find("DTSTAMP:") {
                let line_end = updated[pos..].find('\n').map(|n| pos + n).unwrap_or(updated.len());
                updated.replace_range(pos..line_end, &new_stamp);
            }

            // 3. PUT le .ics mis à jour
            let event_path = format!("events/{}.ics", urlencoding::encode(&event.uid));
            caldav_put(&cfg, &event_path, &updated).await?;

            Ok(format!("✅ Événement mis à jour dans Google Calendar\nUID   : {}\nTitre : {}", event.uid, event.summary))
        }

        "gcal_get_event" => {
            let uid = args["uid"].as_str()
                .ok_or_else(|| "Paramètre 'uid' requis".to_string())?;

            let (event, _ical_text) = fetch_event_by_uid(&cfg, uid).await?;

            let mut details = format!(
                "📅 Détails de l'événement\n\nTitre       : {}\nDébut       : {}\nFin         : {}\nUID         : {}",
                event.summary,
                fmt_ical_date(&event.dtstart),
                fmt_ical_date(&event.dtend),
                event.uid
            );
            if !event.location.is_empty() {
                details.push_str(&format!("\nLieu        : {}", event.location));
            }
            if !event.description.is_empty() {
                let desc = event.description.replace("\\n", "\n").replace("\\,", ",");
                details.push_str(&format!("\nDescription :\n{}", desc));
            }
            Ok(details)
        }

        "gcal_get_free_busy" => {
            let start = args["start"].as_str()
                .ok_or_else(|| "Paramètre 'start' requis".to_string())?;
            let end = args["end"].as_str()
                .ok_or_else(|| "Paramètre 'end' requis".to_string())?;
            let calendar_path = args["calendar_path"].as_str().unwrap_or("events/");

            // Normalise les dates en format iCal datetime si besoin (YYYYMMDD → YYYYMMDDTHHmmssZ)
            let start_ical = if start.len() == 8 {
                format!("{start}T000000Z")
            } else {
                start.to_string()
            };
            let end_ical = if end.len() == 8 {
                format!("{end}T235959Z")
            } else {
                end.to_string()
            };

            // Récupère les événements qui chevauchent la plage via fetch_events_in_range
            let events = fetch_events_in_range(&cfg, &start_ical, &end_ical, calendar_path).await?;

            if events.is_empty() {
                Ok(format!(
                    "📅 Disponibilités du {} au {}\n\n✅ Aucun événement — vous êtes libre sur toute cette plage.",
                    fmt_ical_date(&start_ical),
                    fmt_ical_date(&end_ical)
                ))
            } else {
                Ok(format!(
                    "📅 Créneaux occupés du {} au {} ({} événement(s)) :\n\n{}",
                    fmt_ical_date(&start_ical),
                    fmt_ical_date(&end_ical),
                    events.len(),
                    fmt_events(&events)
                ))
            }
        }

        "gcal_add_attendee" => {
            let uid   = args["uid"].as_str()
                .ok_or_else(|| "Paramètre 'uid' requis".to_string())?;
            let email = args["email"].as_str()
                .ok_or_else(|| "Paramètre 'email' requis".to_string())?;
            let name  = args["name"].as_str().unwrap_or("");

            // 1. Récupère le .ics actuel
            let (event, ical_text) = fetch_event_by_uid(&cfg, uid).await?;

            // 2. Construit la ligne ATTENDEE
            let attendee_line = if name.is_empty() {
                format!("ATTENDEE:mailto:{email}")
            } else {
                format!("ATTENDEE;CN={name}:mailto:{email}")
            };

            // 3. Insère la ligne ATTENDEE avant END:VEVENT
            let updated = ical_text.replace(
                "END:VEVENT",
                &format!("{attendee_line}\r\nEND:VEVENT")
            );

            // 4. PUT le .ics mis à jour
            let event_path = format!("events/{}.ics", urlencoding::encode(&event.uid));
            caldav_put(&cfg, &event_path, &updated).await?;

            let attendee_display = if name.is_empty() {
                email.to_string()
            } else {
                format!("{name} <{email}>")
            };
            Ok(format!(
                "✅ Participant ajouté à l'événement\nÉvénement : {}\nParticipant : {}\nUID : {}",
                event.summary,
                attendee_display,
                event.uid
            ))
        }

        "gcal_list_upcoming_for_calendar" => {
            let calendar_path = args["calendar_path"].as_str()
                .ok_or_else(|| "Paramètre 'calendar_path' requis".to_string())?;
            let days = args["days"].as_i64().unwrap_or(7).clamp(1, 30);
            let (start, end) = date_range_xml(0, days);
            let events = fetch_events_in_range(&cfg, &start, &end, calendar_path).await?;
            Ok(format!(
                "📅 Prochains événements ({} jours) — calendrier '{}' :\n\n{}",
                days,
                calendar_path,
                fmt_events(&events)
            ))
        }

        other => Err(format!("Tool Google Calendar inconnu : {other}")),
    }
}
