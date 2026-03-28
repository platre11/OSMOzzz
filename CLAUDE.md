# OSMOzzz — Documentation Complète

http://localhost:5173/
http://localhost:3000/
npx @anthropic-ai/claude-code

---

## ⚠️ RÈGLES ABSOLUES — BUILD & DAEMON

### 1. Toujours utiliser `./build.sh`

**Après CHAQUE modification de code (Rust OU dashboard), Claude DOIT exécuter `./build.sh`.**

```bash
./build.sh
```

Ce script fait dans l'ordre :
1. `npm run build` si dashboard modifié
2. `touch crates/osmozzz-api/src/server.rs` (force l'embed du dashboard — TOUJOURS)
3. `cargo build --release -p osmozzz-cli`
4. `cp target/release/osmozzz ~/.cargo/bin/osmozzz`
5. `pkill -f "osmozzz daemon"` — kill le daemon actif

**JAMAIS** utiliser `cargo build` seul — ça ne met PAS à jour `~/.cargo/bin/osmozzz`.

### 2. Binaire de développement = `~/.cargo/bin/osmozzz`

- `/usr/local/bin/osmozzz` = version .pkg pour les utilisateurs finaux — **NE JAMAIS MODIFIER**
- `~/.cargo/bin/osmozzz` = binaire de développement — celui qu'on build
- Si le PATH a `/usr/local/bin` avant `~/.cargo/bin`, le mauvais binaire tourne → dashboard figé sur l'ancienne version

### 3. Après le build

`./build.sh` kill automatiquement le daemon. L'utilisateur relance lui-même :
```bash
~/.cargo/bin/osmozzz daemon
```
Puis **Cmd+Shift+R** dans le navigateur sur `localhost:7878`.

---

## Vision

OSMOzzz est une source de données centrale, locale et privée, conçue pour collaborer avec tout client IA compatible MCP.

L'objectif : rester maître de ses données. Que ce soit pour **rechercher des informations** dans ses outils externes (emails, fichiers, notes, Notion, Slack, GitHub…) ou pour **déclencher des actions** (envoyer un message, créer une tâche, modifier un fichier), tout passe par OSMOzzz — sans jamais transmettre de données sensibles à l'extérieur.

Le client IA interroge OSMOzzz via ses tools MCP. OSMOzzz fait tout le travail de recherche, de filtrage et d'exécution. L'IA ne voit que des résultats déjà triés et approuvés par l'utilisateur.

## Stack technique

- **Rust 2021** · LanceDB 0.14 · ONNX Runtime (ort 2.0.0-rc.11, load-dynamic) · all-MiniLM-L6-v2 (384d)
- **Transport MCP** : stdin/stdout JSON-RPC 2.0 (tout client IA compatible MCP)
- **DB** : `~/.osmozzz/vault/` (LanceDB parquet) · Socket UDS legacy : `~/.osmozzz/osmozzz.sock`
- **Dashboard** : React 18 + TypeScript + Vite (embarqué dans le binaire via `include_dir!`)
- **REST API** : Axum sur `127.0.0.1:7878`
- **P2P** : iroh (QUIC + relay n0.computer) + Ed25519 + ALPN `osmozzz/1`
- **Clés dépendances** : tokio full · serde/serde_json · reqwest 0.12 · rusqlite 0.31 bundled · iroh 0.96 · ed25519-dalek 2 · mdns-sd 0.11 · notify 6 · lettre 0.11 · pdf-extract

---

## Architecture des crates (7 crates)

```
osmozzz-core      → types partagés (Document, SearchResult, ActionRequest, traits, PrivacyFilter)
osmozzz-harvester → toutes les sources de données (20 harvesters + FSEvents watcher)
osmozzz-embedder  → ONNX + LanceDB (OnnxEmbedder, VectorStore, Vault, Blacklist)
osmozzz-bridge    → serveur UDS legacy (stdin/stdout bridge, peu utilisé)
osmozzz-api       → REST API + dashboard (Axum) + ActionQueue + Executor
osmozzz-cli       → CLI (clap) + serveur MCP + daemon + MCP proxies
osmozzz-p2p       → réseau P2P mesh (iroh QUIC, identité Ed25519, permissions, audit)
```

---

## Architecture Duale — Harvesters + MCP Proxies (FONDAMENTAL)

**OSMOzzz est TOUJOURS l'intermédiaire unique entre Claude et toutes les sources de données.**
Claude ne parle jamais directement à Notion, Jira, Supabase, etc. Tout passe par OSMOzzz.

Il existe deux systèmes complémentaires dans OSMOzzz :

### 1. Harvesters Rust — Indexation locale

Les harvesters appellent les APIs cloud directement via `reqwest`, transforment les données en `Document` et les indexent dans LanceDB (ONNX).

```
API cloud → harvester Rust (reqwest) → Vec<Document> → Vault (LanceDB) → search_* tools
```

Utilisés pour : **recherche sémantique** dans les données passées.

### 2. MCP Proxies — Subprocesses MCP tiers

Pour les **actions en temps réel** (créer une issue, envoyer un message, exécuter du SQL...), OSMOzzz lance des **subprocesses MCP via `bunx`** (Bun). Ces subprocesses sont des packages npm MCP officiels, proxifiés par OSMOzzz en JSON-RPC stdin/stdout.

```
Claude → OSMOzzz MCP (Rust) → subprocess bunx @pkg/mcp-server → API cloud
```

**Fichiers** : `crates/osmozzz-cli/src/mcp_proxy/`

| Fichier       | Package npm                           | Config                     |
| ------------- | ------------------------------------- | -------------------------- |
| `jira.rs`     | `@aashari/mcp-server-atlassian-jira`  | `~/.osmozzz/jira.toml`     |
| `github.rs`   | `@modelcontextprotocol/server-github` | `~/.osmozzz/github.toml`   |
| `notion.rs`   | `@notionhq/notion-mcp-server`         | `~/.osmozzz/notion.toml`   |
| `slack.rs`    | `@modelcontextprotocol/server-slack`  | `~/.osmozzz/slack.toml`    |
| `linear.rs`   | `@tacticlaunch/mcp-linear`            | `~/.osmozzz/linear.toml`   |
| `supabase.rs` | `@supabase/mcp-server-supabase`       | `~/.osmozzz/supabase.toml` |

**Mécanisme** (`McpSubprocess`) :

- Vérifie/installe Bun automatiquement (`~/.bun/bin/bun`)
- Lance `bunx x --bun <package>` avec les env vars du `.toml`
- Handshake JSON-RPC 2.0 (`initialize` + `notifications/initialized`)
- Découverte automatique des tools (`tools/list`)
- Proxifie les appels de Claude vers le subprocess (`tools/call`)
- Si le `.toml` est absent → subprocess non démarré silencieusement

**Démarrage** : `start_all_proxies()` dans `mod.rs` → appelé au lancement de `osmozzz mcp`.

### Tableau comparatif

|                | Harvester (Rust)        | MCP Proxy (Subprocess)             |
| -------------- | ----------------------- | ---------------------------------- |
| **Rôle**       | Indexation (passé)      | Actions temps réel                 |
| **Transport**  | reqwest HTTP            | bunx subprocess JSON-RPC           |
| **Output**     | Vec<Document> → LanceDB | Réponse JSON → Claude              |
| **Tools**      | `search_*` dans mcp.rs  | Tools natifs du package npm        |
| **Dépendance** | Cargo (reqwest)         | Bun runtime (auto-installé)        |
| **Config**     | `~/.osmozzz/*.toml`     | `~/.osmozzz/*.toml` (même fichier) |

### Supabase MCP Proxy — Détail

**Package** : `@supabase/mcp-server-supabase` (officiel Supabase)
**Config** : `~/.osmozzz/supabase.toml`

```toml
access_token = "sbp_xxxx..."   # Personal Access Token (supabase.com/dashboard/account/tokens)
project_id   = "xxxx"          # optionnel — restreint à un projet spécifique
```

**~38 tools disponibles** répartis en groupes :

| Groupe          | Tools principaux                                                            |
| --------------- | --------------------------------------------------------------------------- |
| Base de données | `execute_sql`, `list_tables`, `list_extensions`, `apply_migration`          |
| Debugging       | `get_logs` (API, PostgreSQL, Edge Functions, Auth, Storage), `get_advisors` |
| Développement   | `get_project_url`, `get_publishable_keys`, `generate_typescript_types`      |
| Edge Functions  | `list_edge_functions`, `deploy_edge_function`                               |
| Storage         | `list_storage_buckets`, `get_storage_config`                                |
| Branching       | `create_branch`, `list_branches`, `merge_branch`, `reset_branch`            |
| Compte          | `list_projects`, `create_project`, `pause_project`, `get_cost`              |
| Docs            | `search_docs`                                                               |

### Pattern pour ajouter un nouveau MCP Proxy

1. Créer `crates/osmozzz-cli/src/mcp_proxy/nom.rs` (charger config TOML + appeler `McpSubprocess::start()`)
2. Déclarer `pub mod nom;` dans `mod.rs`
3. Ajouter `if let Some(p) = nom::start() { proxies.push(p); }` dans `start_all_proxies()`
4. Ajouter la route `POST /api/config/nom` dans `routes.rs`
5. Ajouter card dans le dashboard (ConfigPage)

---

## osmozzz-core — Types partagés

### Types principaux (`src/types.rs`)

**SourceType** (21 variantes) :
`Chrome, File, Pdf, Markdown, Email, IMessage, Safari, Notes, Calendar, Terminal,
Notion, Github, Linear, Jira, Slack, Trello, Todoist, Gitlab, Airtable, Obsidian, Contacts, Arc`

**Document** :

```
id            : String (UUID v4)
source        : SourceType
url           : String (identifiant unique du document)
title         : Option<String>
content       : String (texte indexé)
checksum      : String (SHA-256, déduplication)
harvested_at  : i64 (timestamp Unix)
source_ts     : Option<i64> (timestamp original de la source)
chunk_index   : Option<i32> (position du chunk)
chunk_total   : Option<i32> (nombre total de chunks)
```

**SearchResult** : id, score [0.0–1.0], source, url, title?, content, chunk_index?, chunk_total?

### Traits (`src/traits.rs`)

```rust
trait Harvester {
    async fn harvest(&self) -> Result<Vec<Document>>
}
trait Embedder {
    async fn upsert(&self, doc: &Document) -> Result<()>
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>>
    async fn exists(&self, checksum: &str) -> Result<bool>
    async fn count(&self) -> Result<usize>
}
```

### Système d'actions (`src/action.rs`)

**ActionStatus** : `Pending | Approved | Rejected | Expired`

**ActionRequest** :

```
id             : String (UUID v4)
tool           : String (nom du tool MCP ex: "act_send_email")
params         : serde_json::Value
preview        : String (description lisible pour l'utilisateur)
status         : ActionStatus
created_at     : i64
expires_at     : i64 (created_at + 300s — expire si pas de réponse en 5 min)
execution_result : Option<String> (résultat après exécution)
mcp_proxy      : Option<serde_json::Value>
```

### Système de confidentialité (`src/filter/`)

**PrivacyConfig** (TOML à `~/.osmozzz/privacy.toml`) :

```toml
credit_card = true   # masque les numéros CB
iban        = true   # masque les IBAN
api_keys    = true   # masque les clés API/tokens
email       = false  # masque les adresses email
phone       = false  # masque les numéros de téléphone
```

Le filtre s'applique sur le contenu **avant** envoi aux peers P2P.

---

## osmozzz-harvester — Sources de données (20 harvesters)

### Sources locales macOS

| Harvester     | Source type | Sync daemon | Méthode                                                                          |
| ------------- | ----------- | ----------- | -------------------------------------------------------------------------------- |
| `chrome.rs`   | `chrome`    | Manuel      | Shadow copy SQLite `~/Library/Application Support/Google/Chrome/Default/History` |
| `safari.rs`   | `safari`    | 60s         | Shadow copy SQLite `~/Library/Safari/History.db`                                 |
| `imessage.rs` | `imessage`  | 60s         | Shadow copy SQLite `~/Library/Messages/chat.db` + Contacts                       |
| `notes.rs`    | `notes`     | 60s         | AppleScript bridge                                                               |
| `calendar.rs` | `calendar`  | 60s         | AppleScript bridge                                                               |
| `terminal.rs` | `terminal`  | 60s         | `~/.zsh_history`                                                                 |
| `contacts.rs` | `contacts`  | 10min       | macOS Contacts DB                                                                |
| `arc.rs`      | `arc`       | 60s         | Arc browser history SQLite                                                       |

### Sources fichiers locaux

**`files.rs` — FileHarvester** (source type : `file`, `markdown`, `pdf`) :

- Répertoires scannés : `~/Desktop`, `~/Documents`, `~/Downloads`
- Extensions texte (8) : `md, mdx, txt, rst, org, tex, adoc, csv`
- Extensions image ignorées (17) : `png, jpg, jpeg, gif, svg, webp, ico, bmp, tiff, heic, heif, avif, raw, cr2, nef, arw`
- PDF extrait et chunké si < 5 MB
- **Chunking** : `MAX_CHARS=1600`, `OVERLAP_CHARS=160`, `MAX_TEXT_BYTES=2MB`
- Dirs ignorés (24) : `node_modules, .git, target, __pycache__, .cargo, dist, build, .next, .nuxt, vendor, .build, Pods, DerivedData, .gradle, .idea, venv, .venv, env, .tox, .osmozzz, Library, Temp, obj, Logs`

**`watcher.rs` — FSEvents monitor** :

- `DEBOUNCE_MS=500`, `STARTUP_GRACE_SECS=15` (ignore les events au démarrage)
- Émet `WatchEvent::Upsert(Vec<Document>)` sur création/modification
- Scrute : `~/Desktop`, `~/Documents`

### Sources cloud (nécessitent un fichier `~/.osmozzz/*.toml`)

| Harvester     | Source type | Config          | Sync daemon | API                       |
| ------------- | ----------- | --------------- | ----------- | ------------------------- |
| `gmail.rs`    | `email`     | `gmail.toml`    | 15min       | IMAP imap.gmail.com:993   |
| `notion.rs`   | `notion`    | `notion.toml`   | 60min       | REST api.notion.com       |
| `github.rs`   | `github`    | `github.toml`   | 60min       | REST api.github.com v3    |
| `linear.rs`   | `linear`    | `linear.toml`   | 60min       | GraphQL api.linear.app    |
| `jira.rs`     | `jira`      | `jira.toml`     | 60min       | REST Atlassian            |
| `slack.rs`    | `slack`     | `slack.toml`    | 30min       | REST slack.com/api        |
| `trello.rs`   | `trello`    | `trello.toml`   | 60min       | REST api.trello.com       |
| `todoist.rs`  | `todoist`   | `todoist.toml`  | 15min       | REST API v2               |
| `gitlab.rs`   | `gitlab`    | `gitlab.toml`   | 60min       | REST (défaut: gitlab.com) |
| `airtable.rs` | `airtable`  | `airtable.toml` | 60min       | REST api.airtable.com     |
| `obsidian.rs` | `obsidian`  | `obsidian.toml` | 5min        | Filesystem vault local    |

### Format des fichiers de config cloud

```toml
# gmail.toml
username = "user@gmail.com"
app_password = "xxxx xxxx xxxx xxxx"

# notion.toml
token = "secret_xxx"

# github.toml
token = "ghp_xxx"
repos = ["owner/repo1", "owner/repo2"]

# linear.toml
api_key = "lin_api_xxx"

# jira.toml
base_url = "https://monentreprise.atlassian.net"
email    = "user@company.com"
token    = "ATATT3xxx"

# slack.toml
token    = "xoxp-xxx"
team_id  = "TXXXXXXXX"
channels = ["general", "engineering"]

# trello.toml
api_key = "xxx"
token   = "xxx"

# todoist.toml
token = "xxx"

# gitlab.toml
token    = "glpat-xxx"
base_url = "https://gitlab.com"
groups   = ["my-group"]

# airtable.toml
token = "patXXX"
bases = ["appXXX"]

# obsidian.toml
vault_path = "~/Documents/MyVault"
```

---

## osmozzz-embedder — Stockage & Recherche

### Vault (`src/vault.rs`)

Interface principale :

```rust
Vault::open(model_path, tokenizer_path, db_path) -> Result<Vault>
vault.embed_raw(text) -> Result<Vec<f32>>
vault.compact() -> Result<()>
vault.store_text_only(doc) -> Result<()>
vault.ban_url(url)
vault.ban_source_item(source, identifier)
vault.search_emails_by_keyword(keyword, limit)
vault.search_by_keyword_source(keyword, limit, source)
vault.search_grouped_by_keyword(keyword, per_source)
vault.search_by_keyword_dated(keyword, limit, source)
vault.recent_by_source(source, limit)
vault.get_full_content_by_url(url)
```

### Schéma LanceDB (`documents` table)

```
id           String       UUID v4
source       String       "email", "chrome", "file", etc.
url          String       identifiant unique
title        String       optionnel
content      String       texte indexé
checksum     String       SHA-256 (déduplication)
harvested_at Int64        timestamp Unix
source_ts    Int64        timestamp source original (optionnel)
chunk_index  Int32        position chunk (optionnel)
chunk_total  Int32        total chunks (optionnel)
embedding    Vec<f32>     384 dimensions, L2 normalisé
```

### Modèle d'embedding

- **Modèle** : all-MiniLM-L6-v2 (SentenceTransformers)
- **Dimension** : 384 float32
- **Normalisation** : L2
- **Distance** : cosine [0, 2] → score `1.0 - dist/2.0` → [0, 1]
- **Chemins** : `~/.osmozzz/models/all-MiniLM-L6-v2.onnx` + `~/.osmozzz/models/tokenizer.json`

---

## Sécurité & Confidentialité

OSMOzzz expose quatre mécanismes de contrôle des données envoyées au client IA.

### 1. Filtre de confidentialité (`~/.osmozzz/privacy.toml`)

Masque automatiquement des patterns sensibles **dans tous les résultats** avant envoi au client IA.

```toml
credit_card = true   # masque les numéros CB
iban        = true   # masque les IBAN
api_keys    = true   # masque les clés API/tokens
email       = false  # masque les adresses email
phone       = false  # masque les numéros de téléphone
```

Géré via `GET/POST /api/privacy`. S'applique aussi avant tout envoi P2P vers un peer.

### 2. Alias d'identité / Pseudonymisation (`~/.osmozzz/aliases.toml`)

Remplace les vrais noms par des alias **avant** que le client IA ne reçoive les données.
Le client IA travaille uniquement avec l'alias — il ne voit jamais l'identité réelle.
Si le client IA cherche un alias dans un tool MCP, OSMOzzz résout l'alias vers le vrai nom dans le vault.

```toml
# ~/.osmozzz/aliases.toml
"Jean Pierre" = "Matisse Mouseu"
"user@gmail.com" = "contact-pro"
```

Géré via `GET/POST /api/aliases` et la section **Alias d'identité** du dashboard (page Confidentialité).

### 3. Liste noire / Blacklist (`~/.osmozzz/blacklist.toml`)

Exclut des documents ou des sources entières des résultats envoyés au client IA.

```toml
[urls]        # URLs de documents spécifiques
[gmail]       # adresses email des expéditeurs
[imessage]    # numéros de téléphone
[chrome]      # domaines (substring match)
[safari]      # domaines
[files]       # préfixes de chemins de fichiers
```

- `is_banned(source, url, title)` — vérifié à **l'indexation** (daemon, empêche l'entrée en DB)
- `is_result_banned(source, url, title, content)` — vérifié à la **recherche** (filtre les résultats sortants)

Géré via `POST /api/ban`, `POST /api/unban`, `GET /api/blacklist` et la section **Liste noire** du dashboard.

### 4. Journal d'accès (`~/.osmozzz/audit.jsonl`)

Log append-only de **tous les appels MCP** reçus (quel tool, quelle requête, combien de résultats, bloqué ou non).

```jsonl
{"ts":1710000000,"tool":"search_memory","query":"ismail PDF","results":13,"blocked":false}
{"ts":1710000001,"tool":"search_emails","query":"ismail","results":20,"blocked":false}
{"ts":1710000002,"tool":"search_notes","query":"ismail","results":1,"blocked":false}
```

Champs :

- `ts` — timestamp Unix
- `tool` — nom du tool MCP appelé (ex: `search_memory`, `search_emails`)
- `query` — requête envoyée
- `results` — nombre de résultats retournés
- `blocked` — true si le résultat a été bloqué par une règle de confidentialité
- `data` — optionnel, données supplémentaires

Consulté via `GET /api/audit?limit=200` et l'onglet **Journal d'accès** du dashboard (page Confidentialité/Actions).

### Moteurs de recherche (3)

1. **Vectoriel ONNX → LanceDB** (`search_memory`) — sémantique
   - Blended : top-N global + 3 emails forcés (emails ont scores 0.27–0.35 vs Chrome 0.72–0.85)
2. **Keyword `.contains()`** — exact, scan 100k docs en mémoire, tri par date
3. **Filesystem direct** — `find_file`, `fetch_content`, `get_recent_files`, `list_directory`

**Recherche multi-terme** (dashboard) : si `+` détecté → AND logique (ex: `qonto + style`)

---

## osmozzz-api — REST API & Dashboard

### AppState (`src/state.rs`)

```rust
pub struct AppState {
    vault: Arc<Vault>,
    p2p: Option<Arc<P2pNode>>,
    index_progress: Arc<Mutex<IndexProgress>>,
    action_queue: Arc<ActionQueue>,
}
```

### ActionQueue (`src/action_queue.rs`)

Thread-safe `VecDeque` + canal broadcast SSE :

- `push(action)` — ajoute + notifie les abonnés SSE
- `pending()` — filtre Pending, marque Expired si > 5min
- `approve(id)` / `reject(id)` — met à jour le statut + notifie
- `set_execution_result(id, result)` — stocke le résultat d'exécution
- `subscribe()` — retourne un `broadcast::Receiver<ActionEvent>`

### Executor (`src/executor.rs`) — 16 actions supportées

| Tool                        | Action                         |
| --------------------------- | ------------------------------ |
| `act_send_email`            | SMTP via Gmail config (lettre) |
| `act_create_notion_page`    | Notion API                     |
| `act_send_slack_message`    | Slack API                      |
| `act_create_linear_issue`   | Linear API                     |
| `act_create_todoist_task`   | Todoist API                    |
| `act_create_github_issue`   | GitHub API                     |
| `act_create_trello_card`    | Trello API                     |
| `act_create_gitlab_issue`   | GitLab API                     |
| `act_send_imessage`         | AppleScript bridge             |
| `act_create_calendar_event` | AppleScript                    |
| `act_delete_calendar_event` | AppleScript                    |
| `act_delete_note`           | AppleScript                    |
| `act_create_folder`         | Filesystem                     |
| `act_rename_file`           | Filesystem                     |
| `act_delete_file`           | Filesystem                     |
| `act_run_command`           | Shell (bash -c)                |

### Routes REST (`src/routes.rs`)

#### Statut & Recherche

```
GET  /api/status                              → counts par source + métriques
GET  /api/search?q=...&source=...&from=...&to=... → recherche groupée par source
GET  /api/recent?source=...&q=...&from=...&to=...&limit=200&offset=0
GET  /api/config                              → état des connecteurs (configured: bool)
```

#### Configuration des connecteurs (11)

```
POST /api/config/gmail
POST /api/config/notion
POST /api/config/github
POST /api/config/linear
POST /api/config/jira
POST /api/config/slack
POST /api/config/trello
POST /api/config/todoist
POST /api/config/gitlab
POST /api/config/airtable
POST /api/config/obsidian
```

#### Documents & Blacklist

```
GET  /api/open?url=...                        → ouvre un fichier dans Finder/app
GET  /api/messages/contacts                   → contacts iMessage
GET  /api/messages/conversation?contact=...  → conversation iMessage
POST /api/ban                                 → bannir url ou source+identifier
POST /api/unban                               → débannir
GET  /api/blacklist                           → liste des bannis
POST /api/compact                             → compacter LanceDB
POST /api/reindex/imessage                    → réindexer iMessages
GET  /api/files/search?q=...                  → recherche filesystem live
GET  /api/index/preview                       → preview fichiers qui seraient indexés
GET  /api/index/progress                      → progression de l'indexation en cours
POST /api/index                               → lancer une indexation
GET  /api/privacy                             → lire privacy.toml
POST /api/privacy                             → écrire privacy.toml
GET  /api/aliases                             → alias de recherche
POST /api/aliases                             → créer/modifier alias
```

#### Actions (workflow approbation)

```
GET  /api/actions                             → toutes les actions (historique)
GET  /api/actions/pending                     → actions en attente
GET  /api/actions/stream                      → SSE stream (ActionEvent)
GET  /api/actions/:id                         → détail d'une action
POST /api/actions/:id/approve                 → approuver
POST /api/actions/:id/reject                  → rejeter
GET  /api/permissions                         → règles de permissions globales
POST /api/permissions                         → mettre à jour permissions
GET  /api/source-access                       → accès par source
POST /api/source-access                       → modifier accès source
GET  /api/audit                               → log d'audit
```

#### Réseau P2P

```
GET    /api/network/peers                     → peers connus + état connexion
POST   /api/network/invite                    → générer lien d'invitation (base64 addr iroh)
POST   /api/network/connect                   → connecter depuis lien
DELETE /api/network/peers/:peer_id            → déconnecter un peer
GET    /api/network/permissions/:peer_id      → permissions d'un peer
POST   /api/network/permissions/:peer_id      → modifier permissions
GET    /api/network/history                   → historique des requêtes reçues
```

---

## osmozzz-cli — Interface utilisateur

### Commandes CLI

```bash
osmozzz index   --source chrome|safari|gmail|imessage|notes|calendar|terminal|files|
                          notion|github|linear|jira|slack|trello|todoist|gitlab|airtable|obsidian|contacts|arc
                [--path PATH] [--reset] [--batch-size N]

osmozzz search  QUERY [--source FILTER] [--limit N] [--format text|json]
osmozzz status                          # counts par source
osmozzz compact                         # merge LanceDB fragments
osmozzz daemon                          # serveur HTTP + watcher + auto-sync
osmozzz mcp                             # serveur MCP stdin/stdout (tout client IA compatible MCP)
osmozzz serve   [--socket PATH]         # UDS bridge legacy
osmozzz install                         # copie modèles ONNX + config client MCP
osmozzz verify  --sig S --source S --url U --content C --ts T  # Proof of Context
```

### Daemon (`src/commands/daemon.rs`)

Séquence de démarrage :

1. Copie des modèles ONNX vers `~/.osmozzz/models/`
2. Ouverture du Vault (LanceDB + ONNX)
3. Initialisation du nœud P2P iroh
4. Démarrage du serveur HTTP Axum (port 7878)
5. Ouverture du dashboard dans le navigateur
6. Démarrage du watcher FSEvents (Desktop + Documents)
7. Spawn des tâches auto-sync par source

---

## osmozzz-p2p — Réseau mesh

### Architecture

```
crates/osmozzz-p2p/src/
├── identity.rs    ← clé Ed25519 par machine (~/.osmozzz/identity.toml)
├── node.rs        ← nœud iroh (QUIC + relay) + gestion des connexions
├── protocol.rs    ← messages JSON (Ping/Pong, Hello/Welcome, Search/SearchResult, Info)
├── permissions.rs ← contrôle d'accès par source par peer
├── store.rs       ← peers.toml (liste persistée + permissions)
└── history.rs     ← log JSONL (~/.osmozzz/query_history.jsonl)
```

### Transport

- **Protocol** : iroh QUIC (UDP), ALPN `b"osmozzz/1"`
- **Port** : 47474 UDP (géré par iroh, pas fixe)
- **Relay** : n0.computer (fallback NAT / réseaux restrictifs)
- **Hole punching** : automatique (STUN + iroh relay)
- **Invitation** : lien base64 = endpoint iroh encodé

### Identité (`identity.rs`)

```toml
# ~/.osmozzz/identity.toml
peer_id          = "aabbcc..."  # hex 64 chars de la clé publique Ed25519
private_key_hex  = "..."
display_name     = "MacBook Pro de Thomas"
```

### Protocole de messages

```
Ping / Pong                          ← keepalive
Hello { peer_id, display_name }      ← handshake initial
Welcome { peer_id, display_name }    ← acknowledgment
Search { request_id, query, limit }  ← requête de recherche
SearchResult { request_id, peer_id, peer_name, results }
GetInfo / Info { peer_id, display_name, shared_sources, version }
Error { code, message }
```

### Permissions par peer

**Sources activées par défaut** : File, Notion, Github, Linear, Jira, Slack, Trello, Todoist, Gitlab, Airtable, Obsidian

**Sources désactivées par défaut** (sensibles) : Email, IMessage, Terminal, Chrome, Safari, Notes, Calendar

```
PeerPermissions {
    allowed_sources: Vec<SharedSource>,
    max_results_per_query: usize  // défaut: 10
}
```

### Audit (`history.rs`)

```jsonl
{
  "ts": 1710000000,
  "peer_id": "aabb...",
  "peer_name": "Thomas",
  "query": "budget Q1",
  "results_count": 5,
  "blocked": false
}
```

---

## Les 25 tools MCP (`src/commands/mcp.rs`)

### Recherche sémantique

| Tool            | Paramètres    | Description                                     |
| --------------- | ------------- | ----------------------------------------------- |
| `search_memory` | query, limit? | Vectoriel ONNX blended (global + emails forcés) |

### Recherche par source (keyword exact)

| Tool                  | Paramètres      | Source                          |
| --------------------- | --------------- | ------------------------------- |
| `search_emails`       | keyword, limit? | email                           |
| `get_emails_by_date`  | query?, limit?  | email par période               |
| `read_email`          | id              | email (contenu complet par URL) |
| `search_messages`     | keyword, limit? | imessage                        |
| `search_notes`        | keyword, limit? | notes                           |
| `search_terminal`     | keyword, limit? | terminal                        |
| `search_calendar`     | keyword, limit? | calendar                        |
| `get_upcoming_events` | limit?          | calendar (prochains événements) |
| `search_notion`       | keyword, limit? | notion                          |
| `search_github`       | keyword, limit? | github                          |
| `search_linear`       | keyword, limit? | linear                          |
| `search_jira`         | keyword, limit? | jira                            |
| `search_slack`        | keyword, limit? | slack                           |
| `search_trello`       | keyword, limit? | trello                          |
| `search_todoist`      | keyword, limit? | todoist                         |
| `search_gitlab`       | keyword, limit? | gitlab                          |
| `search_airtable`     | keyword, limit? | airtable                        |
| `search_obsidian`     | keyword, limit? | obsidian                        |

### Filesystem

| Tool               | Paramètres                                   | Description                          |
| ------------------ | -------------------------------------------- | ------------------------------------ |
| `find_file`        | name, limit?                                 | recherche par nom/chemin             |
| `fetch_content`    | path, query?, block_index?, offset?, length? | lire fichier + RAG scoring optionnel |
| `get_recent_files` | hours?                                       | fichiers récemment modifiés          |
| `list_directory`   | path                                         | lister un répertoire                 |
| `index_files`      | path                                         | déclencher l'indexation              |

### Admin

| Tool         | Paramètres | Description       |
| ------------ | ---------- | ----------------- |
| `get_status` | —          | counts par source |

### Workflow action (approbation obligatoire)

Tout tool préfixé `act_` crée une `ActionRequest` → visible dans le dashboard → l'utilisateur approuve/rejette → l'executor exécute.

---

## Dashboard web (5 pages)

Interface React embarquée dans le binaire (`include_dir!` macro dans `server.rs`).

| Page              | Route      | Description                                        |
| ----------------- | ---------- | -------------------------------------------------- |
| **Statut**        | `/`        | Counts par source, métriques disk/RAM/vecteurs     |
| **Recherche**     | `/search`  | Recherche multi-source + filtres date + scope peer |
| **Récents**       | `/recent`  | Documents récents par source, filtres              |
| **Configuration** | `/config`  | Configuration des connecteurs cloud                |
| **Actions**       | `/actions` | File d'approbation (SSE temps réel)                |
| **Réseau**        | `/network` | Peers P2P, invitations, permissions, historique    |

**Règles d'affichage des sources :**

- Sources locales (10) : toujours présentes (Chrome, Safari, Email, iMessage, Notes, Calendar, Terminal, Fichiers, Contacts, Arc)
- Sources cloud (10) : présentes seulement si le `.toml` de config existe
- Sources P2P : affichées uniquement si peers connectés

**Recherche avancée :**

- Multi-terme AND : `qonto + style`
- Filtre de périmètre : [Moi seulement | Tout le réseau | peer_name...]

---

## Variables d'environnement

```bash
# OBLIGATOIRE pour osmozzz mcp (le client IA doit l'avoir dans son env)
ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib

# Optionnel (fallback sur gmail.toml)
OSMOZZZ_GMAIL_USER="user@gmail.com"
OSMOZZZ_GMAIL_PASSWORD="app password"
```

**Exemple de configuration (Claude Desktop)** (`~/Library/Application Support/Claude/claude_desktop_config.json`) :

```json
{
  "mcpServers": {
    "osmozzz": {
      "command": "/Users/VOTRE_USER/.cargo/bin/osmozzz",
      "args": ["mcp"],
      "env": {
        "ORT_DYLIB_PATH": "/opt/homebrew/lib/libonnxruntime.dylib"
      }
    }
  }
}
```

---

## Build & Deploy

### Prérequis

```bash
brew install onnxruntime
export ORT_DYLIB_PATH=$(brew --prefix onnxruntime)/lib/libonnxruntime.dylib
./scripts/download-model.sh   # télécharge ~90MB
```

### Release publique (GitHub Actions) — NE JAMAIS modifier le workflow existant

**Fichier** : `.github/workflows/release.yml`

Le workflow se déclenche sur un tag `v*`. Il fait tout dans l'ordre :

1. `brew install onnxruntime protobuf`
2. Télécharge les modèles ONNX depuis HuggingFace directement (pas de script)
3. `npm ci && npm run build` (dashboard)
4. `touch crates/osmozzz-api/src/server.rs` + `cargo build --release -p osmozzz-cli`
5. Construit le payload `.pkg` à la main (`pkg-root/`) avec `pkgbuild`
6. Publie `osmozzz.pkg` sur GitHub Releases via `softprops/action-gh-release@v2`

**Pour publier une release :**

```bash
git tag v1.2.3
git push --tags
```

**Règles strictes :**

- NE PAS créer de `scripts/release.sh` — le build PKG est inline dans le workflow
- NE PAS modifier la structure `pkg-root/` — elle correspond au `postinstall`
- `ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib` — chemin fixe sur les runners GitHub macOS
- Le `.pkg` s'installe dans `/usr/local/bin/osmozzz` + `/usr/local/lib/libonnxruntime.dylib` + `/Library/OSMOzzz/models/`

---

### Build rapide (développement) — TOUJOURS utiliser

```bash
./build.sh       # smart build : npm si dashboard changé + cargo incremental
osmozzz daemon
```

`build.sh` fait :

1. `npm run build` **seulement si** `dashboard/src/` a changé depuis le dernier build
2. `touch crates/osmozzz-api/src/server.rs` (force recompilation include_dir!)
3. `cargo build --release -p osmozzz-cli` (incremental, ~30s si peu de changements)
4. `cp target/release/osmozzz ~/.cargo/bin/osmozzz`
5. Copie des modèles ONNX vers `~/.osmozzz/models/`

### Build complet (première fois ou changement de deps)

```bash
cd dashboard && npm run build
touch crates/osmozzz-api/src/server.rs
cargo install --path crates/osmozzz-cli --locked
osmozzz daemon
```

**Important** :

- Frontend changé → `npm run build` + `touch server.rs` requis
- Rust seul changé → `./build.sh` suffit (skip npm automatiquement)
- `cargo build` ≠ `cargo install` : le daemon utilise `~/.cargo/bin/osmozzz` (install)
- Après install : Ctrl+C daemon → `osmozzz daemon` → Cmd+Shift+R navigateur

---

## Fichiers de configuration (`~/.osmozzz/`)

```
config.toml          ← config CLI/daemon (optionnel)
privacy.toml         ← filtres confidentialité (credit_card, iban, api_keys...)
blacklist.toml       ← documents bannis par URL/source/identifiant
aliases.toml         ← pseudonymisation (vrai nom → alias vu par le client IA)
audit.jsonl          ← journal d'accès MCP (tool, query, results, blocked) append-only
identity.toml        ← identité P2P Ed25519 (auto-créé)
peers.toml           ← peers P2P connus + permissions
query_history.jsonl  ← audit des requêtes P2P reçues (append-only)
gmail.toml           ← config Gmail IMAP
notion.toml          ← token Notion
github.toml          ← token + repos GitHub
linear.toml          ← api_key Linear
jira.toml            ← base_url + email + token Jira
slack.toml           ← token + team_id + channels Slack
trello.toml          ← api_key + token Trello
todoist.toml         ← token Todoist
gitlab.toml          ← token + base_url + groups GitLab
airtable.toml        ← token + bases Airtable
obsidian.toml        ← vault_path Obsidian
vault/               ← LanceDB (fichiers parquet)
models/              ← all-MiniLM-L6-v2.onnx + tokenizer.json
```

---

## Règles globales

0. **INTERDIT : commandes git sans demande explicite** — ne jamais exécuter `git add`, `git commit`, `git push`, `git reset`, ni aucune commande git destructive ou de publication sans que l'utilisateur le demande explicitement. Modifier des fichiers est autorisé, les commiter/pousser ne l'est JAMAIS de façon autonome.

1. **Jamais de données hors du Mac** — tout est local, les peers P2P ne reçoivent que des résultats filtrés
2. **Philosophie "moins c'est plus"** — pas de sur-ingénierie
3. **Build toujours dans l'ordre** : `npm run build` → `touch server.rs` → `cargo install`
4. **Pas de breaking changes** sur le schéma LanceDB sans migration
5. **Un sous-agent par domaine** — ne pas mélanger les responsabilités
6. **Configuration utilisateur** : uniquement via le dashboard, jamais via toml manuels
7. **ORT_DYLIB_PATH** : toujours injecter dans les configs du client IA (load-dynamic)
8. **Privacy filter** : s'applique TOUJOURS avant envoi à un peer P2P
9. **Permissions P2P** : rechargées à chaque requête (révocables instantanément)
10. **Audit** : historique append-only, jamais modifié

---

## Pattern d'ajout d'un nouveau harvester

1. Créer `crates/osmozzz-harvester/src/nom_source.rs` (impl trait `Harvester`)
2. Exporter depuis `crates/osmozzz-harvester/src/lib.rs`
3. Ajouter variante dans `SourceType` (osmozzz-core)
4. Ajouter dans `osmozzz-cli/src/commands/index.rs` (CLI)
5. Ajouter dans `osmozzz-cli/src/commands/daemon.rs` (auto-sync interval)
6. Ajouter MCP tool dans `osmozzz-cli/src/commands/mcp.rs`
7. Ajouter dans `osmozzz-api/src/routes.rs` (get_status + get_config)
8. Ajouter config POST route si cloud connector
9. Ajouter card dans dashboard (StatusPage, RecentPage, ConfigPage si cloud)
10. Build : `npm run build` → `touch server.rs` → `cargo install`

---

## Sous-agents disponibles

| Agent           | CLAUDE.md                            | Domaine                        |
| --------------- | ------------------------------------ | ------------------------------ |
| Harvester Agent | `crates/osmozzz-harvester/CLAUDE.md` | Nouvelles sources de données   |
| MCP Tools Agent | `crates/osmozzz-cli/CLAUDE.md`       | Interface IA (tools MCP)       |
| Storage Agent   | `crates/osmozzz-embedder/CLAUDE.md`  | LanceDB, recherche, embeddings |

---

## Roadmap

### Fait ✅

- 20 harvesters (locaux + cloud + contacts + arc)
- 25 tools MCP (search, filesystem, get_status, get_upcoming_events)
- Dashboard 5 pages (Statut, Recherche, Récents, Config, Actions, Réseau)
- REST API complète (Axum) avec toutes routes
- Daemon auto-sync par source avec intervals dédiés
- Blacklist / ban documents & sources (URL, expéditeur, domaine, chemin)
- Alias d'identité / pseudonymisation (vrai nom → alias vu par le client IA)
- Journal d'accès MCP (audit.jsonl, onglet "Journal d'accès" dashboard)
- Compact LanceDB
- Système d'actions avec workflow approbation (ActionQueue + Executor)
- 16 types d'actions exécutables (email, Slack, Notion, Linear, Todoist, GitHub, Trello, GitLab, iMessage, Calendar, Notes, Fichiers, Shell)
- Filtre de confidentialité (credit_card, iban, api_keys, email, phone)
- FSEvents watcher (Desktop + Documents)
- P2P mesh iroh (QUIC + relay + Ed25519 + permissions granulaires + audit)
- Proof of Context (HMAC-SHA256 `osmozzz verify`)
- Support PDF (extraction + chunking)
- Contacts macOS harvester
- Arc browser harvester
- Recherche multi-terme AND (`+`)
