/// Connecteur Twilio — REST API 2010-04-01.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct TwilioConfig {
    account_sid: String,
    auth_token: String,
    from_number: Option<String>,
}

impl TwilioConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/twilio.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    /// Base URL for Account-scoped endpoints.
    fn base(&self) -> String {
        format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}",
            self.account_sid
        )
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

fn basic_auth(cfg: &TwilioConfig) -> String {
    use base64::Engine;
    let creds = format!("{}:{}", cfg.account_sid, cfg.auth_token);
    base64::engine::general_purpose::STANDARD.encode(creds.as_bytes())
}

async fn get(cfg: &TwilioConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Basic {}", basic_auth(cfg)))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_form(cfg: &TwilioConfig, url: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let body = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding_simple(k), urlencoding_simple(v)))
        .collect::<Vec<_>>()
        .join("&");
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Basic {}", basic_auth(cfg)))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn delete_req(cfg: &TwilioConfig, url: &str) -> Result<String, String> {
    let resp = reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Basic {}", basic_auth(cfg)))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok("Deleted successfully".to_string())
    } else {
        Err(format!("Delete failed: {}", resp.status()))
    }
}

/// Minimal percent-encoding for URL-encoded form values.
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}

// ─── Tool definitions ─────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── SMS / Messaging ──────────────────────────────────────────────────
        json!({
            "name": "twilio_send_sms",
            "description": "Send an SMS message via Twilio. Returns the message SID and status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to":   { "type": "string", "description": "Destination phone number in E.164 format (e.g. +14155552671)" },
                    "body": { "type": "string", "description": "Text content of the SMS (max 1600 chars)" },
                    "from": { "type": "string", "description": "Sender number (overrides default from_number in config)" }
                },
                "required": ["to", "body"]
            }
        }),
        json!({
            "name": "twilio_send_whatsapp",
            "description": "Send a WhatsApp message via Twilio. Both to/from are wrapped with whatsapp: prefix automatically.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to":   { "type": "string", "description": "Destination WhatsApp number in E.164 format (e.g. +14155552671)" },
                    "body": { "type": "string", "description": "Message body" },
                    "from": { "type": "string", "description": "Sender WhatsApp-enabled number (overrides config default)" }
                },
                "required": ["to", "body"]
            }
        }),
        json!({
            "name": "twilio_list_messages",
            "description": "List SMS/MMS messages from the Twilio account. Returns message SIDs, direction, status, and body preview.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Number of messages to return (default 20, max 100)" },
                    "from":  { "type": "string", "description": "Filter by sender number" },
                    "to":    { "type": "string", "description": "Filter by recipient number" }
                }
            }
        }),
        json!({
            "name": "twilio_get_message",
            "description": "Get full details of a specific Twilio message by its SID (SMxxxxxxx).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "message_sid": { "type": "string", "description": "Message SID (e.g. SM1234...)" }
                },
                "required": ["message_sid"]
            }
        }),
        // ── Voice ────────────────────────────────────────────────────────────
        json!({
            "name": "twilio_create_call",
            "description": "Initiate an outbound phone call via Twilio. Uses a TwiML URL to control call flow.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to":   { "type": "string", "description": "Destination phone number in E.164 format" },
                    "url":  { "type": "string", "description": "TwiML URL that controls call behavior (default: http://demo.twilio.com/docs/voice.xml)" },
                    "from": { "type": "string", "description": "Caller ID number (overrides config default)" }
                },
                "required": ["to"]
            }
        }),
        json!({
            "name": "twilio_list_calls",
            "description": "List calls in the Twilio account. Returns call SIDs, direction, status, duration, and timestamps.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit":  { "type": "integer", "description": "Number of calls to return (default 20, max 100)" },
                    "status": { "type": "string", "description": "Filter by status: queued, ringing, in-progress, canceled, completed, failed, busy, no-answer" }
                }
            }
        }),
        json!({
            "name": "twilio_get_call",
            "description": "Get full details of a specific Twilio call by its SID (CAxxxxxxx).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "call_sid": { "type": "string", "description": "Call SID (e.g. CA1234...)" }
                },
                "required": ["call_sid"]
            }
        }),
        // ── Phone Numbers ────────────────────────────────────────────────────
        json!({
            "name": "twilio_list_numbers",
            "description": "List phone numbers purchased on this Twilio account.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Number of numbers to return (default 20)" }
                }
            }
        }),
        json!({
            "name": "twilio_search_available_numbers",
            "description": "Search for available phone numbers to purchase on Twilio.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "country_code":    { "type": "string", "description": "ISO 3166-1 alpha-2 country code (e.g. US, GB, FR)" },
                    "area_code":       { "type": "string", "description": "Area code to filter by (e.g. 415)" },
                    "sms_enabled":     { "type": "boolean", "description": "Filter by SMS capability (default true)" },
                    "voice_enabled":   { "type": "boolean", "description": "Filter by voice capability (default true)" }
                },
                "required": ["country_code"]
            }
        }),
        json!({
            "name": "twilio_purchase_number",
            "description": "Purchase a phone number for the Twilio account. Use twilio_search_available_numbers first.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "phone_number": { "type": "string", "description": "E.164 phone number to purchase (e.g. +14155552671)" }
                },
                "required": ["phone_number"]
            }
        }),
        json!({
            "name": "twilio_release_number",
            "description": "Release (delete) a purchased Twilio phone number by its IncomingPhoneNumbers SID (PNxxxxxxx).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sid": { "type": "string", "description": "IncomingPhoneNumbers SID (e.g. PN1234...)" }
                },
                "required": ["sid"]
            }
        }),
        // ── Verify ───────────────────────────────────────────────────────────
        json!({
            "name": "twilio_create_verify_service",
            "description": "Create a new Twilio Verify service (used for OTP/verification flows). Returns service SID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "friendly_name": { "type": "string", "description": "Human-readable name for the verify service" }
                },
                "required": ["friendly_name"]
            }
        }),
        json!({
            "name": "twilio_list_verify_services",
            "description": "List all Twilio Verify services on the account.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "twilio_send_verification",
            "description": "Send a verification code (OTP) to a phone/email via a Twilio Verify service.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service_sid": { "type": "string", "description": "Verify service SID (VAxxxxxxx)" },
                    "to":          { "type": "string", "description": "Phone number or email to send the code to" },
                    "channel":     { "type": "string", "description": "Delivery channel: sms (default), call, or email" }
                },
                "required": ["service_sid", "to"]
            }
        }),
        json!({
            "name": "twilio_check_verification",
            "description": "Check whether a verification code (OTP) is correct for a given Twilio Verify service.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "service_sid": { "type": "string", "description": "Verify service SID (VAxxxxxxx)" },
                    "to":          { "type": "string", "description": "Phone number or email the code was sent to" },
                    "code":        { "type": "string", "description": "The OTP code entered by the user" }
                },
                "required": ["service_sid", "to", "code"]
            }
        }),
        // ── Lookup ───────────────────────────────────────────────────────────
        json!({
            "name": "twilio_lookup_phone_number",
            "description": "Look up carrier and caller-name information for any phone number via Twilio Lookup.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "phone_number": { "type": "string", "description": "E.164 phone number to look up (e.g. +14155552671)" }
                },
                "required": ["phone_number"]
            }
        }),
    ]
}

// ─── Handler ──────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = TwilioConfig::load()
        .ok_or_else(|| "Twilio not configured. Create ~/.osmozzz/twilio.toml with account_sid, auth_token, and from_number.".to_string())?;

    match name {
        "twilio_send_sms"              => send_sms(&cfg, args).await,
        "twilio_send_whatsapp"         => send_whatsapp(&cfg, args).await,
        "twilio_list_messages"         => list_messages(&cfg, args).await,
        "twilio_get_message"           => get_message(&cfg, args).await,
        "twilio_create_call"           => create_call(&cfg, args).await,
        "twilio_list_calls"            => list_calls(&cfg, args).await,
        "twilio_get_call"              => get_call(&cfg, args).await,
        "twilio_list_numbers"          => list_numbers(&cfg, args).await,
        "twilio_search_available_numbers" => search_available_numbers(&cfg, args).await,
        "twilio_purchase_number"       => purchase_number(&cfg, args).await,
        "twilio_release_number"        => release_number(&cfg, args).await,
        "twilio_create_verify_service" => create_verify_service(&cfg, args).await,
        "twilio_list_verify_services"  => list_verify_services(&cfg).await,
        "twilio_send_verification"     => send_verification(&cfg, args).await,
        "twilio_check_verification"    => check_verification(&cfg, args).await,
        "twilio_lookup_phone_number"   => lookup_phone_number(&cfg, args).await,
        _ => Err(format!("Unknown twilio tool: {}", name)),
    }
}

// ─── SMS / Messaging ──────────────────────────────────────────────────────────

async fn send_sms(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let to = args["to"].as_str().ok_or("Missing 'to'")?;
    let body = args["body"].as_str().ok_or("Missing 'body'")?;
    let from = args["from"]
        .as_str()
        .or_else(|| cfg.from_number.as_deref())
        .ok_or("Missing 'from' number — set from_number in twilio.toml or pass 'from' arg")?;

    let url = format!("{}/Messages.json", cfg.base());
    let resp = post_form(cfg, &url, &[("To", to), ("From", from), ("Body", body)]).await?;

    let sid    = resp["sid"].as_str().unwrap_or("?");
    let status = resp["status"].as_str().unwrap_or("?");
    let err    = resp["message"].as_str();

    if let Some(e) = err {
        return Err(format!("Twilio error: {}", e));
    }
    Ok(format!(
        "SMS sent\n  SID:    {}\n  Status: {}\n  To:     {}\n  From:   {}",
        sid, status, to, from
    ))
}

async fn send_whatsapp(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let to_raw = args["to"].as_str().ok_or("Missing 'to'")?;
    let body = args["body"].as_str().ok_or("Missing 'body'")?;
    let from_raw = args["from"]
        .as_str()
        .or_else(|| cfg.from_number.as_deref())
        .ok_or("Missing 'from' number — set from_number in twilio.toml or pass 'from' arg")?;

    let to   = format!("whatsapp:{}", to_raw);
    let from = format!("whatsapp:{}", from_raw);

    let url = format!("{}/Messages.json", cfg.base());
    let resp = post_form(cfg, &url, &[("To", &to), ("From", &from), ("Body", body)]).await?;

    let sid    = resp["sid"].as_str().unwrap_or("?");
    let status = resp["status"].as_str().unwrap_or("?");
    if let Some(e) = resp["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }
    Ok(format!(
        "WhatsApp message sent\n  SID:    {}\n  Status: {}\n  To:     {}\n  From:   {}",
        sid, status, to, from
    ))
}

async fn list_messages(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let limit = args["limit"].as_u64().unwrap_or(20).min(100);
    let mut url = format!("{}/Messages.json?PageSize={}", cfg.base(), limit);
    if let Some(from) = args["from"].as_str() {
        url.push_str(&format!("&From={}", urlencoding_simple(from)));
    }
    if let Some(to) = args["to"].as_str() {
        url.push_str(&format!("&To={}", urlencoding_simple(to)));
    }

    let resp = get(cfg, &url).await?;
    let messages = resp["messages"].as_array().cloned().unwrap_or_default();

    if messages.is_empty() {
        return Ok("No messages found.".to_string());
    }

    let mut out = format!("Messages ({}):\n", messages.len());
    for m in &messages {
        let sid       = m["sid"].as_str().unwrap_or("?");
        let direction = m["direction"].as_str().unwrap_or("?");
        let status    = m["status"].as_str().unwrap_or("?");
        let from      = m["from"].as_str().unwrap_or("?");
        let to        = m["to"].as_str().unwrap_or("?");
        let date      = m["date_sent"].as_str().unwrap_or(m["date_created"].as_str().unwrap_or("?"));
        let body      = m["body"].as_str().unwrap_or("");
        let preview   = if body.len() > 80 { format!("{}…", &body[..80]) } else { body.to_string() };
        out.push_str(&format!(
            "\n  [{}] {} | {} → {} | {} | {}\n    {}",
            sid, direction, from, to, status, date, preview
        ));
    }
    Ok(out)
}

async fn get_message(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let sid = args["message_sid"].as_str().ok_or("Missing 'message_sid'")?;
    let url = format!("{}/Messages/{}.json", cfg.base(), sid);
    let m = get(cfg, &url).await?;

    if let Some(e) = m["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }

    Ok(format!(
        "Message {}\n  Direction: {}\n  Status:    {}\n  From:      {}\n  To:        {}\n  Date:      {}\n  Price:     {} {}\n  Body:\n    {}",
        m["sid"].as_str().unwrap_or(sid),
        m["direction"].as_str().unwrap_or("?"),
        m["status"].as_str().unwrap_or("?"),
        m["from"].as_str().unwrap_or("?"),
        m["to"].as_str().unwrap_or("?"),
        m["date_sent"].as_str().unwrap_or(m["date_created"].as_str().unwrap_or("?")),
        m["price"].as_str().unwrap_or("?"),
        m["price_unit"].as_str().unwrap_or(""),
        m["body"].as_str().unwrap_or("")
    ))
}

// ─── Voice ────────────────────────────────────────────────────────────────────

async fn create_call(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let to = args["to"].as_str().ok_or("Missing 'to'")?;
    let url_twiml = args["url"].as_str().unwrap_or("http://demo.twilio.com/docs/voice.xml");
    let from = args["from"]
        .as_str()
        .or_else(|| cfg.from_number.as_deref())
        .ok_or("Missing 'from' number — set from_number in twilio.toml or pass 'from' arg")?;

    let url = format!("{}/Calls.json", cfg.base());
    let resp = post_form(cfg, &url, &[("To", to), ("From", from), ("Url", url_twiml)]).await?;

    if let Some(e) = resp["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }

    Ok(format!(
        "Call created\n  SID:      {}\n  Status:   {}\n  To:       {}\n  From:     {}\n  TwiML:    {}",
        resp["sid"].as_str().unwrap_or("?"),
        resp["status"].as_str().unwrap_or("?"),
        to,
        from,
        url_twiml
    ))
}

async fn list_calls(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let limit = args["limit"].as_u64().unwrap_or(20).min(100);
    let mut url = format!("{}/Calls.json?PageSize={}", cfg.base(), limit);
    if let Some(status) = args["status"].as_str() {
        url.push_str(&format!("&Status={}", urlencoding_simple(status)));
    }

    let resp = get(cfg, &url).await?;
    let calls = resp["calls"].as_array().cloned().unwrap_or_default();

    if calls.is_empty() {
        return Ok("No calls found.".to_string());
    }

    let mut out = format!("Calls ({}):\n", calls.len());
    for c in &calls {
        let sid       = c["sid"].as_str().unwrap_or("?");
        let direction = c["direction"].as_str().unwrap_or("?");
        let status    = c["status"].as_str().unwrap_or("?");
        let from      = c["from"].as_str().unwrap_or("?");
        let to        = c["to"].as_str().unwrap_or("?");
        let duration  = c["duration"].as_str().unwrap_or("0");
        let date      = c["start_time"].as_str().unwrap_or(c["date_created"].as_str().unwrap_or("?"));
        out.push_str(&format!(
            "\n  [{}] {} | {} → {} | {} | {}s | {}",
            sid, direction, from, to, status, duration, date
        ));
    }
    Ok(out)
}

async fn get_call(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let sid = args["call_sid"].as_str().ok_or("Missing 'call_sid'")?;
    let url = format!("{}/Calls/{}.json", cfg.base(), sid);
    let c = get(cfg, &url).await?;

    if let Some(e) = c["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }

    Ok(format!(
        "Call {}\n  Direction:  {}\n  Status:     {}\n  From:       {}\n  To:         {}\n  Start time: {}\n  End time:   {}\n  Duration:   {}s\n  Price:      {} {}",
        c["sid"].as_str().unwrap_or(sid),
        c["direction"].as_str().unwrap_or("?"),
        c["status"].as_str().unwrap_or("?"),
        c["from"].as_str().unwrap_or("?"),
        c["to"].as_str().unwrap_or("?"),
        c["start_time"].as_str().unwrap_or("?"),
        c["end_time"].as_str().unwrap_or("?"),
        c["duration"].as_str().unwrap_or("0"),
        c["price"].as_str().unwrap_or("?"),
        c["price_unit"].as_str().unwrap_or("")
    ))
}

// ─── Phone Numbers ────────────────────────────────────────────────────────────

async fn list_numbers(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let limit = args["limit"].as_u64().unwrap_or(20);
    let url = format!(
        "{}/IncomingPhoneNumbers.json?PageSize={}",
        cfg.base(),
        limit
    );
    let resp = get(cfg, &url).await?;
    let numbers = resp["incoming_phone_numbers"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if numbers.is_empty() {
        return Ok("No phone numbers found on this account.".to_string());
    }

    let mut out = format!("Purchased numbers ({}):\n", numbers.len());
    for n in &numbers {
        let sid          = n["sid"].as_str().unwrap_or("?");
        let number       = n["phone_number"].as_str().unwrap_or("?");
        let friendly     = n["friendly_name"].as_str().unwrap_or("");
        let capabilities = &n["capabilities"];
        let sms          = capabilities["sms"].as_bool().unwrap_or(false);
        let voice        = capabilities["voice"].as_bool().unwrap_or(false);
        let mms          = capabilities["mms"].as_bool().unwrap_or(false);
        out.push_str(&format!(
            "\n  [{}] {} ({}) — SMS:{} Voice:{} MMS:{}",
            sid, number, friendly, sms, voice, mms
        ));
    }
    Ok(out)
}

async fn search_available_numbers(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let country_code = args["country_code"].as_str().ok_or("Missing 'country_code'")?;

    let mut url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/AvailablePhoneNumbers/{}/Local.json",
        cfg.account_sid, country_code
    );

    let sms_enabled   = args["sms_enabled"].as_bool().unwrap_or(true);
    let voice_enabled = args["voice_enabled"].as_bool().unwrap_or(true);
    url.push_str(&format!(
        "?SmsEnabled={}&VoiceEnabled={}",
        sms_enabled, voice_enabled
    ));

    if let Some(area_code) = args["area_code"].as_str() {
        url.push_str(&format!("&AreaCode={}", urlencoding_simple(area_code)));
    }

    let resp = get(cfg, &url).await?;
    let numbers = resp["available_phone_numbers"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if numbers.is_empty() {
        return Ok(format!(
            "No available numbers found in {} with the given filters.",
            country_code
        ));
    }

    let mut out = format!("Available numbers in {} ({}):\n", country_code, numbers.len());
    for n in numbers.iter().take(20) {
        let number   = n["phone_number"].as_str().unwrap_or("?");
        let friendly = n["friendly_name"].as_str().unwrap_or("");
        let region   = n["region"].as_str().unwrap_or("");
        let caps     = &n["capabilities"];
        let sms      = caps["SMS"].as_bool().unwrap_or(false);
        let voice    = caps["voice"].as_bool().unwrap_or(false);
        out.push_str(&format!(
            "\n  {} ({}) — {} — SMS:{} Voice:{}",
            number, friendly, region, sms, voice
        ));
    }
    Ok(out)
}

async fn purchase_number(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let phone_number = args["phone_number"].as_str().ok_or("Missing 'phone_number'")?;
    let url = format!("{}/IncomingPhoneNumbers.json", cfg.base());
    let resp = post_form(cfg, &url, &[("PhoneNumber", phone_number)]).await?;

    if let Some(e) = resp["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }

    Ok(format!(
        "Number purchased\n  SID:           {}\n  Number:        {}\n  Friendly name: {}\n  Date created:  {}",
        resp["sid"].as_str().unwrap_or("?"),
        resp["phone_number"].as_str().unwrap_or("?"),
        resp["friendly_name"].as_str().unwrap_or("?"),
        resp["date_created"].as_str().unwrap_or("?")
    ))
}

async fn release_number(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let sid = args["sid"].as_str().ok_or("Missing 'sid'")?;
    let url = format!("{}/IncomingPhoneNumbers/{}.json", cfg.base(), sid);
    delete_req(cfg, &url).await
}

// ─── Verify ───────────────────────────────────────────────────────────────────

async fn create_verify_service(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let friendly_name = args["friendly_name"].as_str().ok_or("Missing 'friendly_name'")?;
    let url = "https://verify.twilio.com/v2/Services";
    let resp = post_form(cfg, url, &[("FriendlyName", friendly_name)]).await?;

    if let Some(e) = resp["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }

    Ok(format!(
        "Verify service created\n  SID:           {}\n  Friendly name: {}\n  Date created:  {}",
        resp["sid"].as_str().unwrap_or("?"),
        resp["friendly_name"].as_str().unwrap_or("?"),
        resp["date_created"].as_str().unwrap_or("?")
    ))
}

async fn list_verify_services(cfg: &TwilioConfig) -> Result<String, String> {
    let url = "https://verify.twilio.com/v2/Services";
    let resp = get(cfg, url).await?;
    let services = resp["services"].as_array().cloned().unwrap_or_default();

    if services.is_empty() {
        return Ok("No Verify services found.".to_string());
    }

    let mut out = format!("Verify services ({}):\n", services.len());
    for s in &services {
        let sid  = s["sid"].as_str().unwrap_or("?");
        let name = s["friendly_name"].as_str().unwrap_or("?");
        let date = s["date_created"].as_str().unwrap_or("?");
        out.push_str(&format!("\n  [{}] {} — created {}", sid, name, date));
    }
    Ok(out)
}

async fn send_verification(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let service_sid = args["service_sid"].as_str().ok_or("Missing 'service_sid'")?;
    let to          = args["to"].as_str().ok_or("Missing 'to'")?;
    let channel     = args["channel"].as_str().unwrap_or("sms");

    let url  = format!("https://verify.twilio.com/v2/Services/{}/Verifications", service_sid);
    let resp = post_form(cfg, &url, &[("To", to), ("Channel", channel)]).await?;

    if let Some(e) = resp["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }

    Ok(format!(
        "Verification sent\n  SID:     {}\n  Status:  {}\n  To:      {}\n  Channel: {}",
        resp["sid"].as_str().unwrap_or("?"),
        resp["status"].as_str().unwrap_or("?"),
        to,
        channel
    ))
}

async fn check_verification(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let service_sid = args["service_sid"].as_str().ok_or("Missing 'service_sid'")?;
    let to          = args["to"].as_str().ok_or("Missing 'to'")?;
    let code        = args["code"].as_str().ok_or("Missing 'code'")?;

    let url  = format!("https://verify.twilio.com/v2/Services/{}/VerificationCheck", service_sid);
    let resp = post_form(cfg, &url, &[("To", to), ("Code", code)]).await?;

    if let Some(e) = resp["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }

    let status = resp["status"].as_str().unwrap_or("?");
    let valid  = resp["valid"].as_bool().unwrap_or(false);

    Ok(format!(
        "Verification check\n  Status: {}\n  Valid:  {}\n  To:     {}",
        status, valid, to
    ))
}

// ─── Lookup ───────────────────────────────────────────────────────────────────

async fn lookup_phone_number(cfg: &TwilioConfig, args: &Value) -> Result<String, String> {
    let phone_number = args["phone_number"].as_str().ok_or("Missing 'phone_number'")?;
    let encoded      = urlencoding_simple(phone_number);
    let url = format!(
        "https://lookups.twilio.com/v1/PhoneNumbers/{}?Type=carrier&Type=caller-name",
        encoded
    );
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["message"].as_str() {
        return Err(format!("Twilio error: {}", e));
    }

    let number     = resp["phone_number"].as_str().unwrap_or(phone_number);
    let national   = resp["national_format"].as_str().unwrap_or("?");
    let country    = resp["country_code"].as_str().unwrap_or("?");
    let line_type  = resp["carrier"]["type"].as_str().unwrap_or("?");
    let carrier    = resp["carrier"]["name"].as_str().unwrap_or("?");
    let caller     = resp["caller_name"]["caller_name"].as_str().unwrap_or("?");
    let caller_type = resp["caller_name"]["caller_type"].as_str().unwrap_or("?");

    Ok(format!(
        "Phone number lookup: {}\n  National format: {}\n  Country:         {}\n  Line type:       {}\n  Carrier:         {}\n  Caller name:     {} ({})",
        number, national, country, line_type, carrier, caller, caller_type
    ))
}
