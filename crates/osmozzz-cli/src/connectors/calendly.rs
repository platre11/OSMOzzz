/// Connecteur Calendly — REST API v2 (Personal Access Token).
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct CalendlyConfig {
    token: String,
}

impl CalendlyConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/calendly.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!("https://api.calendly.com/{}", path.trim_start_matches('/'))
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &CalendlyConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &CalendlyConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

// ─── Helper : récupérer l'URI de l'utilisateur courant ───────────────────────

async fn get_user_uri(cfg: &CalendlyConfig) -> Result<String, String> {
    let resp = get(cfg, &cfg.api("/users/me")).await?;
    resp["resource"]["uri"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            let msg = resp["message"].as_str().unwrap_or("token invalide ?");
            format!("Impossible de récupérer l'URI utilisateur Calendly : {msg}")
        })
}

// ─── Formatters ───────────────────────────────────────────────────────────────

fn format_event(e: &Value) -> String {
    let r         = if e["resource"].is_object() { &e["resource"] } else { e };
    let name      = r["name"].as_str().unwrap_or("—");
    let status    = r["status"].as_str().unwrap_or("—");
    let start     = r["start_time"].as_str().unwrap_or("—");
    let end       = r["end_time"].as_str().unwrap_or("—");
    let location  = r["location"]["location"].as_str()
        .or_else(|| r["location"]["join_url"].as_str())
        .unwrap_or("—");
    let uri       = r["uri"].as_str().unwrap_or("—");
    let uuid      = uri.split('/').last().unwrap_or("—");
    format!("• [{uuid}] {name}\n  Statut : {status} | De : {start} → {end}\n  Lieu   : {location}")
}

fn format_event_type(et: &Value) -> String {
    let r        = if et["resource"].is_object() { &et["resource"] } else { et };
    let name     = r["name"].as_str().unwrap_or("—");
    let duration = r["duration"].as_i64().unwrap_or(0);
    let active   = r["active"].as_bool().unwrap_or(false);
    let slug     = r["slug"].as_str().unwrap_or("—");
    let link     = r["scheduling_url"].as_str().unwrap_or("—");
    let active_str = if active { "actif" } else { "inactif" };
    format!("• {name} — {duration}min — {active_str}\n  Slug : {slug}\n  Lien : {link}")
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "calendly_list_scheduled_events",
            "description": "CALENDLY 📅 — Liste les événements planifiés (RDV à venir et passés). Retourne nom, statut, dates, lieu et UUID. Utiliser calendly_get_event pour le détail ou calendly_list_invitees pour voir qui a réservé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["active", "canceled"],
                        "description": "Filtrer par statut : active (RDV confirmés) ou canceled (annulés). Défaut : active.",
                        "default": "active"
                    },
                    "count": {
                        "type": "integer",
                        "default": 20,
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Nombre d'événements à retourner (défaut: 20)"
                    },
                    "min_start_time": {
                        "type": "string",
                        "description": "Date de début minimale ISO 8601 (ex: '2024-01-01T00:00:00Z') — optionnel"
                    },
                    "max_start_time": {
                        "type": "string",
                        "description": "Date de début maximale ISO 8601 — optionnel"
                    }
                }
            }
        }),
        json!({
            "name": "calendly_get_event",
            "description": "CALENDLY 📅 — Récupère le détail complet d'un événement planifié : nom, dates, lieu, statut, type d'événement. Utiliser calendly_list_scheduled_events pour obtenir l'UUID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uuid": { "type": "string", "description": "UUID de l'événement (dernière partie de l'URI, ex: 'abc123def456')" }
                },
                "required": ["uuid"]
            }
        }),
        json!({
            "name": "calendly_list_event_types",
            "description": "CALENDLY 📅 — Liste les types d'événements configurés (réunion 30min, appel découverte, etc.) avec leur durée, statut actif/inactif et lien de réservation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "active": {
                        "type": "boolean",
                        "default": true,
                        "description": "Retourner uniquement les types actifs (défaut: true)"
                    }
                }
            }
        }),
        json!({
            "name": "calendly_list_invitees",
            "description": "CALENDLY 📅 — Liste les invités qui ont réservé un événement spécifique : nom, email, statut, date de réservation. Utiliser calendly_list_scheduled_events pour obtenir l'UUID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "event_uuid": { "type": "string", "description": "UUID de l'événement" },
                    "status": {
                        "type": "string",
                        "enum": ["active", "canceled"],
                        "default": "active",
                        "description": "Filtrer par statut invité (défaut: active)"
                    }
                },
                "required": ["event_uuid"]
            }
        }),
        json!({
            "name": "calendly_get_user",
            "description": "CALENDLY 📅 — Récupère le profil de l'utilisateur connecté : nom, email, timezone, lien de planning et URL de profil Calendly.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "calendly_cancel_event",
            "description": "CALENDLY 📅 — Annule un événement planifié avec un motif optionnel. L'invité reçoit automatiquement un email de notification. Utiliser calendly_list_scheduled_events pour obtenir l'UUID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uuid":   { "type": "string", "description": "UUID de l'événement à annuler" },
                    "reason": { "type": "string", "description": "Motif de l'annulation (optionnel — envoyé à l'invité)" }
                },
                "required": ["uuid"]
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = CalendlyConfig::load()
        .ok_or_else(|| "Calendly non configuré — créer ~/.osmozzz/calendly.toml avec token".to_string())?;

    match name {
        "calendly_list_scheduled_events" => {
            let user_uri  = get_user_uri(&cfg).await?;
            let status    = args["status"].as_str().unwrap_or("active");
            let count     = args["count"].as_u64().unwrap_or(20);

            let mut url = format!(
                "{}?user={}&status={}&count={}&sort=start_time:asc",
                cfg.api("/scheduled_events"),
                urlencoding(&user_uri),
                status,
                count,
            );
            if let Some(min) = args["min_start_time"].as_str() {
                url.push_str(&format!("&min_start_time={}", urlencoding(min)));
            }
            if let Some(max) = args["max_start_time"].as_str() {
                url.push_str(&format!("&max_start_time={}", urlencoding(max)));
            }

            let resp   = get(&cfg, &url).await?;
            let events = resp["collection"].as_array().cloned().unwrap_or_default();

            if events.is_empty() {
                return Ok(format!("Aucun événement ({status})."));
            }

            let mut out = format!("{} événement(s) {} :\n\n", events.len(), status);
            for e in &events { out.push_str(&format_event(e)); out.push('\n'); }
            Ok(out.trim_end().to_string())
        }

        "calendly_get_event" => {
            let uuid = args["uuid"].as_str().ok_or("Paramètre 'uuid' requis")?;
            let url  = cfg.api(&format!("/scheduled_events/{uuid}"));
            let resp = get(&cfg, &url).await?;
            let r    = &resp["resource"];

            let name      = r["name"].as_str().unwrap_or("—");
            let status    = r["status"].as_str().unwrap_or("—");
            let start     = r["start_time"].as_str().unwrap_or("—");
            let end       = r["end_time"].as_str().unwrap_or("—");
            let created   = r["created_at"].as_str().unwrap_or("—");
            let location  = r["location"]["location"].as_str()
                .or_else(|| r["location"]["join_url"].as_str())
                .unwrap_or("Non spécifié");
            let loc_type  = r["location"]["type"].as_str().unwrap_or("—");

            let mut out = format!(
                "📅 {name}\nStatut  : {status}\nDébut   : {start}\nFin     : {end}\nLieu    : {location} ({loc_type})\nCréé le : {created}\n"
            );

            if let Some(members) = r["event_memberships"].as_array() {
                if !members.is_empty() {
                    out.push_str("\nOrganisateurs :\n");
                    for m in members {
                        let email = m["user_email"].as_str().unwrap_or("—");
                        let role  = m["user_role"].as_str().unwrap_or("—");
                        out.push_str(&format!("  • {email} ({role})\n"));
                    }
                }
            }

            Ok(out.trim_end().to_string())
        }

        "calendly_list_event_types" => {
            let user_uri = get_user_uri(&cfg).await?;
            let active   = args["active"].as_bool().unwrap_or(true);
            let url      = format!(
                "{}?user={}&active={}",
                cfg.api("/event_types"),
                urlencoding(&user_uri),
                active,
            );
            let resp  = get(&cfg, &url).await?;
            let types = resp["collection"].as_array().cloned().unwrap_or_default();

            if types.is_empty() {
                return Ok("Aucun type d'événement configuré.".to_string());
            }

            let mut out = format!("{} type(s) d'événement :\n\n", types.len());
            for et in &types { out.push_str(&format_event_type(et)); out.push('\n'); }
            Ok(out.trim_end().to_string())
        }

        "calendly_list_invitees" => {
            let event_uuid = args["event_uuid"].as_str().ok_or("Paramètre 'event_uuid' requis")?;
            let status     = args["status"].as_str().unwrap_or("active");
            let url        = format!(
                "{}/invitees?status={status}",
                cfg.api(&format!("/scheduled_events/{event_uuid}"))
            );
            let resp     = get(&cfg, &url).await?;
            let invitees = resp["collection"].as_array().cloned().unwrap_or_default();

            if invitees.is_empty() {
                return Ok(format!("Aucun invité ({status}) pour cet événement."));
            }

            let mut out = format!("{} invité(s) :\n\n", invitees.len());
            for inv in &invitees {
                let r         = if inv["resource"].is_object() { &inv["resource"] } else { inv };
                let name      = r["name"].as_str().unwrap_or("—");
                let email     = r["email"].as_str().unwrap_or("—");
                let status    = r["status"].as_str().unwrap_or("—");
                let created   = r["created_at"].as_str().unwrap_or("—");
                let timezone  = r["timezone"].as_str().unwrap_or("—");
                out.push_str(&format!(
                    "• {name} <{email}>\n  Statut : {status} | TZ : {timezone} | Réservé : {created}\n"
                ));
            }
            Ok(out.trim_end().to_string())
        }

        "calendly_get_user" => {
            let url  = cfg.api("/users/me");
            let resp = get(&cfg, &url).await?;
            let r    = &resp["resource"];

            let name          = r["name"].as_str().unwrap_or("—");
            let email         = r["email"].as_str().unwrap_or("—");
            let tz            = r["timezone"].as_str().unwrap_or("—");
            let slug          = r["slug"].as_str().unwrap_or("—");
            let scheduling    = r["scheduling_url"].as_str().unwrap_or("—");
            let created       = r["created_at"].as_str().unwrap_or("—");

            Ok(format!(
                "👤 {name}\nEmail     : {email}\nTimezone  : {tz}\nSlug      : {slug}\nLien      : {scheduling}\nCréé le   : {created}"
            ))
        }

        "calendly_cancel_event" => {
            let uuid   = args["uuid"].as_str().ok_or("Paramètre 'uuid' requis")?;
            let reason = args["reason"].as_str().unwrap_or("");
            let url    = cfg.api(&format!("/scheduled_events/{uuid}/cancellation"));
            let body   = if reason.is_empty() {
                json!({})
            } else {
                json!({ "reason": reason })
            };
            let resp = post_json(&cfg, &url, &body).await?;

            if resp["resource"]["status"].as_str() == Some("canceled") ||
               resp["resource"].is_object() {
                Ok(format!("Événement {uuid} annulé. L'invité a été notifié par email."))
            } else {
                let msg = resp["message"].as_str().unwrap_or("Vérifier le statut dans Calendly.");
                Ok(format!("Demande d'annulation envoyée. {msg}"))
            }
        }

        _ => Err(format!("Tool Calendly inconnu : {name}")),
    }
}

// ─── URL encoding minimal ─────────────────────────────────────────────────────

fn urlencoding(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                vec![c]
            }
            c => {
                let mut buf = [0u8; 4];
                let bytes = c.encode_utf8(&mut buf);
                bytes.bytes().flat_map(|b| {
                    vec!['%', char::from_digit((b >> 4) as u32, 16).unwrap_or('0').to_ascii_uppercase(),
                              char::from_digit((b & 0xf) as u32, 16).unwrap_or('0').to_ascii_uppercase()]
                }).collect()
            }
        })
        .collect()
}
