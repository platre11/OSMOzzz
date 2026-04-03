/// Connecteur Shopify — REST Admin API 2024-01 (Private App Access Token).
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ShopifyConfig {
    shop_domain: String,
    access_token: String,
}

impl ShopifyConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/shopify.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!(
            "https://{}/admin/api/2024-01/{}",
            self.shop_domain,
            path.trim_start_matches('/')
        )
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &ShopifyConfig, url: &str) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .get(url)
        .header("X-Shopify-Access-Token", &cfg.access_token)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(errors) = resp.get("errors") {
        return Err(format!("Shopify API error: {}", errors));
    }
    Ok(resp)
}

async fn post_json(cfg: &ShopifyConfig, url: &str, body: &Value) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .post(url)
        .header("X-Shopify-Access-Token", &cfg.access_token)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(errors) = resp.get("errors") {
        return Err(format!("Shopify API error: {}", errors));
    }
    Ok(resp)
}

async fn put_json(cfg: &ShopifyConfig, url: &str, body: &Value) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .put(url)
        .header("X-Shopify-Access-Token", &cfg.access_token)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(errors) = resp.get("errors") {
        return Err(format!("Shopify API error: {}", errors));
    }
    Ok(resp)
}

async fn delete_req(cfg: &ShopifyConfig, url: &str) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .delete(url)
        .header("X-Shopify-Access-Token", &cfg.access_token)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if status.is_success() {
        // DELETE often returns 200 with empty body or minimal JSON
        let text = resp.text().await.unwrap_or_default();
        if text.trim().is_empty() || text.trim() == "{}" {
            return Ok(json!({}));
        }
        serde_json::from_str::<Value>(&text).map_err(|e| e.to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Shopify DELETE error {status}: {text}"))
    }
}

// ─── Formatters ───────────────────────────────────────────────────────────────

fn format_order(o: &Value) -> String {
    let id                = o["id"].as_u64().map(|v| v.to_string()).unwrap_or_else(|| "—".to_string());
    let order_number      = o["order_number"].as_u64().map(|v| format!("#{v}")).unwrap_or_else(|| "—".to_string());
    let email             = o["email"].as_str().unwrap_or("—");
    let financial_status  = o["financial_status"].as_str().unwrap_or("—");
    let fulfillment_status = o["fulfillment_status"].as_str().unwrap_or("unfulfilled");
    let total_price       = o["total_price"].as_str().unwrap_or("—");
    let currency          = o["currency"].as_str().unwrap_or("");
    let created_at        = o["created_at"].as_str().unwrap_or("—");
    let line_items_count  = o["line_items"].as_array().map(|a| a.len()).unwrap_or(0);
    format!(
        "• [{id}] {order_number} | {email}\n  Paiement : {financial_status} | Livraison : {fulfillment_status} | Total : {total_price} {currency}\n  {line_items_count} article(s) | Créé : {created_at}"
    )
}

fn format_product(p: &Value) -> String {
    let id            = p["id"].as_u64().map(|v| v.to_string()).unwrap_or_else(|| "—".to_string());
    let title         = p["title"].as_str().unwrap_or("—");
    let status        = p["status"].as_str().unwrap_or("—");
    let vendor        = p["vendor"].as_str().unwrap_or("—");
    let variants      = p["variants"].as_array().map(|a| a.len()).unwrap_or(0);
    // Price range from variants
    let prices: Vec<f64> = p["variants"].as_array().unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v["price"].as_str().and_then(|s| s.parse::<f64>().ok()))
        .collect();
    let price_range = if prices.is_empty() {
        "—".to_string()
    } else if prices.len() == 1 || prices.iter().cloned().fold(f64::INFINITY, f64::min) == prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max) {
        format!("{:.2}", prices[0])
    } else {
        let min = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        format!("{min:.2}–{max:.2}")
    };
    format!(
        "• [{id}] {title}\n  Statut : {status} | Vendeur : {vendor} | {variants} variante(s) | Prix : {price_range}"
    )
}

fn format_variant(v: &Value) -> String {
    let id       = v["id"].as_u64().map(|x| x.to_string()).unwrap_or_else(|| "—".to_string());
    let title    = v["title"].as_str().unwrap_or("Default");
    let price    = v["price"].as_str().unwrap_or("—");
    let sku      = v["sku"].as_str().unwrap_or("—");
    let inv      = v["inventory_quantity"].as_i64().unwrap_or(0);
    format!("• [{id}] {title} | Prix : {price} | SKU : {sku} | Stock : {inv}")
}

fn format_draft_order(d: &Value) -> String {
    let id          = d["id"].as_u64().map(|x| x.to_string()).unwrap_or_else(|| "—".to_string());
    let name        = d["name"].as_str().unwrap_or("—");
    let status      = d["status"].as_str().unwrap_or("—");
    let total_price = d["total_price"].as_str().unwrap_or("—");
    let customer_email = d["customer"]["email"].as_str().unwrap_or("—");
    format!("• [{id}] {name} | Statut : {status} | Total : {total_price} | Client : {customer_email}")
}

fn format_customer(c: &Value) -> String {
    let id            = c["id"].as_u64().map(|v| v.to_string()).unwrap_or_else(|| "—".to_string());
    let first_name    = c["first_name"].as_str().unwrap_or("");
    let last_name     = c["last_name"].as_str().unwrap_or("");
    let email         = c["email"].as_str().unwrap_or("—");
    let orders_count  = c["orders_count"].as_u64().unwrap_or(0);
    let total_spent   = c["total_spent"].as_str().unwrap_or("0.00");
    format!(
        "• [{id}] {first_name} {last_name} | {email}\n  Commandes : {orders_count} | Total dépensé : {total_spent}"
    )
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "shopify_get_shop",
            "description": "SHOPIFY 🛍️ — Récupère les informations de la boutique : nom, email, domaine, plan Shopify, devise principale, fuseau horaire et adresse.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "shopify_list_orders",
            "description": "SHOPIFY 🛍️ — Liste les commandes de la boutique avec id, numéro, email client, statut paiement/livraison, total et date. Utiliser shopify_get_order pour le détail complet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["open", "closed", "cancelled", "any"],
                        "default": "any",
                        "description": "Filtrer par statut : open (en cours), closed (terminées), cancelled (annulées), any (toutes). Défaut : any."
                    },
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 250,
                        "description": "Nombre de commandes à retourner (défaut: 50, max: 250)"
                    }
                }
            }
        }),
        json!({
            "name": "shopify_get_order",
            "description": "SHOPIFY 🛍️ — Récupère le détail complet d'une commande : articles, adresses livraison/facturation, paiement, remises, notes et historique de livraison.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "order_id": { "type": "string", "description": "ID numérique de la commande" }
                },
                "required": ["order_id"]
            }
        }),
        json!({
            "name": "shopify_cancel_order",
            "description": "SHOPIFY 🛍️ — Annule une commande avec un motif optionnel. La commande doit être dans un état annulable (non déjà expédiée ou remboursée).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "order_id": { "type": "string", "description": "ID numérique de la commande à annuler" },
                    "reason": {
                        "type": "string",
                        "enum": ["customer", "fraud", "inventory", "declined", "other"],
                        "description": "Motif d'annulation (optionnel) : customer, fraud, inventory, declined, other"
                    }
                },
                "required": ["order_id"]
            }
        }),
        json!({
            "name": "shopify_fulfill_order",
            "description": "SHOPIFY 🛍️ — Marque une commande comme expédiée/livrée en créant un fulfillment. Optionnellement ajouter un numéro de suivi et le transporteur.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "order_id": { "type": "string", "description": "ID numérique de la commande à fulfiller" },
                    "tracking_number": { "type": "string", "description": "Numéro de suivi (optionnel)" },
                    "tracking_company": { "type": "string", "description": "Transporteur (ex: UPS, DHL, FedEx) — optionnel" },
                    "notify_customer": {
                        "type": "boolean",
                        "default": true,
                        "description": "Envoyer un email de notification au client (défaut: true)"
                    }
                },
                "required": ["order_id"]
            }
        }),
        json!({
            "name": "shopify_list_products",
            "description": "SHOPIFY 🛍️ — Liste les produits de la boutique avec id, titre, statut, vendeur, nombre de variantes et gamme de prix. Utiliser shopify_get_product pour le détail.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 250,
                        "description": "Nombre de produits à retourner (défaut: 50)"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["active", "archived", "draft"],
                        "description": "Filtrer par statut (optionnel) : active, archived, draft"
                    }
                }
            }
        }),
        json!({
            "name": "shopify_get_product",
            "description": "SHOPIFY 🛍️ — Récupère le détail complet d'un produit : variantes, images, tags, options, prix, SKUs et inventaire.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "product_id": { "type": "string", "description": "ID numérique du produit" }
                },
                "required": ["product_id"]
            }
        }),
        json!({
            "name": "shopify_create_product",
            "description": "SHOPIFY 🛍️ — Crée un nouveau produit dans la boutique avec titre, description HTML, vendeur, type et statut.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Titre du produit" },
                    "body_html": { "type": "string", "description": "Description HTML du produit (optionnel)" },
                    "vendor": { "type": "string", "description": "Nom du vendeur/marque (optionnel)" },
                    "product_type": { "type": "string", "description": "Type de produit (optionnel)" },
                    "status": {
                        "type": "string",
                        "enum": ["active", "draft", "archived"],
                        "default": "draft",
                        "description": "Statut initial (défaut: draft)"
                    },
                    "tags": { "type": "string", "description": "Tags séparés par des virgules (optionnel)" }
                },
                "required": ["title"]
            }
        }),
        json!({
            "name": "shopify_update_product",
            "description": "SHOPIFY 🛍️ — Met à jour un produit existant (titre, description, vendeur, type, statut, tags). Seuls les champs fournis sont modifiés.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "product_id": { "type": "string", "description": "ID numérique du produit à modifier" },
                    "title": { "type": "string", "description": "Nouveau titre (optionnel)" },
                    "body_html": { "type": "string", "description": "Nouvelle description HTML (optionnel)" },
                    "vendor": { "type": "string", "description": "Nouveau vendeur (optionnel)" },
                    "product_type": { "type": "string", "description": "Nouveau type (optionnel)" },
                    "status": {
                        "type": "string",
                        "enum": ["active", "draft", "archived"],
                        "description": "Nouveau statut (optionnel)"
                    },
                    "tags": { "type": "string", "description": "Nouveaux tags (optionnel)" }
                },
                "required": ["product_id"]
            }
        }),
        json!({
            "name": "shopify_delete_product",
            "description": "SHOPIFY 🛍️ — Supprime définitivement un produit et toutes ses variantes. Action irréversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "product_id": { "type": "string", "description": "ID numérique du produit à supprimer" }
                },
                "required": ["product_id"]
            }
        }),
        json!({
            "name": "shopify_list_customers",
            "description": "SHOPIFY 🛍️ — Liste les clients de la boutique avec id, nom, email, nombre de commandes et total dépensé. Utiliser shopify_search_customers pour chercher par nom/email.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 250,
                        "description": "Nombre de clients à retourner (défaut: 50)"
                    }
                }
            }
        }),
        json!({
            "name": "shopify_get_customer",
            "description": "SHOPIFY 🛍️ — Récupère le profil complet d'un client : coordonnées, adresses, historique commandes, tags et notes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "customer_id": { "type": "string", "description": "ID numérique du client" }
                },
                "required": ["customer_id"]
            }
        }),
        json!({
            "name": "shopify_search_customers",
            "description": "SHOPIFY 🛍️ — Recherche des clients par nom, email ou téléphone. Retourne id, nom, email, commandes et total dépensé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Terme de recherche (nom, email, téléphone)" },
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 250,
                        "description": "Nombre de résultats (défaut: 50)"
                    }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "shopify_create_customer",
            "description": "SHOPIFY 🛍️ — Crée un nouveau client dans la boutique avec prénom, nom, email et téléphone.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "first_name": { "type": "string", "description": "Prénom du client" },
                    "last_name": { "type": "string", "description": "Nom du client" },
                    "email": { "type": "string", "description": "Adresse email" },
                    "phone": { "type": "string", "description": "Numéro de téléphone (optionnel, format E.164 ex: +33612345678)" },
                    "tags": { "type": "string", "description": "Tags séparés par virgule (optionnel)" },
                    "note": { "type": "string", "description": "Note interne (optionnel)" }
                },
                "required": ["email"]
            }
        }),
        json!({
            "name": "shopify_update_customer",
            "description": "SHOPIFY 🛍️ — Met à jour un client existant (prénom, nom, email, téléphone, tags, note). Seuls les champs fournis sont modifiés.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "customer_id": { "type": "string", "description": "ID numérique du client à modifier" },
                    "first_name": { "type": "string", "description": "Nouveau prénom (optionnel)" },
                    "last_name": { "type": "string", "description": "Nouveau nom (optionnel)" },
                    "email": { "type": "string", "description": "Nouvel email (optionnel)" },
                    "phone": { "type": "string", "description": "Nouveau téléphone (optionnel)" },
                    "tags": { "type": "string", "description": "Nouveaux tags (optionnel)" },
                    "note": { "type": "string", "description": "Nouvelle note (optionnel)" }
                },
                "required": ["customer_id"]
            }
        }),
        json!({
            "name": "shopify_list_collections",
            "description": "SHOPIFY 🛍️ — Liste les collections personnalisées (custom collections) de la boutique avec id, titre, handle et nombre de produits.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 250,
                        "description": "Nombre de collections (défaut: 50)"
                    }
                }
            }
        }),
        json!({
            "name": "shopify_get_inventory_levels",
            "description": "SHOPIFY 🛍️ — Récupère les niveaux d'inventaire pour tous les emplacements (locations) de la boutique. Retourne les quantités disponibles par location et variant.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 250,
                        "description": "Nombre d'entrées d'inventaire (défaut: 50)"
                    }
                }
            }
        }),
        json!({
            "name": "shopify_list_webhooks",
            "description": "SHOPIFY 🛍️ — Liste tous les webhooks configurés sur la boutique : topic, URL de destination, format et date de création.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "shopify_create_webhook",
            "description": "SHOPIFY 🛍️ — Crée un nouveau webhook pour recevoir des notifications d'événements Shopify (ex: orders/create, products/update) vers une URL externe.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Événement à écouter (ex: orders/create, orders/updated, products/create, customers/create, app/uninstalled)"
                    },
                    "address": {
                        "type": "string",
                        "description": "URL HTTPS de destination qui recevra les notifications"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["json", "xml"],
                        "default": "json",
                        "description": "Format des données envoyées (défaut: json)"
                    }
                },
                "required": ["topic", "address"]
            }
        }),
        json!({
            "name": "shopify_list_price_rules",
            "description": "SHOPIFY 🛍️ — Liste les règles de prix (remises) configurées : titre, type de remise (pourcentage/montant fixe), valeur, conditions d'utilisation et dates de validité.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 250,
                        "description": "Nombre de règles de prix (défaut: 50)"
                    }
                }
            }
        }),
        json!({
            "name": "shopify_list_locations",
            "description": "SHOPIFY 🛍️ — Liste les emplacements de fulfillment (locations) de la boutique : id, nom et adresse. Nécessaire pour la gestion d'inventaire (shopify_update_inventory).",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "shopify_update_inventory",
            "description": "SHOPIFY 🛍️ — Définit la quantité disponible d'un article d'inventaire dans un emplacement donné. Utiliser shopify_list_locations pour obtenir location_id et shopify_get_product pour inventory_item_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "location_id": { "type": "integer", "description": "ID numérique de l'emplacement (location)" },
                    "inventory_item_id": { "type": "integer", "description": "ID de l'article d'inventaire (inventory_item_id de la variante)" },
                    "available": { "type": "integer", "description": "Quantité disponible à définir" }
                },
                "required": ["location_id", "inventory_item_id", "available"]
            }
        }),
        json!({
            "name": "shopify_list_product_variants",
            "description": "SHOPIFY 🛍️ — Liste toutes les variantes d'un produit : id, titre, prix, SKU et quantité en stock.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "product_id": { "type": "string", "description": "ID numérique du produit" }
                },
                "required": ["product_id"]
            }
        }),
        json!({
            "name": "shopify_update_product_variant",
            "description": "SHOPIFY 🛍️ — Met à jour une variante de produit (prix, SKU, prix barré). Seuls les champs fournis sont modifiés.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "variant_id": { "type": "string", "description": "ID numérique de la variante" },
                    "price": { "type": "string", "description": "Nouveau prix (ex: \"29.99\") — optionnel" },
                    "sku": { "type": "string", "description": "Nouveau SKU — optionnel" },
                    "compare_at_price": { "type": "string", "description": "Prix barré (prix de référence) — optionnel" }
                },
                "required": ["variant_id"]
            }
        }),
        json!({
            "name": "shopify_refund_order",
            "description": "SHOPIFY 🛍️ — Crée un remboursement sur une commande avec le montant et une note optionnelle. Remboursement via la passerelle de paiement originale.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "order_id": { "type": "string", "description": "ID numérique de la commande à rembourser" },
                    "amount": { "type": "string", "description": "Montant à rembourser (ex: \"15.00\")" },
                    "note": { "type": "string", "description": "Note interne expliquant le remboursement (optionnel)" }
                },
                "required": ["order_id", "amount"]
            }
        }),
        json!({
            "name": "shopify_list_draft_orders",
            "description": "SHOPIFY 🛍️ — Liste les commandes brouillon (draft orders) : id, nom, statut, total et client. Utile pour les commandes créées manuellement ou en attente de paiement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["open", "invoice_sent", "completed"],
                        "default": "open",
                        "description": "Filtrer par statut : open (ouvertes), invoice_sent (facture envoyée), completed (complétées). Défaut : open."
                    }
                }
            }
        }),
        json!({
            "name": "shopify_create_draft_order",
            "description": "SHOPIFY 🛍️ — Crée une commande brouillon avec des articles (variant_id + quantité), un client optionnel et une note.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "line_items": {
                        "type": "string",
                        "description": "Tableau JSON d'articles, ex: [{\"variant_id\": 123, \"quantity\": 2}]"
                    },
                    "customer_id": { "type": "string", "description": "ID du client à associer (optionnel)" },
                    "note": { "type": "string", "description": "Note interne (optionnel)" }
                },
                "required": ["line_items"]
            }
        }),
        json!({
            "name": "shopify_complete_draft_order",
            "description": "SHOPIFY 🛍️ — Marque une commande brouillon comme complète/payée, la convertissant en commande réelle.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID numérique de la commande brouillon" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "shopify_list_transactions",
            "description": "SHOPIFY 🛍️ — Liste les transactions de paiement d'une commande : id, type (authorization/capture/refund/sale), statut, montant et passerelle de paiement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "order_id": { "type": "string", "description": "ID numérique de la commande" }
                },
                "required": ["order_id"]
            }
        }),
        json!({
            "name": "shopify_create_collection",
            "description": "SHOPIFY 🛍️ — Crée une collection personnalisée (custom collection) avec un titre et une description HTML optionnelle.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Titre de la collection" },
                    "body_html": { "type": "string", "description": "Description HTML de la collection (optionnel)" }
                },
                "required": ["title"]
            }
        }),
        json!({
            "name": "shopify_list_smart_collections",
            "description": "SHOPIFY 🛍️ — Liste les collections automatiques (smart collections) qui regroupent les produits selon des règles définies : id, titre, handle, règles et nombre de produits.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "minimum": 1,
                        "maximum": 250,
                        "description": "Nombre de collections (défaut: 50)"
                    }
                }
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = ShopifyConfig::load()
        .ok_or_else(|| "Shopify non configuré — créer ~/.osmozzz/shopify.toml avec shop_domain et access_token".to_string())?;

    match name {
        "shopify_get_shop" => {
            let url  = cfg.api("/shop.json");
            let resp = get(&cfg, &url).await?;
            let s    = &resp["shop"];

            let name_       = s["name"].as_str().unwrap_or("—");
            let email       = s["email"].as_str().unwrap_or("—");
            let domain      = s["domain"].as_str().unwrap_or("—");
            let myshopify   = s["myshopify_domain"].as_str().unwrap_or("—");
            let plan        = s["plan_display_name"].as_str().unwrap_or("—");
            let currency    = s["currency"].as_str().unwrap_or("—");
            let timezone    = s["iana_timezone"].as_str().unwrap_or("—");
            let country     = s["country_name"].as_str().unwrap_or("—");
            let created     = s["created_at"].as_str().unwrap_or("—");

            Ok(format!(
                "🛍️ {name_}\nEmail       : {email}\nDomaine     : {domain}\nMyShopify   : {myshopify}\nPlan        : {plan}\nDevise      : {currency}\nTimezone    : {timezone}\nPays        : {country}\nCréé le     : {created}"
            ))
        }

        "shopify_list_orders" => {
            let status = args["status"].as_str().unwrap_or("any");
            let limit  = args["limit"].as_u64().unwrap_or(50);
            let url    = cfg.api(&format!("/orders.json?limit={limit}&status={status}"));
            let resp   = get(&cfg, &url).await?;
            let orders = resp["orders"].as_array().cloned().unwrap_or_default();

            if orders.is_empty() {
                return Ok(format!("Aucune commande (statut: {status})."));
            }

            let mut out = format!("{} commande(s) [statut: {status}] :\n\n", orders.len());
            for o in &orders {
                out.push_str(&format_order(o));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_get_order" => {
            let order_id = args["order_id"].as_str().ok_or("Paramètre 'order_id' requis")?;
            let url      = cfg.api(&format!("/orders/{order_id}.json"));
            let resp     = get(&cfg, &url).await?;
            let o        = &resp["order"];

            let order_number      = o["order_number"].as_u64().map(|v| format!("#{v}")).unwrap_or_else(|| "—".to_string());
            let email             = o["email"].as_str().unwrap_or("—");
            let financial_status  = o["financial_status"].as_str().unwrap_or("—");
            let fulfillment_status = o["fulfillment_status"].as_str().unwrap_or("unfulfilled");
            let total_price       = o["total_price"].as_str().unwrap_or("—");
            let subtotal_price    = o["subtotal_price"].as_str().unwrap_or("—");
            let total_tax         = o["total_tax"].as_str().unwrap_or("0.00");
            let currency          = o["currency"].as_str().unwrap_or("");
            let created_at        = o["created_at"].as_str().unwrap_or("—");
            let note              = o["note"].as_str().unwrap_or("");

            let shipping_addr = &o["shipping_address"];
            let ship_str = if shipping_addr.is_object() {
                format!(
                    "{} {}, {}, {} {}",
                    shipping_addr["first_name"].as_str().unwrap_or(""),
                    shipping_addr["last_name"].as_str().unwrap_or(""),
                    shipping_addr["address1"].as_str().unwrap_or("—"),
                    shipping_addr["city"].as_str().unwrap_or("—"),
                    shipping_addr["country"].as_str().unwrap_or("—"),
                )
            } else {
                "—".to_string()
            };

            let mut out = format!(
                "📦 Commande {order_number} (ID: {order_id})\nEmail           : {email}\nPaiement        : {financial_status}\nLivraison       : {fulfillment_status}\nSous-total      : {subtotal_price} {currency}\nTaxes           : {total_tax} {currency}\nTotal           : {total_price} {currency}\nAdresse         : {ship_str}\nDate            : {created_at}\n"
            );

            if !note.is_empty() {
                out.push_str(&format!("Note            : {note}\n"));
            }

            if let Some(items) = o["line_items"].as_array() {
                out.push_str(&format!("\n{} article(s) :\n", items.len()));
                for item in items {
                    let title    = item["title"].as_str().unwrap_or("—");
                    let quantity = item["quantity"].as_u64().unwrap_or(1);
                    let price    = item["price"].as_str().unwrap_or("—");
                    out.push_str(&format!("  • {title} × {quantity} — {price} {currency}\n"));
                }
            }

            Ok(out.trim_end().to_string())
        }

        "shopify_cancel_order" => {
            let order_id = args["order_id"].as_str().ok_or("Paramètre 'order_id' requis")?;
            let url      = cfg.api(&format!("/orders/{order_id}/cancel.json"));
            let mut body = json!({});
            if let Some(reason) = args["reason"].as_str() {
                body["reason"] = json!(reason);
            }
            let resp = post_json(&cfg, &url, &body).await?;
            let o    = &resp["order"];
            let status = o["cancel_reason"].as_str().unwrap_or("cancelled");
            Ok(format!("Commande {order_id} annulée. Motif : {status}."))
        }

        "shopify_fulfill_order" => {
            let order_id = args["order_id"].as_str().ok_or("Paramètre 'order_id' requis")?;
            let notify   = args["notify_customer"].as_bool().unwrap_or(true);

            let mut fulfillment = json!({
                "notify_customer": notify
            });

            if let Some(tn) = args["tracking_number"].as_str() {
                if !tn.is_empty() {
                    fulfillment["tracking_number"] = json!(tn);
                }
            }
            if let Some(tc) = args["tracking_company"].as_str() {
                if !tc.is_empty() {
                    fulfillment["tracking_company"] = json!(tc);
                }
            }

            let body = json!({ "fulfillment": fulfillment });
            let url  = cfg.api(&format!("/orders/{order_id}/fulfillments.json"));
            let resp = post_json(&cfg, &url, &body).await?;
            let f    = &resp["fulfillment"];
            let status = f["status"].as_str().unwrap_or("success");
            Ok(format!("Commande {order_id} fulfillée. Statut : {status}."))
        }

        "shopify_list_products" => {
            let limit  = args["limit"].as_u64().unwrap_or(50);
            let mut url = format!("{}&limit={limit}", cfg.api("/products.json?"));
            if let Some(status) = args["status"].as_str() {
                url.push_str(&format!("&status={status}"));
            }
            let resp     = get(&cfg, &url).await?;
            let products = resp["products"].as_array().cloned().unwrap_or_default();

            if products.is_empty() {
                return Ok("Aucun produit trouvé.".to_string());
            }

            let mut out = format!("{} produit(s) :\n\n", products.len());
            for p in &products {
                out.push_str(&format_product(p));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_get_product" => {
            let product_id = args["product_id"].as_str().ok_or("Paramètre 'product_id' requis")?;
            let url        = cfg.api(&format!("/products/{product_id}.json"));
            let resp       = get(&cfg, &url).await?;
            let p          = &resp["product"];

            let title        = p["title"].as_str().unwrap_or("—");
            let status       = p["status"].as_str().unwrap_or("—");
            let vendor       = p["vendor"].as_str().unwrap_or("—");
            let product_type = p["product_type"].as_str().unwrap_or("—");
            let tags         = p["tags"].as_str().unwrap_or("—");
            let created      = p["created_at"].as_str().unwrap_or("—");
            let updated      = p["updated_at"].as_str().unwrap_or("—");

            let mut out = format!(
                "🛍️ {title} (ID: {product_id})\nStatut     : {status}\nVendeur    : {vendor}\nType       : {product_type}\nTags       : {tags}\nCréé       : {created}\nMis à jour : {updated}\n"
            );

            if let Some(variants) = p["variants"].as_array() {
                out.push_str(&format!("\n{} variante(s) :\n", variants.len()));
                for v in variants {
                    let v_title = v["title"].as_str().unwrap_or("Default");
                    let price   = v["price"].as_str().unwrap_or("—");
                    let sku     = v["sku"].as_str().unwrap_or("—");
                    let inv     = v["inventory_quantity"].as_i64().unwrap_or(0);
                    out.push_str(&format!("  • {v_title} — {price} | SKU: {sku} | Stock: {inv}\n"));
                }
            }

            Ok(out.trim_end().to_string())
        }

        "shopify_create_product" => {
            let title = args["title"].as_str().ok_or("Paramètre 'title' requis")?;
            let mut product = json!({ "title": title });
            if let Some(v) = args["body_html"].as_str()    { product["body_html"] = json!(v); }
            if let Some(v) = args["vendor"].as_str()       { product["vendor"] = json!(v); }
            if let Some(v) = args["product_type"].as_str() { product["product_type"] = json!(v); }
            if let Some(v) = args["tags"].as_str()         { product["tags"] = json!(v); }
            product["status"] = json!(args["status"].as_str().unwrap_or("draft"));

            let body = json!({ "product": product });
            let url  = cfg.api("/products.json");
            let resp = post_json(&cfg, &url, &body).await?;
            let p    = &resp["product"];
            let id   = p["id"].as_u64().unwrap_or(0);
            Ok(format!("Produit '{title}' créé (ID: {id})."))
        }

        "shopify_update_product" => {
            let product_id = args["product_id"].as_str().ok_or("Paramètre 'product_id' requis")?;
            let mut product = json!({ "id": product_id });
            if let Some(v) = args["title"].as_str()        { product["title"] = json!(v); }
            if let Some(v) = args["body_html"].as_str()    { product["body_html"] = json!(v); }
            if let Some(v) = args["vendor"].as_str()       { product["vendor"] = json!(v); }
            if let Some(v) = args["product_type"].as_str() { product["product_type"] = json!(v); }
            if let Some(v) = args["status"].as_str()       { product["status"] = json!(v); }
            if let Some(v) = args["tags"].as_str()         { product["tags"] = json!(v); }

            let body = json!({ "product": product });
            let url  = cfg.api(&format!("/products/{product_id}.json"));
            let _    = put_json(&cfg, &url, &body).await?;
            Ok(format!("Produit {product_id} mis à jour."))
        }

        "shopify_delete_product" => {
            let product_id = args["product_id"].as_str().ok_or("Paramètre 'product_id' requis")?;
            let url        = cfg.api(&format!("/products/{product_id}.json"));
            delete_req(&cfg, &url).await?;
            Ok(format!("Produit {product_id} supprimé."))
        }

        "shopify_list_customers" => {
            let limit     = args["limit"].as_u64().unwrap_or(50);
            let url       = cfg.api(&format!("/customers.json?limit={limit}"));
            let resp      = get(&cfg, &url).await?;
            let customers = resp["customers"].as_array().cloned().unwrap_or_default();

            if customers.is_empty() {
                return Ok("Aucun client trouvé.".to_string());
            }

            let mut out = format!("{} client(s) :\n\n", customers.len());
            for c in &customers {
                out.push_str(&format_customer(c));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_get_customer" => {
            let customer_id = args["customer_id"].as_str().ok_or("Paramètre 'customer_id' requis")?;
            let url         = cfg.api(&format!("/customers/{customer_id}.json"));
            let resp        = get(&cfg, &url).await?;
            let c           = &resp["customer"];

            let first_name   = c["first_name"].as_str().unwrap_or("");
            let last_name    = c["last_name"].as_str().unwrap_or("");
            let email        = c["email"].as_str().unwrap_or("—");
            let phone        = c["phone"].as_str().unwrap_or("—");
            let orders_count = c["orders_count"].as_u64().unwrap_or(0);
            let total_spent  = c["total_spent"].as_str().unwrap_or("0.00");
            let tags         = c["tags"].as_str().unwrap_or("—");
            let note         = c["note"].as_str().unwrap_or("—");
            let created      = c["created_at"].as_str().unwrap_or("—");
            let verified     = c["verified_email"].as_bool().unwrap_or(false);

            let mut out = format!(
                "👤 {first_name} {last_name} (ID: {customer_id})\nEmail        : {email}\nTéléphone    : {phone}\nCommandes    : {orders_count}\nTotal dépensé: {total_spent}\nTags         : {tags}\nNote         : {note}\nEmail vérifié: {verified}\nCréé le      : {created}\n"
            );

            if let Some(addrs) = c["addresses"].as_array() {
                if !addrs.is_empty() {
                    out.push_str("\nAdresses :\n");
                    for addr in addrs {
                        let a1      = addr["address1"].as_str().unwrap_or("—");
                        let city    = addr["city"].as_str().unwrap_or("—");
                        let country = addr["country"].as_str().unwrap_or("—");
                        let default = addr["default"].as_bool().unwrap_or(false);
                        let def_str = if default { " [par défaut]" } else { "" };
                        out.push_str(&format!("  • {a1}, {city}, {country}{def_str}\n"));
                    }
                }
            }

            Ok(out.trim_end().to_string())
        }

        "shopify_search_customers" => {
            let query     = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let limit     = args["limit"].as_u64().unwrap_or(50);
            let url       = cfg.api(&format!("/customers/search.json?query={}&limit={limit}", urlencoding(query)));
            let resp      = get(&cfg, &url).await?;
            let customers = resp["customers"].as_array().cloned().unwrap_or_default();

            if customers.is_empty() {
                return Ok(format!("Aucun client trouvé pour « {query} »."));
            }

            let mut out = format!("{} client(s) pour « {query} » :\n\n", customers.len());
            for c in &customers {
                out.push_str(&format_customer(c));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_create_customer" => {
            let email = args["email"].as_str().ok_or("Paramètre 'email' requis")?;
            let mut customer = json!({ "email": email });
            if let Some(v) = args["first_name"].as_str() { customer["first_name"] = json!(v); }
            if let Some(v) = args["last_name"].as_str()  { customer["last_name"]  = json!(v); }
            if let Some(v) = args["phone"].as_str()      { customer["phone"]      = json!(v); }
            if let Some(v) = args["tags"].as_str()       { customer["tags"]       = json!(v); }
            if let Some(v) = args["note"].as_str()       { customer["note"]       = json!(v); }

            let body = json!({ "customer": customer });
            let url  = cfg.api("/customers.json");
            let resp = post_json(&cfg, &url, &body).await?;
            let c    = &resp["customer"];
            let id   = c["id"].as_u64().unwrap_or(0);
            Ok(format!("Client '{email}' créé (ID: {id})."))
        }

        "shopify_update_customer" => {
            let customer_id = args["customer_id"].as_str().ok_or("Paramètre 'customer_id' requis")?;
            let mut customer = json!({ "id": customer_id });
            if let Some(v) = args["first_name"].as_str() { customer["first_name"] = json!(v); }
            if let Some(v) = args["last_name"].as_str()  { customer["last_name"]  = json!(v); }
            if let Some(v) = args["email"].as_str()      { customer["email"]      = json!(v); }
            if let Some(v) = args["phone"].as_str()      { customer["phone"]      = json!(v); }
            if let Some(v) = args["tags"].as_str()       { customer["tags"]       = json!(v); }
            if let Some(v) = args["note"].as_str()       { customer["note"]       = json!(v); }

            let body = json!({ "customer": customer });
            let url  = cfg.api(&format!("/customers/{customer_id}.json"));
            let _    = put_json(&cfg, &url, &body).await?;
            Ok(format!("Client {customer_id} mis à jour."))
        }

        "shopify_list_collections" => {
            let limit = args["limit"].as_u64().unwrap_or(50);
            let url   = cfg.api(&format!("/custom_collections.json?limit={limit}"));
            let resp  = get(&cfg, &url).await?;
            let cols  = resp["custom_collections"].as_array().cloned().unwrap_or_default();

            if cols.is_empty() {
                return Ok("Aucune collection trouvée.".to_string());
            }

            let mut out = format!("{} collection(s) :\n\n", cols.len());
            for col in &cols {
                let id     = col["id"].as_u64().unwrap_or(0);
                let title  = col["title"].as_str().unwrap_or("—");
                let handle = col["handle"].as_str().unwrap_or("—");
                let count  = col["products_count"].as_u64().unwrap_or(0);
                out.push_str(&format!("• [{id}] {title} (handle: {handle}) — {count} produit(s)\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_get_inventory_levels" => {
            let limit = args["limit"].as_u64().unwrap_or(50);

            // First get locations
            let loc_url  = cfg.api("/locations.json");
            let loc_resp = get(&cfg, &loc_url).await?;
            let locations = loc_resp["locations"].as_array().cloned().unwrap_or_default();

            if locations.is_empty() {
                return Ok("Aucun emplacement (location) configuré.".to_string());
            }

            let location_ids: Vec<String> = locations.iter()
                .filter_map(|l| l["id"].as_u64())
                .map(|id| id.to_string())
                .collect();

            let ids_param = location_ids.join(",");
            let inv_url   = cfg.api(&format!("/inventory_levels.json?location_ids={ids_param}&limit={limit}"));
            let inv_resp  = get(&cfg, &inv_url).await?;
            let levels    = inv_resp["inventory_levels"].as_array().cloned().unwrap_or_default();

            if levels.is_empty() {
                return Ok("Aucun niveau d'inventaire disponible.".to_string());
            }

            // Build location name map
            let mut loc_names: std::collections::HashMap<u64, &str> = std::collections::HashMap::new();
            for l in &locations {
                if let (Some(id), Some(name)) = (l["id"].as_u64(), l["name"].as_str()) {
                    loc_names.insert(id, name);
                }
            }

            let mut out = format!("{} entrée(s) d'inventaire :\n\n", levels.len());
            for level in &levels {
                let inv_item_id = level["inventory_item_id"].as_u64().unwrap_or(0);
                let location_id = level["location_id"].as_u64().unwrap_or(0);
                let available   = level["available"].as_i64().unwrap_or(0);
                let loc_name    = loc_names.get(&location_id).copied().unwrap_or("—");
                out.push_str(&format!(
                    "• Item {inv_item_id} | Location: {loc_name} ({location_id}) | Disponible: {available}\n"
                ));
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_list_webhooks" => {
            let url      = cfg.api("/webhooks.json");
            let resp     = get(&cfg, &url).await?;
            let webhooks = resp["webhooks"].as_array().cloned().unwrap_or_default();

            if webhooks.is_empty() {
                return Ok("Aucun webhook configuré.".to_string());
            }

            let mut out = format!("{} webhook(s) :\n\n", webhooks.len());
            for wh in &webhooks {
                let id      = wh["id"].as_u64().unwrap_or(0);
                let topic   = wh["topic"].as_str().unwrap_or("—");
                let address = wh["address"].as_str().unwrap_or("—");
                let format  = wh["format"].as_str().unwrap_or("json");
                let created = wh["created_at"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {topic}\n  URL : {address}\n  Format : {format} | Créé : {created}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_create_webhook" => {
            let topic   = args["topic"].as_str().ok_or("Paramètre 'topic' requis")?;
            let address = args["address"].as_str().ok_or("Paramètre 'address' requis")?;
            let format  = args["format"].as_str().unwrap_or("json");

            let body = json!({
                "webhook": {
                    "topic": topic,
                    "address": address,
                    "format": format
                }
            });
            let url  = cfg.api("/webhooks.json");
            let resp = post_json(&cfg, &url, &body).await?;
            let wh   = &resp["webhook"];
            let id   = wh["id"].as_u64().unwrap_or(0);
            Ok(format!("Webhook créé (ID: {id}) pour le topic '{topic}' → {address}."))
        }

        "shopify_list_price_rules" => {
            let limit = args["limit"].as_u64().unwrap_or(50);
            let url   = cfg.api(&format!("/price_rules.json?limit={limit}"));
            let resp  = get(&cfg, &url).await?;
            let rules = resp["price_rules"].as_array().cloned().unwrap_or_default();

            if rules.is_empty() {
                return Ok("Aucune règle de prix configurée.".to_string());
            }

            let mut out = format!("{} règle(s) de prix :\n\n", rules.len());
            for r in &rules {
                let id             = r["id"].as_u64().unwrap_or(0);
                let title          = r["title"].as_str().unwrap_or("—");
                let value_type     = r["value_type"].as_str().unwrap_or("—");
                let value          = r["value"].as_str().unwrap_or("—");
                let usage_limit    = r["usage_limit"].as_u64();
                let times_used     = r["usage_count"].as_u64().unwrap_or(0);
                let starts_at      = r["starts_at"].as_str().unwrap_or("—");
                let ends_at        = r["ends_at"].as_str().unwrap_or("aucune");

                let usage_str = match usage_limit {
                    Some(limit) => format!("{times_used}/{limit} utilisations"),
                    None        => format!("{times_used} utilisations (illimité)"),
                };

                let type_str = match value_type {
                    "percentage"  => format!("{value}%"),
                    "fixed_amount" => format!("{value} (montant fixe)"),
                    other         => format!("{value} ({other})"),
                };

                out.push_str(&format!(
                    "• [{id}] {title}\n  Remise : {type_str} | {usage_str}\n  Valide : {starts_at} → {ends_at}\n"
                ));
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_list_locations" => {
            let url       = cfg.api("/locations.json");
            let resp      = get(&cfg, &url).await?;
            let locations = resp["locations"].as_array().cloned().unwrap_or_default();

            if locations.is_empty() {
                return Ok("Aucun emplacement (location) configuré.".to_string());
            }

            let mut out = format!("{} emplacement(s) :\n\n", locations.len());
            for l in &locations {
                let id      = l["id"].as_u64().unwrap_or(0);
                let name    = l["name"].as_str().unwrap_or("—");
                let addr1   = l["address1"].as_str().unwrap_or("—");
                let city    = l["city"].as_str().unwrap_or("—");
                let country = l["country"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {name} — {addr1}, {city}, {country}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_update_inventory" => {
            let location_id      = args["location_id"].as_i64().ok_or("Paramètre 'location_id' requis")?;
            let inventory_item_id = args["inventory_item_id"].as_i64().ok_or("Paramètre 'inventory_item_id' requis")?;
            let available        = args["available"].as_i64().ok_or("Paramètre 'available' requis")?;

            let body = json!({
                "location_id": location_id,
                "inventory_item_id": inventory_item_id,
                "available": available
            });
            let url = cfg.api("/inventory_levels/set.json");
            let resp = post_json(&cfg, &url, &body).await?;
            let level = &resp["inventory_level"];
            let returned_available = level["available"].as_i64().unwrap_or(available);
            Ok(format!(
                "Inventaire mis à jour — item {inventory_item_id} @ location {location_id} : {returned_available} disponible(s)."
            ))
        }

        "shopify_list_product_variants" => {
            let product_id = args["product_id"].as_str().ok_or("Paramètre 'product_id' requis")?;
            let url        = cfg.api(&format!("/products/{product_id}/variants.json"));
            let resp       = get(&cfg, &url).await?;
            let variants   = resp["variants"].as_array().cloned().unwrap_or_default();

            if variants.is_empty() {
                return Ok(format!("Aucune variante pour le produit {product_id}."));
            }

            let mut out = format!("{} variante(s) pour le produit {product_id} :\n\n", variants.len());
            for v in &variants {
                out.push_str(&format_variant(v));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_update_product_variant" => {
            let variant_id = args["variant_id"].as_str().ok_or("Paramètre 'variant_id' requis")?;
            let mut variant = json!({ "id": variant_id });
            if let Some(v) = args["price"].as_str()            { variant["price"] = json!(v); }
            if let Some(v) = args["sku"].as_str()              { variant["sku"] = json!(v); }
            if let Some(v) = args["compare_at_price"].as_str() { variant["compare_at_price"] = json!(v); }

            let body = json!({ "variant": variant });
            let url  = cfg.api(&format!("/variants/{variant_id}.json"));
            let _    = put_json(&cfg, &url, &body).await?;
            Ok(format!("Variante {variant_id} mise à jour."))
        }

        "shopify_refund_order" => {
            let order_id = args["order_id"].as_str().ok_or("Paramètre 'order_id' requis")?;
            let amount   = args["amount"].as_str().ok_or("Paramètre 'amount' requis")?;
            let note     = args["note"].as_str().unwrap_or("");

            // Retrieve original order to get gateway
            let order_url  = cfg.api(&format!("/orders/{order_id}.json"));
            let order_resp = get(&cfg, &order_url).await?;
            let gateway    = order_resp["order"]["gateway"].as_str().unwrap_or("manual").to_string();

            let mut refund_obj = json!({
                "refund_line_items": [],
                "transactions": [{
                    "kind": "refund",
                    "gateway": gateway,
                    "amount": amount
                }]
            });
            if !note.is_empty() {
                refund_obj["note"] = json!(note);
            }

            let body = json!({ "refund": refund_obj });
            let url  = cfg.api(&format!("/orders/{order_id}/refunds.json"));
            let resp = post_json(&cfg, &url, &body).await?;
            let refund = &resp["refund"];
            let id     = refund["id"].as_u64().unwrap_or(0);
            Ok(format!("Remboursement créé (ID: {id}) — {amount} remboursé sur la commande {order_id}."))
        }

        "shopify_list_draft_orders" => {
            let status = args["status"].as_str().unwrap_or("open");
            let url    = cfg.api(&format!("/draft_orders.json?limit=50&status={status}"));
            let resp   = get(&cfg, &url).await?;
            let orders = resp["draft_orders"].as_array().cloned().unwrap_or_default();

            if orders.is_empty() {
                return Ok(format!("Aucune commande brouillon (statut: {status})."));
            }

            let mut out = format!("{} commande(s) brouillon [statut: {status}] :\n\n", orders.len());
            for d in &orders {
                out.push_str(&format_draft_order(d));
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_create_draft_order" => {
            let line_items_str = args["line_items"].as_str().ok_or("Paramètre 'line_items' requis")?;
            let line_items: Value = serde_json::from_str(line_items_str)
                .map_err(|e| format!("Format JSON invalide pour line_items : {e}"))?;

            let mut draft = json!({ "line_items": line_items });
            if let Some(cid) = args["customer_id"].as_str() {
                draft["customer"] = json!({ "id": cid });
            }
            if let Some(note) = args["note"].as_str() {
                draft["note"] = json!(note);
            }

            let body = json!({ "draft_order": draft });
            let url  = cfg.api("/draft_orders.json");
            let resp = post_json(&cfg, &url, &body).await?;
            let d    = &resp["draft_order"];
            let id   = d["id"].as_u64().unwrap_or(0);
            let name = d["name"].as_str().unwrap_or("—");
            Ok(format!("Commande brouillon créée — {name} (ID: {id})."))
        }

        "shopify_complete_draft_order" => {
            let id  = args["id"].as_str().ok_or("Paramètre 'id' requis")?;
            let url = cfg.api(&format!("/draft_orders/{id}/complete.json"));
            let _   = put_json(&cfg, &url, &json!({})).await?;
            Ok(format!("Commande brouillon {id} marquée comme complète/payée."))
        }

        "shopify_list_transactions" => {
            let order_id   = args["order_id"].as_str().ok_or("Paramètre 'order_id' requis")?;
            let url        = cfg.api(&format!("/orders/{order_id}/transactions.json"));
            let resp       = get(&cfg, &url).await?;
            let txns       = resp["transactions"].as_array().cloned().unwrap_or_default();

            if txns.is_empty() {
                return Ok(format!("Aucune transaction pour la commande {order_id}."));
            }

            let mut out = format!("{} transaction(s) pour la commande {order_id} :\n\n", txns.len());
            for t in &txns {
                let tid     = t["id"].as_u64().unwrap_or(0);
                let kind    = t["kind"].as_str().unwrap_or("—");
                let status  = t["status"].as_str().unwrap_or("—");
                let amount  = t["amount"].as_str().unwrap_or("—");
                let gateway = t["gateway"].as_str().unwrap_or("—");
                let created = t["created_at"].as_str().unwrap_or("—");
                out.push_str(&format!(
                    "• [{tid}] {kind} | Statut : {status} | Montant : {amount} | Passerelle : {gateway} | {created}\n"
                ));
            }
            Ok(out.trim_end().to_string())
        }

        "shopify_create_collection" => {
            let title = args["title"].as_str().ok_or("Paramètre 'title' requis")?;
            let mut collection = json!({ "title": title });
            if let Some(v) = args["body_html"].as_str() {
                collection["body_html"] = json!(v);
            }

            let body = json!({ "custom_collection": collection });
            let url  = cfg.api("/custom_collections.json");
            let resp = post_json(&cfg, &url, &body).await?;
            let col  = &resp["custom_collection"];
            let id   = col["id"].as_u64().unwrap_or(0);
            Ok(format!("Collection '{title}' créée (ID: {id})."))
        }

        "shopify_list_smart_collections" => {
            let limit = args["limit"].as_u64().unwrap_or(50);
            let url   = cfg.api(&format!("/smart_collections.json?limit={limit}"));
            let resp  = get(&cfg, &url).await?;
            let cols  = resp["smart_collections"].as_array().cloned().unwrap_or_default();

            if cols.is_empty() {
                return Ok("Aucune collection automatique (smart collection) trouvée.".to_string());
            }

            let mut out = format!("{} collection(s) automatique(s) :\n\n", cols.len());
            for col in &cols {
                let id     = col["id"].as_u64().unwrap_or(0);
                let title  = col["title"].as_str().unwrap_or("—");
                let handle = col["handle"].as_str().unwrap_or("—");
                let count  = col["products_count"].as_u64().unwrap_or(0);
                let rules  = col["rules"].as_array().map(|r| r.len()).unwrap_or(0);
                out.push_str(&format!("• [{id}] {title} (handle: {handle}) — {count} produit(s) | {rules} règle(s)\n"));
            }
            Ok(out.trim_end().to_string())
        }

        _ => Err(format!("Tool Shopify inconnu : {name}")),
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
                    vec![
                        '%',
                        char::from_digit((b >> 4) as u32, 16).unwrap_or('0').to_ascii_uppercase(),
                        char::from_digit((b & 0xf) as u32, 16).unwrap_or('0').to_ascii_uppercase(),
                    ]
                }).collect()
            }
        })
        .collect()
}
