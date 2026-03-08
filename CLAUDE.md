# OSMOzzz — Chef de Projet

## Vision

OSMOzzz est le hub de données local pour les IAs externes (Claude, etc.).
Objectif : donner à Claude un accès complet à toutes les données personnelles
(emails, fichiers, messages, historique, notes, calendrier, outils cloud) sans jamais
envoyer de données à l'extérieur. Tout tourne sur le Mac de l'utilisateur.

## Stack technique

- Rust 2021 · LanceDB 0.14 · ONNX Runtime (ort v2) · all-MiniLM-L6-v2 (384d)
- Transport MCP : stdin/stdout JSON-RPC 2.0
- DB : `~/.osmozzz/vault/` · Socket UDS : `~/.osmozzz/osmozzz.sock`
- Dashboard : React + Vite (embarqué dans le binaire via `include_dir!`)
- REST API : Axum sur `127.0.0.1:PORT`

## Architecture des crates

```
osmozzz-core      → types partagés (Document, SearchResult, traits Harvester/Embedder)
osmozzz-harvester → toutes les sources de données (18 harvesters)
osmozzz-embedder  → ONNX + LanceDB (OnnxEmbedder, VectorStore, Vault)
osmozzz-bridge    → serveur UDS
osmozzz-api       → REST API + serveur dashboard (Axum)
osmozzz-cli       → CLI + serveur MCP + daemon
```

## Sources de données implémentées (18)

### Sources locales macOS — toujours disponibles, pas de config
| Source | Fichier harvester | Données indexées |
|--------|------------------|------------------|
| Chrome | `chrome.rs` | Historique navigation (titre + URL) |
| Safari | `safari.rs` | Historique navigation (titre + URL) |
| Gmail | `gmail.rs` | Emails IMAP (objet + corps) — config `~/.osmozzz/gmail.toml` |
| iMessage | `imessage.rs` | Messages SMS/iMessage |
| Apple Notes | `notes.rs` | Notes (titre + snippet) |
| Apple Calendar | `calendar.rs` | Événements calendrier |
| Terminal | `terminal.rs` | Historique zsh (~/.zsh_history) |
| Fichiers | `files.rs` | Fichiers .md/.txt Desktop/Documents |

### Sources cloud — nécessitent un token dans `~/.osmozzz/*.toml`
| Source | Fichier harvester | Config | Sync auto |
|--------|------------------|--------|-----------|
| Notion | `notion.rs` | `notion.toml` | 1h |
| GitHub | `github.rs` | `github.toml` | 1h |
| Linear | `linear.rs` | `linear.toml` | 1h |
| Jira | `jira.rs` | `jira.toml` | 1h |
| Slack | `slack.rs` | `slack.toml` | 30min |
| Trello | `trello.rs` | `trello.toml` | 1h |
| Todoist | `todoist.rs` | `todoist.toml` | 15min |
| GitLab | `gitlab.rs` | `gitlab.toml` | 1h |
| Airtable | `airtable.rs` | `airtable.toml` | 1h |
| Obsidian | `obsidian.rs` | `obsidian.toml` | 5min |

## Les 24 tools MCP

| Tool | Moteur | Source |
|------|--------|--------|
| `search_memory` | ONNX + LanceDB vectoriel (blended) | Toutes sources |
| `search_emails` | keyword `.contains()` | email |
| `get_emails_by_date` | filtre date LanceDB | email |
| `read_email` | LanceDB par URL | email |
| `search_messages` | keyword `.contains()` | imessage |
| `search_notes` | keyword `.contains()` | notes |
| `search_terminal` | keyword `.contains()` | terminal |
| `search_calendar` | keyword `.contains()` | calendar |
| `search_notion` | keyword `.contains()` | notion |
| `search_github` | keyword `.contains()` | github |
| `search_linear` | keyword `.contains()` | linear |
| `search_jira` | keyword `.contains()` | jira |
| `search_slack` | keyword `.contains()` | slack |
| `search_trello` | keyword `.contains()` | trello |
| `search_todoist` | keyword `.contains()` | todoist |
| `search_gitlab` | keyword `.contains()` | gitlab |
| `search_airtable` | keyword `.contains()` | airtable |
| `search_obsidian` | keyword `.contains()` | obsidian |
| `find_file` | walkdir filesystem | fichiers Mac |
| `fetch_content` | lecture fichier + ONNX optionnel | fichiers Mac |
| `get_recent_files` | walkdir + mtime | fichiers Mac |
| `list_directory` | std::fs::read_dir | fichiers Mac |
| `index_files` | FileHarvester | fichiers Mac |
| `get_status` | counts LanceDB | Toutes sources |

## Dashboard web

Interface React embarquée dans le binaire (`include_dir!` au build).
Pages : Statut · Recherche · Récents · Configuration

**Règle d'affichage des sources :**
- Sources locales (8) : toujours présentes dans Statut et Récents
- Sources cloud (10) : présentes seulement si le `.toml` de config existe

**Configuration des connecteurs :** uniquement via le dashboard (page Configuration).
Jamais de modification manuelle des `.toml` par l'utilisateur.

## Build & Deploy

### Build rapide (développement) — utiliser TOUJOURS cette méthode
```bash
# Depuis la racine du workspace
./build.sh
osmozzz daemon
```
`build.sh` fait :
1. `npm run build` **seulement si** `dashboard/src/` a changé depuis le dernier build
2. `cargo build --release -p osmozzz-cli` (incremental — ~30s si peu de changements)
3. `cp target/release/osmozzz ~/.cargo/bin/osmozzz`

### Build complet (première fois ou après changement de dépendances)
```bash
cd dashboard && npm run build
touch crates/osmozzz-api/src/server.rs
cargo install --path crates/osmozzz-cli --locked
osmozzz daemon
```

**Important :** `include_dir!` dans `server.rs` embarque `dashboard/dist/` dans le binaire.
- Frontend changé → `npm run build` + `touch server.rs` requis
- Rust seul changé → `./build.sh` suffit (skip npm automatiquement)

## Architecture MCP — Comment fonctionne le système Claude ↔ OSMOzzz

**OSMOzzz fait TOUT le travail de recherche et de filtrage. Claude ne voit que des résultats déjà triés.**

```
Claude Desktop ──► osmozzz (process Rust)
                   stdin : {"method":"tools/call","params":{"name":"search_emails","arguments":{…}}}
                   stdout: {"result":{"content":[{"type":"text","text":"EMAIL #1 | Objet: …"}]}}
```

### Les 3 moteurs de recherche

**1. Vectoriel ONNX → LanceDB (sémantique)**
- Tool : `search_memory`
- Blended : global top-N + 3 résultats email forcés (emails ont des scores faibles 0.27-0.35 vs Chrome 0.72-0.85)

**2. Keyword scan `.contains()` (exact)**
- 14 tools de recherche par source
- Scan de 100k docs en mémoire Rust → filtre → tri par date

**3. Filesystem direct**
- `find_file`, `fetch_content`, `get_recent_files`, `list_directory`

## Règles globales

1. **Jamais de données hors du Mac** — tout est local
2. **Philosophie "moins c'est plus"** — pas de sur-ingénierie
3. **Build toujours dans l'ordre** : `npm run build` → `touch server.rs` → `cargo install`
4. **Pas de breaking changes** sur le schéma LanceDB sans migration
5. **Un sous-agent par domaine** — ne pas mélanger les responsabilités
6. **Configuration utilisateur** : uniquement via le dashboard, jamais via CLI/toml manuels

## Pattern d'un nouveau harvester

1. Créer `crates/osmozzz-harvester/src/nom_source.rs`
2. Implémenter le trait `Harvester` de osmozzz-core
3. Exporter depuis `crates/osmozzz-harvester/src/lib.rs`
4. Ajouter CLI dans `crates/osmozzz-cli/src/commands/index.rs`
5. Ajouter MCP tool dans `crates/osmozzz-cli/src/commands/mcp.rs`
6. Ajouter la source dans `osmozzz-api/src/routes.rs` (get_status + get_config)
7. Ajouter la card dans le dashboard (ConfigPage, StatusPage, RecentPage)
8. Build : `npm run build` → `touch server.rs` → `cargo install`

## Sous-agents disponibles

| Agent | CLAUDE.md | Domaine |
|-------|-----------|---------|
| Harvester Agent | `crates/osmozzz-harvester/CLAUDE.md` | Nouvelles sources de données |
| MCP Tools Agent | `crates/osmozzz-cli/CLAUDE.md` | Interface Claude (tools MCP) |
| Storage Agent | `crates/osmozzz-embedder/CLAUDE.md` | LanceDB, recherche, embeddings |

## Roadmap

### Fait ✅
- Sources locales macOS : Chrome, Safari, Gmail, iMessage, Notes, Calendar, Terminal, Fichiers
- Sources cloud : Notion, GitHub, Linear, Jira, Slack, Trello, Todoist, GitLab, Airtable, Obsidian
- 24 tools MCP
- Dashboard web : Statut, Recherche, Récents, Configuration
- REST API (Axum) avec configuration des connecteurs
- Daemon avec auto-sync par source
- Blacklist / ban de documents
- Compact LanceDB

### À faire 🔲
- Recherche hybride BM25 + vecteurs
- PDF support
- Proof of Context (HMAC-SHA256) — déjà commencé

### En cours 🔧 — P2P Mesh Enterprise

**Objectif :** Réseau de daemons OSMOzzz qui collaborent. Les données restent sur chaque machine. Seules les réponses (filtrées par le pare-feu) transitent.

**Principe fondamental :**
- Les index LanceDB ne bougent JAMAIS de la machine locale
- Ton OSMOzzz envoie une requête → l'OSMOzzz du collègue cherche dans SES données → renvoie uniquement le texte de réponse filtré
- Le pare-feu de confidentialité s'applique AVANT l'envoi vers un peer

**Nouveau crate : `osmozzz-p2p`**
```
crates/osmozzz-p2p/src/
├── identity.rs      ← clé Ed25519 par machine (persistée dans ~/.osmozzz/identity.toml)
├── node.rs          ← serveur TCP/TLS + client (tokio + rustls)
├── discovery.rs     ← mDNS pour réseau local auto (mdns-sd)
├── protocol.rs      ← messages JSON entre daemons (Search, SearchResult, Ping, Info)
├── permissions.rs   ← contrôle d'accès par source par peer
├── store.rs         ← peers.toml (liste des peers connus + leurs permissions)
└── history.rs       ← log des requêtes reçues (qui a cherché quoi quand)
```

**Transport choisi : Option A (simple, enterprise-ready)**
- TCP + TLS (rustls + Ed25519)
- Invitation via lien contenant IP:port + clé publique
- mDNS pour auto-découverte sur réseau local
- Fonctionne avec IPs fixes / VPN entreprise (pas de NAT traversal complexe)

**Nouvelles routes API :**
- GET  /api/network/peers         → liste des peers connectés
- POST /api/network/invite        → génère un lien d'invitation
- POST /api/network/connect       → connexion depuis un lien
- DELETE /api/network/peer/:id    → déconnecte un peer
- GET  /api/network/permissions   → mes permissions de partage
- POST /api/network/permissions   → met à jour mes permissions
- GET  /api/network/history       → historique des requêtes reçues

**Nouveau tool MCP :**
- `search_network(query, peer_id?)` → cherche chez tous les peers autorisés (ou un seul)

**Dashboard — nouvelle page "Réseau" :**
- Vue des connexions actives avec sources partagées par peer
- Génération d'invitation (QR code + lien)
- Permissions granulaires par source par peer
- Historique des requêtes reçues
- Notifications badge quand un collègue t'interroge

**Recherche dashboard — filtre périmètre :**
- Dropdown [Tout le monde ▾] par défaut
- Options : Moi seulement | Thomas | Sarah | ...

**Règles d'implémentation :**
1. Le pare-feu (osmozzz-core::filter) s'applique TOUJOURS avant d'envoyer au peer
2. Chaque résultat retourné à Claude identifie l'auteur (peer_id + peer_name)
3. Un peer offline = ses résultats sont absents silencieusement (pas d'erreur)
4. Permissions révocables instantanément (rechargées à chaque requête)
5. Historique append-only en JSONL (~/.osmozzz/query_history.jsonl)
