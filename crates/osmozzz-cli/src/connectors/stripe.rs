/// Connecteur Stripe — REST API v1 officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct StripeConfig { secret_key: String }

impl StripeConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/stripe.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
    fn api(&self, path: &str) -> String {
        format!("https://api.stripe.com/v1/{}", path.trim_start_matches('/'))
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &StripeConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.secret_key))
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post_form(cfg: &StripeConfig, url: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let body = params.iter()
        .map(|(k, v)| format!("{}={}", urlencoding_simple(k), urlencoding_simple(v)))
        .collect::<Vec<_>>()
        .join("&");
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.secret_key))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post_form_raw(cfg: &StripeConfig, url: &str, body: String) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.secret_key))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn delete_req(cfg: &StripeConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bearer {}", cfg.secret_key))
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

/// Minimal percent-encoding for form values (encodes space, &, =, +, %).
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}

// ─── Formatters ──────────────────────────────────────────────────────────────

/// Convertit un montant en centimes vers un affichage lisible (ex: 1000 EUR → "10.00 EUR").
fn fmt_amount(amount: i64, currency: &str) -> String {
    let cur = currency.to_uppercase();
    // Most currencies use 2 decimal places; JPY and some others use 0.
    let zero_decimal = matches!(cur.as_str(), "JPY" | "KRW" | "VND" | "CLP" | "GNF" | "MGA" | "PYG" | "RWF" | "UGX" | "XAF" | "XOF");
    if zero_decimal {
        format!("{} {}", amount, cur)
    } else {
        format!("{:.2} {}", amount as f64 / 100.0, cur)
    }
}

/// Convertit un timestamp Unix en "YYYY-MM-DD HH:MM UTC".
fn fmt_ts(ts: i64) -> String {
    use chrono::{TimeZone, Utc};
    match Utc.timestamp_opt(ts, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M UTC").to_string(),
        _ => ts.to_string(),
    }
}

/// Extrait un i64 d'une Value puis appelle fmt_ts.
fn fmt_ts_val(v: &Value) -> String {
    match v.as_i64() {
        Some(ts) => fmt_ts(ts),
        None     => "—".to_string(),
    }
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"stripe_get_balance","description":"STRIPE 💳 — Affiche le solde du compte Stripe : montants disponibles et en attente par devise. Point de départ pour vérifier l'état financier du compte.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"stripe_list_customers","description":"STRIPE 💳 — Liste les clients Stripe avec id, nom, email, date de création et téléphone. Filtrage optionnel par email. Retourne les 10 derniers par défaut (max 100).","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","default":10,"minimum":1,"maximum":100,"description":"Nombre de clients à retourner (défaut: 10)"},"email":{"type":"string","description":"Filtrer par adresse email exacte (optionnel)"}}}}),
        json!({"name":"stripe_get_customer","description":"STRIPE 💳 — Récupère le détail complet d'un client Stripe : nom, email, téléphone, devise, solde, adresse, source de paiement par défaut. Utiliser stripe_list_customers pour obtenir le customer_id.","inputSchema":{"type":"object","properties":{"customer_id":{"type":"string","description":"ID du client Stripe (ex: cus_xxx)"}},"required":["customer_id"]}}),
        json!({"name":"stripe_create_customer","description":"STRIPE 💳 — Crée un nouveau client Stripe. Seul l'email est obligatoire. Retourne l'id du client créé.","inputSchema":{"type":"object","properties":{"email":{"type":"string","description":"Adresse email du client (obligatoire)"},"name":{"type":"string","description":"Nom complet du client (optionnel)"},"phone":{"type":"string","description":"Numéro de téléphone (optionnel)"},"description":{"type":"string","description":"Description interne (optionnel)"}},"required":["email"]}}),
        json!({"name":"stripe_list_payment_intents","description":"STRIPE 💳 — Liste les PaymentIntents avec id, montant+devise, statut, date et description. Filtrage optionnel par client. Retourne les 10 derniers par défaut.","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","default":10,"minimum":1,"maximum":100,"description":"Nombre de PaymentIntents à retourner (défaut: 10)"},"customer_id":{"type":"string","description":"Filtrer par ID client Stripe (optionnel)"}}}}),
        json!({"name":"stripe_get_payment_intent","description":"STRIPE 💳 — Récupère le détail complet d'un PaymentIntent : montant, devise, statut, client, description, méthode de paiement, date, dernière erreur. Utiliser stripe_list_payment_intents pour obtenir l'id.","inputSchema":{"type":"object","properties":{"payment_intent_id":{"type":"string","description":"ID du PaymentIntent (ex: pi_xxx)"}},"required":["payment_intent_id"]}}),
        json!({"name":"stripe_list_subscriptions","description":"STRIPE 💳 — Liste les abonnements avec id, client, statut, fin de période en cours et plan/prix. Filtrage optionnel par client et statut (active/canceled/past_due/all).","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","default":10,"minimum":1,"maximum":100,"description":"Nombre d'abonnements à retourner (défaut: 10)"},"customer_id":{"type":"string","description":"Filtrer par ID client (optionnel)"},"status":{"type":"string","description":"Filtrer par statut : active, canceled, past_due, trialing, all (défaut: active)"}}}}),
        json!({"name":"stripe_get_subscription","description":"STRIPE 💳 — Récupère le détail complet d'un abonnement : articles (produits/prix), dates d'essai, annulation en fin de période. Utiliser stripe_list_subscriptions pour obtenir l'id.","inputSchema":{"type":"object","properties":{"subscription_id":{"type":"string","description":"ID de l'abonnement Stripe (ex: sub_xxx)"}},"required":["subscription_id"]}}),
        json!({"name":"stripe_list_invoices","description":"STRIPE 💳 — Liste les factures avec id, client, montant dû+devise, statut et date d'échéance. Filtrage optionnel par client et statut (draft/open/paid/uncollectible/void).","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","default":10,"minimum":1,"maximum":100,"description":"Nombre de factures à retourner (défaut: 10)"},"customer_id":{"type":"string","description":"Filtrer par ID client (optionnel)"},"status":{"type":"string","description":"Filtrer par statut : draft, open, paid, uncollectible, void (optionnel)"}}}}),
        json!({"name":"stripe_get_invoice","description":"STRIPE 💳 — Récupère le détail complet d'une facture : lignes, sous-total, taxe, total, statut, client, date d'échéance et lien vers la facture hébergée. Utiliser stripe_list_invoices pour obtenir l'id.","inputSchema":{"type":"object","properties":{"invoice_id":{"type":"string","description":"ID de la facture Stripe (ex: in_xxx)"}},"required":["invoice_id"]}}),
        json!({"name":"stripe_list_events","description":"STRIPE 💳 — Liste les événements Stripe (webhook events) avec id, type, date, version API et mode live/test. Filtrage optionnel par type et par nombre d'heures (since_hours).","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","default":20,"minimum":1,"maximum":100,"description":"Nombre d'événements à retourner (défaut: 20)"},"event_type":{"type":"string","description":"Filtrer par type d'événement (ex: payment_intent.succeeded) — optionnel"},"since_hours":{"type":"integer","description":"Retourner uniquement les événements des N dernières heures (optionnel)"}}}}),
        json!({"name":"stripe_get_event","description":"STRIPE 💳 — Récupère le détail complet d'un événement Stripe : type, date, résumé des données. Utiliser stripe_list_events pour obtenir l'event_id.","inputSchema":{"type":"object","properties":{"event_id":{"type":"string","description":"ID de l'événement Stripe (ex: evt_xxx)"}},"required":["event_id"]}}),
        json!({"name":"stripe_list_webhooks","description":"STRIPE 💳 — Liste les endpoints webhook Stripe configurés avec id, URL, statut (enabled/disabled) et événements abonnés.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"stripe_get_webhook","description":"STRIPE 💳 — Récupère le détail complet d'un webhook Stripe : URL, statut, liste complète des événements abonnés. Utiliser stripe_list_webhooks pour obtenir le webhook_id.","inputSchema":{"type":"object","properties":{"webhook_id":{"type":"string","description":"ID du webhook endpoint (ex: we_xxx)"}},"required":["webhook_id"]}}),
        json!({"name":"stripe_create_webhook","description":"STRIPE 💳 — Crée un nouvel endpoint webhook Stripe. Les événements doivent être fournis sous forme de liste séparée par des virgules (ex: payment_intent.succeeded,customer.created).","inputSchema":{"type":"object","properties":{"url":{"type":"string","description":"URL HTTPS du endpoint webhook (ex: https://monsite.com/stripe/webhook)"},"events":{"type":"string","description":"Liste des événements séparés par des virgules (ex: payment_intent.succeeded,customer.created)"}},"required":["url","events"]}}),
        json!({"name":"stripe_delete_webhook","description":"STRIPE 💳 — Supprime un endpoint webhook Stripe. Utiliser stripe_list_webhooks pour obtenir le webhook_id.","inputSchema":{"type":"object","properties":{"webhook_id":{"type":"string","description":"ID du webhook endpoint à supprimer (ex: we_xxx)"}},"required":["webhook_id"]}}),
        json!({"name":"stripe_list_payouts","description":"STRIPE 💳 — Liste les virements (payouts) vers le compte bancaire avec id, montant+devise, statut, date d'arrivée et description. Filtrage optionnel par statut (paid/pending/in_transit/canceled/failed).","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","default":10,"minimum":1,"maximum":100,"description":"Nombre de virements à retourner (défaut: 10)"},"status":{"type":"string","description":"Filtrer par statut : paid, pending, in_transit, canceled, failed (optionnel)"}}}}),
        json!({"name":"stripe_get_payout","description":"STRIPE 💳 — Récupère le détail complet d'un virement Stripe : montant, devise, statut, date d'arrivée, description, méthode et type de source. Utiliser stripe_list_payouts pour obtenir le payout_id.","inputSchema":{"type":"object","properties":{"payout_id":{"type":"string","description":"ID du virement Stripe (ex: po_xxx)"}},"required":["payout_id"]}}),
        json!({"name":"stripe_search_customers","description":"STRIPE 💳 — Recherche des clients Stripe par requête Stripe Search (ex: email:'test@example.com' ou name:'Jean'). Retourne id, nom, email, téléphone et date de création.","inputSchema":{"type":"object","properties":{"query":{"type":"string","description":"Requête Stripe Search (ex: email:'test@example.com', name:'Jean')"},"limit":{"type":"integer","default":10,"minimum":1,"maximum":100,"description":"Nombre de résultats (défaut: 10)"}},"required":["query"]}}),
        json!({"name":"stripe_update_customer","description":"STRIPE 💳 — Met à jour les informations d'un client Stripe existant. Seul customer_id est obligatoire ; seuls les champs fournis sont modifiés.","inputSchema":{"type":"object","properties":{"customer_id":{"type":"string","description":"ID du client Stripe à modifier (ex: cus_xxx)"},"email":{"type":"string","description":"Nouvelle adresse email (optionnel)"},"name":{"type":"string","description":"Nouveau nom complet (optionnel)"},"phone":{"type":"string","description":"Nouveau numéro de téléphone (optionnel)"},"description":{"type":"string","description":"Nouvelle description interne (optionnel)"}},"required":["customer_id"]}}),
        json!({"name":"stripe_list_products","description":"STRIPE 💳 — Liste les produits Stripe avec id, nom, description et statut actif/inactif. Filtrage optionnel par statut actif.","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","default":20,"minimum":1,"maximum":100,"description":"Nombre de produits à retourner (défaut: 20)"},"active":{"type":"boolean","default":true,"description":"Filtrer les produits actifs uniquement (défaut: true)"}}}}),
        json!({"name":"stripe_create_product","description":"STRIPE 💳 — Crée un nouveau produit Stripe. Retourne l'id et le nom du produit créé.","inputSchema":{"type":"object","properties":{"name":{"type":"string","description":"Nom du produit (obligatoire)"},"description":{"type":"string","description":"Description du produit (optionnel)"}},"required":["name"]}}),
        json!({"name":"stripe_list_prices","description":"STRIPE 💳 — Liste les prix Stripe avec id, produit, montant+devise, type (recurring/one_time) et intervalle si récurrent. Filtrage optionnel par produit.","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","default":20,"minimum":1,"maximum":100,"description":"Nombre de prix à retourner (défaut: 20)"},"product_id":{"type":"string","description":"Filtrer par ID produit (optionnel)"}}}}),
        json!({"name":"stripe_create_price","description":"STRIPE 💳 — Crée un nouveau prix Stripe pour un produit. Si interval est fourni (month/year/week/day), crée un prix récurrent ; sinon crée un prix one_time.","inputSchema":{"type":"object","properties":{"product_id":{"type":"string","description":"ID du produit Stripe (ex: prod_xxx)"},"amount":{"type":"integer","description":"Montant en centimes (ex: 1000 = 10.00)"},"currency":{"type":"string","description":"Code devise ISO (ex: eur, usd)"},"interval":{"type":"string","enum":["month","year","week","day"],"description":"Intervalle de récurrence (optionnel — si absent, prix one_time)"}},"required":["product_id","amount","currency"]}}),
        json!({"name":"stripe_create_subscription","description":"STRIPE 💳 — Crée un abonnement Stripe pour un client avec un prix donné. Optionnellement avec une période d'essai.","inputSchema":{"type":"object","properties":{"customer_id":{"type":"string","description":"ID du client Stripe (ex: cus_xxx)"},"price_id":{"type":"string","description":"ID du prix Stripe (ex: price_xxx)"},"trial_days":{"type":"integer","description":"Nombre de jours d'essai gratuit (optionnel)"}},"required":["customer_id","price_id"]}}),
        json!({"name":"stripe_create_payment_link","description":"STRIPE 💳 — Crée un lien de paiement Stripe pour un prix donné. Retourne l'id et l'URL du lien de paiement.","inputSchema":{"type":"object","properties":{"price_id":{"type":"string","description":"ID du prix Stripe (ex: price_xxx)"},"quantity":{"type":"integer","default":1,"minimum":1,"description":"Quantité (défaut: 1)"}},"required":["price_id"]}}),
        json!({"name":"stripe_create_checkout_session","description":"STRIPE 💳 — Crée une session Checkout Stripe et retourne l'URL vers laquelle rediriger l'utilisateur pour le paiement. Modes : payment (paiement unique), subscription (abonnement), setup (enregistrement CB).","inputSchema":{"type":"object","properties":{"price_id":{"type":"string","description":"ID du prix Stripe (ex: price_xxx)"},"success_url":{"type":"string","description":"URL de redirection après paiement réussi"},"cancel_url":{"type":"string","description":"URL de redirection en cas d'annulation (optionnel — défaut: même que success_url)"},"mode":{"type":"string","enum":["payment","subscription","setup"],"default":"payment","description":"Mode de la session (défaut: payment)"},"quantity":{"type":"integer","default":1,"minimum":1,"description":"Quantité (défaut: 1)"}},"required":["price_id","success_url"]}}),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = match StripeConfig::load() {
        Some(c) => c,
        None    => return Ok("Stripe non configuré (stripe.toml manquant)".to_string()),
    };

    match name {
        // ── 1. stripe_get_balance ─────────────────────────────────────────────
        "stripe_get_balance" => {
            let url  = cfg.api("balance");
            let data = get(&cfg, &url).await?;

            let available = data["available"].as_array().cloned().unwrap_or_default();
            let pending   = data["pending"].as_array().cloned().unwrap_or_default();

            let fmt_entries = |entries: &[Value]| -> String {
                if entries.is_empty() { return "  (aucun)".to_string(); }
                entries.iter().map(|e| {
                    let amount   = e["amount"].as_i64().unwrap_or(0);
                    let currency = e["currency"].as_str().unwrap_or("usd");
                    format!("  • {}", fmt_amount(amount, currency))
                }).collect::<Vec<_>>().join("\n")
            };

            Ok(format!(
                "STRIPE 💳 — Solde du compte\n\nDisponible :\n{}\n\nEn attente :\n{}",
                fmt_entries(&available),
                fmt_entries(&pending)
            ))
        }

        // ── 2. stripe_list_customers ──────────────────────────────────────────
        "stripe_list_customers" => {
            let limit = args["limit"].as_u64().unwrap_or(10).min(100);
            let email = args["email"].as_str().unwrap_or("").to_string();

            let mut url = format!("{}?limit={}", cfg.api("customers"), limit);
            if !email.is_empty() {
                url.push_str(&format!("&email={}", urlencoding_simple(&email)));
            }

            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun client trouvé.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Clients ({}) :\n\n{}",
                items.len(),
                items.iter().map(|c| {
                    let id      = c["id"].as_str().unwrap_or("?");
                    let name    = c["name"].as_str().unwrap_or("—");
                    let mail    = c["email"].as_str().unwrap_or("—");
                    let phone   = c["phone"].as_str().unwrap_or("—");
                    let created = fmt_ts_val(&c["created"]);
                    format!("👤 {name}\n  id={id} · email={mail}\n  tél={phone} · créé={created}")
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── 3. stripe_get_customer ────────────────────────────────────────────
        "stripe_get_customer" => {
            let customer_id = args["customer_id"].as_str().ok_or("Missing param: customer_id")?.to_string();
            let url  = cfg.api(&format!("customers/{}", customer_id));
            let c    = get(&cfg, &url).await?;

            let name        = c["name"].as_str().unwrap_or("—");
            let email       = c["email"].as_str().unwrap_or("—");
            let phone       = c["phone"].as_str().unwrap_or("—");
            let currency    = c["currency"].as_str().unwrap_or("—");
            let balance     = c["balance"].as_i64().unwrap_or(0);
            let description = c["description"].as_str().unwrap_or("—");
            let created     = fmt_ts_val(&c["created"]);
            let def_source  = c["default_source"].as_str().unwrap_or("—");
            let addr_line1  = c["address"]["line1"].as_str().unwrap_or("—");
            let addr_city   = c["address"]["city"].as_str().unwrap_or("—");
            let addr_country= c["address"]["country"].as_str().unwrap_or("—");

            Ok(format!(
                "STRIPE 💳 — Client {customer_id}\n  nom={name}\n  email={email}\n  tél={phone}\n  devise={currency} · solde={}\n  description={description}\n  créé={created}\n  source_défaut={def_source}\n  adresse={addr_line1}, {addr_city}, {addr_country}",
                fmt_amount(balance, currency)
            ))
        }

        // ── 4. stripe_create_customer ─────────────────────────────────────────
        "stripe_create_customer" => {
            let email       = args["email"].as_str().ok_or("Missing param: email")?.to_string();
            let name        = args["name"].as_str().unwrap_or("").to_string();
            let phone       = args["phone"].as_str().unwrap_or("").to_string();
            let description = args["description"].as_str().unwrap_or("").to_string();

            let mut params: Vec<(&str, &str)> = vec![];
            // Owned strings needed to outlive the params vec
            let email_s = email.clone();
            let name_s  = name.clone();
            let phone_s = phone.clone();
            let desc_s  = description.clone();

            params.push(("email", &email_s));
            if !name_s.is_empty()  { params.push(("name", &name_s)); }
            if !phone_s.is_empty() { params.push(("phone", &phone_s)); }
            if !desc_s.is_empty()  { params.push(("description", &desc_s)); }

            let url  = cfg.api("customers");
            let data = post_form(&cfg, &url, &params).await?;

            let id      = data["id"].as_str().unwrap_or("?");
            let created = fmt_ts_val(&data["created"]);
            Ok(format!("STRIPE 💳 — ✅ Client créé.\n  id={id} · email={email} · créé={created}"))
        }

        // ── 5. stripe_list_payment_intents ────────────────────────────────────
        "stripe_list_payment_intents" => {
            let limit       = args["limit"].as_u64().unwrap_or(10).min(100);
            let customer_id = args["customer_id"].as_str().unwrap_or("").to_string();

            let mut url = format!("{}?limit={}", cfg.api("payment_intents"), limit);
            if !customer_id.is_empty() {
                url.push_str(&format!("&customer={}", urlencoding_simple(&customer_id)));
            }

            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun PaymentIntent trouvé.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — PaymentIntents ({}) :\n\n{}",
                items.len(),
                items.iter().map(|pi| {
                    let id       = pi["id"].as_str().unwrap_or("?");
                    let amount   = pi["amount"].as_i64().unwrap_or(0);
                    let currency = pi["currency"].as_str().unwrap_or("usd");
                    let status   = pi["status"].as_str().unwrap_or("?");
                    let created  = fmt_ts_val(&pi["created"]);
                    let desc     = pi["description"].as_str().unwrap_or("—");
                    format!("💳 {} · {}\n  id={id} · statut={status}\n  créé={created}\n  description={desc}",
                        fmt_amount(amount, currency), currency.to_uppercase())
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── 6. stripe_get_payment_intent ──────────────────────────────────────
        "stripe_get_payment_intent" => {
            let pi_id = args["payment_intent_id"].as_str().ok_or("Missing param: payment_intent_id")?.to_string();
            let url   = cfg.api(&format!("payment_intents/{}", pi_id));
            let pi    = get(&cfg, &url).await?;

            let amount      = pi["amount"].as_i64().unwrap_or(0);
            let currency    = pi["currency"].as_str().unwrap_or("usd");
            let status      = pi["status"].as_str().unwrap_or("?");
            let customer    = pi["customer"].as_str().unwrap_or("—");
            let description = pi["description"].as_str().unwrap_or("—");
            let pm          = pi["payment_method"].as_str().unwrap_or("—");
            let created     = fmt_ts_val(&pi["created"]);
            let last_error  = pi["last_payment_error"]["message"].as_str().unwrap_or("—");

            Ok(format!(
                "STRIPE 💳 — PaymentIntent {pi_id}\n  montant={}\n  statut={status}\n  client={customer}\n  description={description}\n  méthode_paiement={pm}\n  créé={created}\n  dernière_erreur={last_error}",
                fmt_amount(amount, currency)
            ))
        }

        // ── 7. stripe_list_subscriptions ─────────────────────────────────────
        "stripe_list_subscriptions" => {
            let limit       = args["limit"].as_u64().unwrap_or(10).min(100);
            let customer_id = args["customer_id"].as_str().unwrap_or("").to_string();
            let status_f    = args["status"].as_str().unwrap_or("active").to_string();

            let mut url = format!("{}?limit={}", cfg.api("subscriptions"), limit);
            if !customer_id.is_empty() {
                url.push_str(&format!("&customer={}", urlencoding_simple(&customer_id)));
            }
            // "all" means no status filter; otherwise pass the status
            if status_f != "all" && !status_f.is_empty() {
                url.push_str(&format!("&status={}", urlencoding_simple(&status_f)));
            }

            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun abonnement trouvé.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Abonnements ({}) :\n\n{}",
                items.len(),
                items.iter().map(|sub| {
                    let id         = sub["id"].as_str().unwrap_or("?");
                    let customer   = sub["customer"].as_str().unwrap_or("—");
                    let status     = sub["status"].as_str().unwrap_or("?");
                    let period_end = fmt_ts_val(&sub["current_period_end"]);
                    // Price / plan info (first item)
                    let price_id   = sub["items"]["data"][0]["price"]["id"].as_str().unwrap_or("—");
                    let product_id = sub["items"]["data"][0]["price"]["product"].as_str().unwrap_or("—");
                    format!("🔄 {id}\n  client={customer} · statut={status}\n  fin_période={period_end}\n  prix={price_id} · produit={product_id}")
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── 9. stripe_get_subscription ────────────────────────────────────────
        "stripe_get_subscription" => {
            let sub_id = args["subscription_id"].as_str().ok_or("Missing param: subscription_id")?.to_string();
            let url    = cfg.api(&format!("subscriptions/{}", sub_id));
            let sub    = get(&cfg, &url).await?;

            let customer        = sub["customer"].as_str().unwrap_or("—");
            let status          = sub["status"].as_str().unwrap_or("?");
            let created         = fmt_ts_val(&sub["created"]);
            let period_start    = fmt_ts_val(&sub["current_period_start"]);
            let period_end      = fmt_ts_val(&sub["current_period_end"]);
            let cancel_at_end   = sub["cancel_at_period_end"].as_bool().unwrap_or(false);
            let trial_start     = sub["trial_start"].as_i64().map(fmt_ts).unwrap_or_else(|| "—".to_string());
            let trial_end       = sub["trial_end"].as_i64().map(fmt_ts).unwrap_or_else(|| "—".to_string());

            let items = sub["items"]["data"].as_array().cloned().unwrap_or_default();
            let items_str = items.iter().map(|item| {
                let price_id   = item["price"]["id"].as_str().unwrap_or("?");
                let product    = item["price"]["product"].as_str().unwrap_or("?");
                let amount     = item["price"]["unit_amount"].as_i64().unwrap_or(0);
                let currency   = item["price"]["currency"].as_str().unwrap_or("usd");
                let interval   = item["price"]["recurring"]["interval"].as_str().unwrap_or("?");
                format!("  • {price_id} · produit={product} · {} / {interval}", fmt_amount(amount, currency))
            }).collect::<Vec<_>>().join("\n");

            Ok(format!(
                "STRIPE 💳 — Abonnement {sub_id}\n  client={customer} · statut={status}\n  créé={created}\n  période={period_start} → {period_end}\n  annulation_fin_période={cancel_at_end}\n  essai={trial_start} → {trial_end}\n\nArticles :\n{}",
                if items_str.is_empty() { "  (aucun)".to_string() } else { items_str }
            ))
        }

        // ── 10. stripe_list_invoices ──────────────────────────────────────────
        "stripe_list_invoices" => {
            let limit       = args["limit"].as_u64().unwrap_or(10).min(100);
            let customer_id = args["customer_id"].as_str().unwrap_or("").to_string();
            let status_f    = args["status"].as_str().unwrap_or("").to_string();

            let mut url = format!("{}?limit={}", cfg.api("invoices"), limit);
            if !customer_id.is_empty() {
                url.push_str(&format!("&customer={}", urlencoding_simple(&customer_id)));
            }
            if !status_f.is_empty() {
                url.push_str(&format!("&status={}", urlencoding_simple(&status_f)));
            }

            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucune facture trouvée.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Factures ({}) :\n\n{}",
                items.len(),
                items.iter().map(|inv| {
                    let id        = inv["id"].as_str().unwrap_or("?");
                    let customer  = inv["customer"].as_str().unwrap_or("—");
                    let amount    = inv["amount_due"].as_i64().unwrap_or(0);
                    let currency  = inv["currency"].as_str().unwrap_or("usd");
                    let status    = inv["status"].as_str().unwrap_or("?");
                    let due_date  = inv["due_date"].as_i64().map(fmt_ts).unwrap_or_else(|| "—".to_string());
                    format!("🧾 {id}\n  client={customer} · {}\n  statut={status} · échéance={due_date}",
                        fmt_amount(amount, currency))
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── 12. stripe_get_invoice ────────────────────────────────────────────
        "stripe_get_invoice" => {
            let invoice_id = args["invoice_id"].as_str().ok_or("Missing param: invoice_id")?.to_string();
            let url        = cfg.api(&format!("invoices/{}", invoice_id));
            let inv        = get(&cfg, &url).await?;

            let customer    = inv["customer"].as_str().unwrap_or("—");
            let status      = inv["status"].as_str().unwrap_or("?");
            let amount_due  = inv["amount_due"].as_i64().unwrap_or(0);
            let subtotal    = inv["subtotal"].as_i64().unwrap_or(0);
            let tax         = inv["tax"].as_i64().unwrap_or(0);
            let total       = inv["total"].as_i64().unwrap_or(0);
            let currency    = inv["currency"].as_str().unwrap_or("usd");
            let due_date    = inv["due_date"].as_i64().map(fmt_ts).unwrap_or_else(|| "—".to_string());
            let hosted_url  = inv["hosted_invoice_url"].as_str().unwrap_or("—");

            let lines = inv["lines"]["data"].as_array().cloned().unwrap_or_default();
            let lines_str = lines.iter().map(|line| {
                let desc   = line["description"].as_str().unwrap_or("—");
                let amount = line["amount"].as_i64().unwrap_or(0);
                format!("  • {} — {}", desc, fmt_amount(amount, currency))
            }).collect::<Vec<_>>().join("\n");

            Ok(format!(
                "STRIPE 💳 — Facture {invoice_id}\n  client={customer} · statut={status}\n  sous-total={} · taxe={} · total={}\n  montant_dû={}\n  échéance={due_date}\n  url={hosted_url}\n\nLignes :\n{}",
                fmt_amount(subtotal, currency),
                fmt_amount(tax, currency),
                fmt_amount(total, currency),
                fmt_amount(amount_due, currency),
                if lines_str.is_empty() { "  (aucune)".to_string() } else { lines_str }
            ))
        }

        // ── 13. stripe_list_events ────────────────────────────────────────────
        "stripe_list_events" => {
            let limit       = args["limit"].as_u64().unwrap_or(20).min(100);
            let event_type  = args["event_type"].as_str().unwrap_or("").to_string();
            let since_hours = args["since_hours"].as_i64();

            let mut url = format!("{}?limit={}", cfg.api("events"), limit);
            if !event_type.is_empty() {
                url.push_str(&format!("&type={}", urlencoding_simple(&event_type)));
            }
            if let Some(hours) = since_hours {
                let since_ts = chrono::Utc::now().timestamp() - hours * 3600;
                url.push_str(&format!("&created[gte]={}", since_ts));
            }

            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun événement trouvé.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Événements ({}) :\n\n{}",
                items.len(),
                items.iter().map(|ev| {
                    let id          = ev["id"].as_str().unwrap_or("?");
                    let typ         = ev["type"].as_str().unwrap_or("?");
                    let created     = fmt_ts_val(&ev["created"]);
                    let api_version = ev["api_version"].as_str().unwrap_or("—");
                    let livemode    = ev["livemode"].as_bool().unwrap_or(false);
                    let mode        = if livemode { "live" } else { "test" };
                    format!("⚡ {typ}\n  id={id} · {created}\n  api_version={api_version} · mode={mode}")
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── 14. stripe_get_event ──────────────────────────────────────────────
        "stripe_get_event" => {
            let event_id = args["event_id"].as_str().ok_or("Missing param: event_id")?.to_string();
            let url      = cfg.api(&format!("events/{}", event_id));
            let ev       = get(&cfg, &url).await?;

            let typ         = ev["type"].as_str().unwrap_or("?");
            let created     = fmt_ts_val(&ev["created"]);
            let api_version = ev["api_version"].as_str().unwrap_or("—");
            let livemode    = ev["livemode"].as_bool().unwrap_or(false);
            let mode        = if livemode { "live" } else { "test" };
            // Summarize the data object (top-level keys of data.object)
            let data_obj    = &ev["data"]["object"];
            let summary     = if data_obj.is_object() {
                data_obj.as_object().map(|m| {
                    m.keys().take(8).cloned().collect::<Vec<_>>().join(", ")
                }).unwrap_or_default()
            } else {
                "—".to_string()
            };

            Ok(format!(
                "STRIPE 💳 — Événement {event_id}\n  type={typ}\n  créé={created}\n  api_version={api_version} · mode={mode}\n  données (champs) : {summary}"
            ))
        }

        // ── 15. stripe_list_webhooks ──────────────────────────────────────────
        "stripe_list_webhooks" => {
            let url   = cfg.api("webhook_endpoints");
            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun webhook configuré.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Webhooks ({}) :\n\n{}",
                items.len(),
                items.iter().map(|wh| {
                    let id         = wh["id"].as_str().unwrap_or("?");
                    let hook_url   = wh["url"].as_str().unwrap_or("—");
                    let status     = wh["status"].as_str().unwrap_or("?");
                    let status_ico = if status == "enabled" { "✅" } else { "❌" };
                    let created    = fmt_ts_val(&wh["created"]);
                    let ev_count   = wh["enabled_events"].as_array().map(|a| a.len()).unwrap_or(0);
                    format!("{status_ico} {hook_url}\n  id={id} · statut={status}\n  événements={ev_count} · créé={created}")
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── 16. stripe_get_webhook ────────────────────────────────────────────
        "stripe_get_webhook" => {
            let webhook_id = args["webhook_id"].as_str().ok_or("Missing param: webhook_id")?.to_string();
            let url        = cfg.api(&format!("webhook_endpoints/{}", webhook_id));
            let wh         = get(&cfg, &url).await?;

            let hook_url   = wh["url"].as_str().unwrap_or("—");
            let status     = wh["status"].as_str().unwrap_or("?");
            let created    = fmt_ts_val(&wh["created"]);
            let api_version= wh["api_version"].as_str().unwrap_or("—");
            let events     = wh["enabled_events"].as_array().cloned().unwrap_or_default();
            let events_str = events.iter()
                .map(|e| format!("  • {}", e.as_str().unwrap_or("?")))
                .collect::<Vec<_>>()
                .join("\n");

            Ok(format!(
                "STRIPE 💳 — Webhook {webhook_id}\n  url={hook_url}\n  statut={status}\n  api_version={api_version}\n  créé={created}\n\nÉvénements abonnés ({}) :\n{}",
                events.len(),
                if events_str.is_empty() { "  (aucun)".to_string() } else { events_str }
            ))
        }

        // ── 17. stripe_create_webhook ─────────────────────────────────────────
        "stripe_create_webhook" => {
            let hook_url = args["url"].as_str().ok_or("Missing param: url")?.to_string();
            let events_raw = args["events"].as_str().ok_or("Missing param: events")?.to_string();

            // Build form body manually: url=...&enabled_events[]=evt1&enabled_events[]=evt2
            let mut body = format!("url={}", urlencoding_simple(&hook_url));
            for event in events_raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                body.push_str(&format!("&enabled_events[]={}", urlencoding_simple(event)));
            }

            let url  = cfg.api("webhook_endpoints");
            let data = post_form_raw(&cfg, &url, body).await?;

            let id        = data["id"].as_str().unwrap_or("?");
            let status    = data["status"].as_str().unwrap_or("?");
            let ev_count  = data["enabled_events"].as_array().map(|a| a.len()).unwrap_or(0);
            Ok(format!(
                "STRIPE 💳 — ✅ Webhook créé.\n  id={id} · url={hook_url}\n  statut={status} · événements={ev_count}"
            ))
        }

        // ── 18. stripe_delete_webhook ─────────────────────────────────────────
        "stripe_delete_webhook" => {
            let webhook_id = args["webhook_id"].as_str().ok_or("Missing param: webhook_id")?.to_string();
            let url        = cfg.api(&format!("webhook_endpoints/{}", webhook_id));
            let data       = delete_req(&cfg, &url).await?;

            let deleted = data["deleted"].as_bool().unwrap_or(false);
            if deleted {
                Ok(format!("STRIPE 💳 — ✅ Webhook {webhook_id} supprimé."))
            } else {
                Ok(format!("STRIPE 💳 — ⚠️ Suppression demandée pour {webhook_id} (réponse: {:?}).", data))
            }
        }

        // ── 19. stripe_list_payouts ───────────────────────────────────────────
        "stripe_list_payouts" => {
            let limit    = args["limit"].as_u64().unwrap_or(10).min(100);
            let status_f = args["status"].as_str().unwrap_or("").to_string();

            let mut url = format!("{}?limit={}", cfg.api("payouts"), limit);
            if !status_f.is_empty() {
                url.push_str(&format!("&status={}", urlencoding_simple(&status_f)));
            }

            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun virement trouvé.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Virements ({}) :\n\n{}",
                items.len(),
                items.iter().map(|po| {
                    let id           = po["id"].as_str().unwrap_or("?");
                    let amount       = po["amount"].as_i64().unwrap_or(0);
                    let currency     = po["currency"].as_str().unwrap_or("usd");
                    let status       = po["status"].as_str().unwrap_or("?");
                    let arrival_date = po["arrival_date"].as_i64().map(fmt_ts).unwrap_or_else(|| "—".to_string());
                    let description  = po["description"].as_str().unwrap_or("—");
                    format!("🏦 {} · {}\n  id={id} · statut={status}\n  arrivée={arrival_date}\n  description={description}",
                        fmt_amount(amount, currency), currency.to_uppercase())
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── 20. stripe_get_payout ─────────────────────────────────────────────
        "stripe_get_payout" => {
            let payout_id = args["payout_id"].as_str().ok_or("Missing param: payout_id")?.to_string();
            let url       = cfg.api(&format!("payouts/{}", payout_id));
            let po        = get(&cfg, &url).await?;

            let amount       = po["amount"].as_i64().unwrap_or(0);
            let currency     = po["currency"].as_str().unwrap_or("usd");
            let status       = po["status"].as_str().unwrap_or("?");
            let arrival_date = po["arrival_date"].as_i64().map(fmt_ts).unwrap_or_else(|| "—".to_string());
            let description  = po["description"].as_str().unwrap_or("—");
            let method       = po["method"].as_str().unwrap_or("—");
            let source_type  = po["source_type"].as_str().unwrap_or("—");
            let created      = fmt_ts_val(&po["created"]);

            Ok(format!(
                "STRIPE 💳 — Virement {payout_id}\n  montant={}\n  statut={status}\n  arrivée={arrival_date}\n  description={description}\n  méthode={method}\n  type_source={source_type}\n  créé={created}",
                fmt_amount(amount, currency)
            ))
        }

        // ── stripe_search_customers ───────────────────────────────────────────
        "stripe_search_customers" => {
            let query = args["query"].as_str().ok_or("Missing param: query")?.to_string();
            let limit = args["limit"].as_u64().unwrap_or(10).min(100);

            let url = format!("{}?query={}&limit={}", cfg.api("customers/search"), urlencoding_simple(&query), limit);
            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun client trouvé.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Clients trouvés ({}) :\n\n{}",
                items.len(),
                items.iter().map(|c| {
                    let id      = c["id"].as_str().unwrap_or("?");
                    let name    = c["name"].as_str().unwrap_or("—");
                    let mail    = c["email"].as_str().unwrap_or("—");
                    let phone   = c["phone"].as_str().unwrap_or("—");
                    let created = fmt_ts_val(&c["created"]);
                    format!("👤 {name}\n  id={id} · email={mail}\n  tél={phone} · créé={created}")
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── stripe_update_customer ────────────────────────────────────────────
        "stripe_update_customer" => {
            let customer_id = args["customer_id"].as_str().ok_or("Missing param: customer_id")?.to_string();
            let email       = args["email"].as_str().unwrap_or("").to_string();
            let name        = args["name"].as_str().unwrap_or("").to_string();
            let phone       = args["phone"].as_str().unwrap_or("").to_string();
            let description = args["description"].as_str().unwrap_or("").to_string();

            let mut params: Vec<(&str, &str)> = vec![];
            let email_s = email.clone();
            let name_s  = name.clone();
            let phone_s = phone.clone();
            let desc_s  = description.clone();

            if !email_s.is_empty()  { params.push(("email", &email_s)); }
            if !name_s.is_empty()   { params.push(("name", &name_s)); }
            if !phone_s.is_empty()  { params.push(("phone", &phone_s)); }
            if !desc_s.is_empty()   { params.push(("description", &desc_s)); }

            let url  = cfg.api(&format!("customers/{}", customer_id));
            let data = post_form(&cfg, &url, &params).await?;

            let id = data["id"].as_str().unwrap_or("?");
            Ok(format!("STRIPE 💳 — ✅ Client mis à jour.\n  id={id}"))
        }

        // ── stripe_list_products ──────────────────────────────────────────────
        "stripe_list_products" => {
            let limit  = args["limit"].as_u64().unwrap_or(20).min(100);
            let active = args["active"].as_bool().unwrap_or(true);

            let url   = format!("{}?limit={}&active={}", cfg.api("products"), limit, active);
            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun produit trouvé.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Produits ({}) :\n\n{}",
                items.len(),
                items.iter().map(|p| {
                    let id      = p["id"].as_str().unwrap_or("?");
                    let name    = p["name"].as_str().unwrap_or("—");
                    let desc    = p["description"].as_str().unwrap_or("—");
                    let active  = p["active"].as_bool().unwrap_or(false);
                    let created = fmt_ts_val(&p["created"]);
                    let status  = if active { "actif" } else { "inactif" };
                    format!("📦 {name}\n  id={id} · statut={status}\n  description={desc}\n  créé={created}")
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── stripe_create_product ─────────────────────────────────────────────
        "stripe_create_product" => {
            let name        = args["name"].as_str().ok_or("Missing param: name")?.to_string();
            let description = args["description"].as_str().unwrap_or("").to_string();

            let mut params: Vec<(&str, &str)> = vec![];
            let name_s = name.clone();
            let desc_s = description.clone();
            params.push(("name", &name_s));
            if !desc_s.is_empty() { params.push(("description", &desc_s)); }

            let url  = cfg.api("products");
            let data = post_form(&cfg, &url, &params).await?;

            let id   = data["id"].as_str().unwrap_or("?");
            let pname = data["name"].as_str().unwrap_or("?");
            Ok(format!("STRIPE 💳 — ✅ Produit créé.\n  id={id} · nom={pname}"))
        }

        // ── stripe_list_prices ────────────────────────────────────────────────
        "stripe_list_prices" => {
            let limit      = args["limit"].as_u64().unwrap_or(20).min(100);
            let product_id = args["product_id"].as_str().unwrap_or("").to_string();

            let mut url = format!("{}?limit={}", cfg.api("prices"), limit);
            if !product_id.is_empty() {
                url.push_str(&format!("&product={}", urlencoding_simple(&product_id)));
            }

            let data  = get(&cfg, &url).await?;
            let items = data["data"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("STRIPE 💳 — Aucun prix trouvé.".to_string()); }

            Ok(format!(
                "STRIPE 💳 — Prix ({}) :\n\n{}",
                items.len(),
                items.iter().map(|p| {
                    let id       = p["id"].as_str().unwrap_or("?");
                    let product  = p["product"].as_str().unwrap_or("—");
                    let amount   = p["unit_amount"].as_i64().unwrap_or(0);
                    let currency = p["currency"].as_str().unwrap_or("usd");
                    let typ      = p["type"].as_str().unwrap_or("?");
                    let interval = p["recurring"]["interval"].as_str().unwrap_or("—");
                    let interval_str = if typ == "recurring" { format!(" / {interval}") } else { String::new() };
                    format!("💰 {}\n  id={id} · produit={product}\n  type={typ}{interval_str}",
                        fmt_amount(amount, currency))
                }).collect::<Vec<_>>().join("\n\n")
            ))
        }

        // ── stripe_create_price ───────────────────────────────────────────────
        "stripe_create_price" => {
            let product_id = args["product_id"].as_str().ok_or("Missing param: product_id")?.to_string();
            let amount     = args["amount"].as_i64().ok_or("Missing param: amount")?;
            let currency   = args["currency"].as_str().ok_or("Missing param: currency")?.to_string();
            let interval   = args["interval"].as_str().unwrap_or("").to_string();

            let amount_s   = amount.to_string();
            let mut params: Vec<(&str, &str)> = vec![
                ("product",    &product_id),
                ("unit_amount",&amount_s),
                ("currency",   &currency),
            ];

            let interval_s = interval.clone();
            if !interval_s.is_empty() {
                params.push(("recurring[interval]", &interval_s));
            } else {
                params.push(("type", "one_time"));
            }

            let url  = cfg.api("prices");
            let data = post_form(&cfg, &url, &params).await?;

            let id   = data["id"].as_str().unwrap_or("?");
            let typ  = data["type"].as_str().unwrap_or("?");
            Ok(format!("STRIPE 💳 — ✅ Prix créé.\n  id={id} · montant={} · type={typ}",
                fmt_amount(amount, &currency)))
        }

        // ── stripe_create_subscription ────────────────────────────────────────
        "stripe_create_subscription" => {
            let customer_id = args["customer_id"].as_str().ok_or("Missing param: customer_id")?.to_string();
            let price_id    = args["price_id"].as_str().ok_or("Missing param: price_id")?.to_string();
            let trial_days  = args["trial_days"].as_i64();

            let mut params: Vec<(&str, &str)> = vec![
                ("customer",         &customer_id),
                ("items[0][price]",  &price_id),
            ];

            let trial_s = trial_days.map(|d| d.to_string());
            if let Some(ref t) = trial_s { params.push(("trial_period_days", t.as_str())); }

            let url  = cfg.api("subscriptions");
            let data = post_form(&cfg, &url, &params).await?;

            let id         = data["id"].as_str().unwrap_or("?");
            let status     = data["status"].as_str().unwrap_or("?");
            let period_end = fmt_ts_val(&data["current_period_end"]);
            Ok(format!("STRIPE 💳 — ✅ Abonnement créé.\n  id={id} · statut={status}\n  fin_période={period_end}"))
        }

        // ── stripe_create_payment_link ────────────────────────────────────────
        "stripe_create_payment_link" => {
            let price_id = args["price_id"].as_str().ok_or("Missing param: price_id")?.to_string();
            let quantity  = args["quantity"].as_u64().unwrap_or(1).max(1);
            let qty_s     = quantity.to_string();

            let params: Vec<(&str, &str)> = vec![
                ("line_items[0][price]",    &price_id),
                ("line_items[0][quantity]", &qty_s),
            ];

            let url  = cfg.api("payment_links");
            let data = post_form(&cfg, &url, &params).await?;

            let id   = data["id"].as_str().unwrap_or("?");
            let link = data["url"].as_str().unwrap_or("?");
            Ok(format!("STRIPE 💳 — ✅ Lien de paiement créé.\n  id={id}\n  url={link}"))
        }

        // ── stripe_create_checkout_session ────────────────────────────────────
        "stripe_create_checkout_session" => {
            let price_id    = args["price_id"].as_str().ok_or("Missing param: price_id")?.to_string();
            let success_url = args["success_url"].as_str().ok_or("Missing param: success_url")?.to_string();
            let cancel_url  = args["cancel_url"].as_str().unwrap_or(&success_url).to_string();
            let mode        = args["mode"].as_str().unwrap_or("payment").to_string();
            let quantity    = args["quantity"].as_u64().unwrap_or(1).max(1);
            let qty_s       = quantity.to_string();

            let params: Vec<(&str, &str)> = vec![
                ("line_items[0][price]",    &price_id),
                ("line_items[0][quantity]", &qty_s),
                ("success_url",             &success_url),
                ("cancel_url",              &cancel_url),
                ("mode",                    &mode),
            ];

            let url  = cfg.api("checkout/sessions");
            let data = post_form(&cfg, &url, &params).await?;

            let id          = data["id"].as_str().unwrap_or("?");
            let session_url = data["url"].as_str().unwrap_or("?");
            Ok(format!("STRIPE 💳 — ✅ Session Checkout créée.\n  id={id}\n  url={session_url}"))
        }

        other => Err(format!("Unknown stripe tool: {other}")),
    }
}
