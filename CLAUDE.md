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

## ⚠️ ÉTAT ACTUEL DU CODE — DÉSACTIVATIONS TEMPORAIRES INTENTIONNELLES

Ces éléments sont **volontairement désactivés** par le développeur. Ne jamais tenter de les réactiver ou de les "corriger" sans demande explicite.

### 1. LanceDB / ONNX — DÉSACTIVÉ (osmozzz-embedder est un stub)

`osmozzz-embedder` ne fait **rien** en ce moment :
- `vault.upsert()` → no-op
- `vault.search()` → retourne toujours `Ok(vec![])`
- `vault.count_source()` → retourne toujours `Ok(0)`

→ **Conséquence :** `GET /api/search`, `GET /api/recent`, et tous les tools MCP de type `search_*` qui passent par le vault retournent des résultats vides. C'est intentionnel.

### 2. Harvesters cloud — DÉSACTIVÉS (code mort temporaire)

Ces 10 harvesters existent dans `osmozzz-harvester/src/` mais **ne sont pas déclarés dans `lib.rs`** → inaccessibles :
`airtable.rs`, `github.rs`, `gitlab.rs`, `jira.rs`, `linear.rs`, `notion.rs`, `obsidian.rs`, `slack.rs`, `todoist.rs`, `trello.rs`

→ Ne pas les rajouter dans `lib.rs` sans demande explicite.

### 3. Harvesters locaux macOS — DÉSACTIVÉS

Ces harvesters existent et sont exportés mais **ne sont plus appelés** :
- `ArcHarvester` (arc.rs)
- `ContactsHarvester` (contacts.rs)

Les harvesters macOS suivants (iMessage, Calendar, Notes, Safari) sont compilés mais **les tools MCP correspondants (`search_messages`, `search_notes`, `search_calendar`, `search_arc`) retournent vide** car ils passent par le vault stub.

→ **Le daemon ne lance AUCUN harvester en boucle.** Il est uniquement serveur REST + P2P.

### 4. Ce qui fonctionne RÉELLEMENT aujourd'hui

| Fonctionnel ✅ | Désactivé ❌ |
|---|---|
| 27 connecteurs natifs MCP (~600 tools) | Recherche sémantique (LanceDB stub) |
| Dashboard 5 pages | Indexation locale (vault no-op) |
| P2P mesh (Iroh + permissions + SSE) | Harvesters cloud (pas dans lib.rs) |
| ActionQueue + approbation dashboard | iMessage/Calendar/Notes search |
| Gmail SMTP (envoi) | Arc, Contacts harvesters |
| Filesystem (find_file, fetch_content, list_directory) | osmozzz-bridge (20K, rôle flou) |
| Audit log, privacy filter | splitter, watcher modules |

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
osmozzz-harvester → toutes les sources de données (21 harvesters + FSEvents watcher)
osmozzz-embedder  → ONNX + LanceDB (OnnxEmbedder, VectorStore, Vault, Blacklist)
osmozzz-bridge    → serveur UDS legacy (stdin/stdout bridge, peu utilisé)
osmozzz-api       → REST API + dashboard (Axum) + ActionQueue + Executor
osmozzz-cli       → CLI (clap) + serveur MCP + daemon + MCP proxies + connectors natifs
osmozzz-p2p       → réseau P2P mesh (iroh QUIC, identité Ed25519, permissions, audit)
```

---

## Architecture Triple — Harvesters + MCP Proxies + Connectors Natifs (FONDAMENTAL)

**OSMOzzz est TOUJOURS l'intermédiaire unique entre Claude et toutes les sources de données.**
Claude ne parle jamais directement à Notion, Jira, Supabase, etc. Tout passe par OSMOzzz.

Il existe **trois systèmes complémentaires** dans OSMOzzz :

### 1. Harvesters Rust — Indexation locale

Les harvesters appellent les APIs cloud directement via `reqwest`, transforment les données en `Document` et les indexent dans LanceDB (ONNX).

```
API cloud → harvester Rust (reqwest) → Vec<Document> → Vault (LanceDB) → search_* tools
```

Utilisés pour : **recherche sémantique** dans les données passées.

### 2. MCP Proxies — Subprocesses MCP tiers

Pour les **actions complètes via packages npm officiels**, OSMOzzz lance des **subprocesses MCP via `bunx`** (Bun). Ces subprocesses sont des packages npm MCP officiels, proxifiés par OSMOzzz en JSON-RPC stdin/stdout.

```
Claude → OSMOzzz MCP (Rust) → subprocess bunx @pkg/mcp-server → API cloud
```

**Fichiers** : `crates/osmozzz-cli/src/mcp_proxy/`

| Fichier         | Package npm                           | Config                     |
| --------------- | ------------------------------------- | -------------------------- |
| `github.rs`     | `@modelcontextprotocol/server-github` | `~/.osmozzz/github.toml`   |
| `notion.rs`     | `@notionhq/notion-mcp-server`         | `~/.osmozzz/notion.toml`   |
| `slack.rs`      | `@modelcontextprotocol/server-slack`  | `~/.osmozzz/slack.toml`    |
| `supabase.rs`   | `@supabase/mcp-server-supabase`       | `~/.osmozzz/supabase.toml` |
| `gitlab.rs`     | `@zereight/mcp-gitlab`                | `~/.osmozzz/gitlab.toml`   |
| `sentry.rs`     | `@sentry/mcp-server`                  | `~/.osmozzz/sentry.toml`   |
| `cloudflare.rs` | `@cloudflare/mcp-server-cloudflare`   | `~/.osmozzz/cloudflare.toml` |

**Mécanisme** (`McpSubprocess`) :
- Vérifie/installe Bun automatiquement (`~/.bun/bin/bun`)
- Lance `bunx x --bun <package>` avec les env vars du `.toml`
- Handshake JSON-RPC 2.0 (`initialize` + `notifications/initialized`)
- Découverte automatique des tools (`tools/list`)
- Proxifie les appels de Claude vers le subprocess (`tools/call`)
- Si le `.toml` est absent → subprocess non démarré silencieusement

### 3. Connectors Natifs Rust — Actions temps réel

Pour les connecteurs implémentés **directement en Rust** (sans subprocess npm), OSMOzzz appelle les APIs REST directement. Les tools sont déclarés dans le crate `osmozzz-cli` et dispatchés depuis `mcp.rs`.

```
Claude → OSMOzzz MCP (Rust) → connector natif Rust (reqwest) → API cloud
```

**Fichiers** : `crates/osmozzz-cli/src/connectors/`

| Connecteur | Fichier       | Nb tools | Config                     |
| ---------- | ------------- | -------- | -------------------------- |
| Linear     | `linear.rs`   | 17       | `~/.osmozzz/linear.toml`   |
| Jira       | `jira.rs`     | 23       | `~/.osmozzz/jira.toml`     |
| GitLab     | `gitlab.rs`   | 23       | `~/.osmozzz/gitlab.toml`   |
| Vercel     | `vercel.rs`   | 15       | `~/.osmozzz/vercel.toml`   |
| Railway    | `railway.rs`  | 14       | `~/.osmozzz/railway.toml`  |
| Render     | `render.rs`   | 14       | `~/.osmozzz/render.toml`   |
| Google Cal | `gcal.rs`     | 12       | `~/.osmozzz/google.toml`   |
| Stripe     | `stripe.rs`   | 27       | `~/.osmozzz/stripe.toml`   |
| HubSpot    | `hubspot.rs`  | 26       | `~/.osmozzz/hubspot.toml`  |
| PostHog    | `posthog.rs`  | 18       | `~/.osmozzz/posthog.toml`  |
| Resend     | `resend.rs`   | 14       | `~/.osmozzz/resend.toml`   |
| Twilio     | `twilio.rs`   | 16       | `~/.osmozzz/twilio.toml`   |
| Figma      | `figma.rs`    | 15       | `~/.osmozzz/figma.toml`    |
| Discord    | `discord.rs`  | 28       | `~/.osmozzz/discord.toml`  |

**Total : 262 tools natifs** répartis sur 14 connecteurs. Dispatch dans `mcp.rs` via le pattern `tool_name.starts_with("stripe_")`, etc.

### Tableau comparatif des 3 architectures

|                | Harvester (Rust)        | MCP Proxy (Subprocess)             | Connector Natif (Rust)      |
| -------------- | ----------------------- | ---------------------------------- | --------------------------- |
| **Rôle**       | Indexation (passé)      | Actions via package npm officiel   | Actions temps réel          |
| **Transport**  | reqwest HTTP            | bunx subprocess JSON-RPC           | reqwest HTTP direct         |
| **Output**     | Vec<Document> → LanceDB | Réponse JSON → Claude              | Réponse JSON → Claude       |
| **Tools**      | `search_*` dans mcp.rs  | Tools natifs du package npm        | `xxx_*` déclarés en Rust    |
| **Dépendance** | Cargo (reqwest)         | Bun runtime (auto-installé)        | Cargo (reqwest)             |
| **Config**     | `~/.osmozzz/*.toml`     | `~/.osmozzz/*.toml`                | `~/.osmozzz/*.toml`         |
| **Fichiers**   | `osmozzz-harvester/src/`| `osmozzz-cli/src/mcp_proxy/`       | `osmozzz-cli/src/connectors/`|

### ⚠️ CHECKLIST COMPLÈTE — Ajouter un nouveau Connector Natif

**8 endroits à modifier. Oublier l'un = bug silencieux (connecteur invisible en P2P ou permissions cassées).**

#### Rust (5 fichiers)

1. **Créer** `crates/osmozzz-cli/src/connectors/nom.rs` :
   - `pub fn tools() -> Vec<Value>` — déclare les tools JSON Schema
   - `pub async fn handle(tool: &str, args: &Value) -> Result<String, String>` — dispatch + appels API
   - `pub fn load_config() -> Option<NomConfig>` — lit `~/.osmozzz/nom.toml`

2. **`connectors/mod.rs`** — 3 endroits dans ce fichier :
   - `pub mod nom;` — déclaration du module
   - `tools.extend(nom::tools());` dans `all_tools()`
   - `if name.starts_with("nom_") { return Some(nom::handle(name, args).await); }` dans `handle()`

3. **`mcp.rs`** — 2 endroits dans ce fichier :
   - `if name.starts_with("nom_") { return Some("nom"); }` dans `tool_source()`
   - `tools.extend(connectors::all_tools());` est déjà global — rien à faire si `all_tools()` est mis à jour

4. **`routes.rs`** — 2 endroits dans ce fichier :
   - Ajouter `("nom.toml", "nom")` dans le mapping de `get_configured_connectors()`
   - Ajouter la fonction `pub async fn post_config_nom(...)` (validation + écriture `nom.toml`)

5. **`server.rs`** — 1 ligne :
   - `.route("/config/nom", post(routes::post_config_nom))`

#### P2P (1 fichier)

6. **`crates/osmozzz-p2p/src/node.rs`** — 1 ligne :
   - Ajouter `"nom"` dans `ALL_CONNECTORS` de `build_tool_sync_map()`
   - ⚠️ Sans ça, le connecteur n'apparaît JAMAIS dans la card permissions des peers

#### Dashboard (2 fichiers)

7. **`dashboard/src/pages/NetworkPage.tsx`** — 1 ligne :
   - Ajouter `{ id: 'nom', label: 'Nom' }` dans `TOOL_LABELS`
   - ⚠️ Sans ça, le connecteur s'affiche comme son ID brut dans la card P2P

8. **`dashboard/src/pages/ConfigPage.tsx`** — 1 entrée :
   - Ajouter `{ id: 'nom', name: 'Nom', desc: 'Description courte' }` dans `CONNECTORS`
   - Ajouter le composant form `NomForm` si besoin d'une UI de configuration

---

### Pattern pour ajouter un nouveau MCP Proxy

1. Créer `crates/osmozzz-cli/src/mcp_proxy/nom.rs` (charger config TOML + appeler `LazyProxy::new()`)
2. Déclarer `pub mod nom;` dans `mod.rs`
3. Ajouter `if let Some(p) = nom::lazy() { proxies.push(p); }` dans `start_all_proxies()`
4. Ajouter `("nom.toml", "nom")` dans `get_configured_connectors()` de `routes.rs`
5. Ajouter `.route("/config/nom", post(routes::post_config_nom))` dans `server.rs`
6. Ajouter `"nom"` dans `ALL_CONNECTORS` de `node.rs`
7. Ajouter `{ id: 'nom', label: 'Nom' }` dans `TOOL_LABELS` de `NetworkPage.tsx`
8. Ajouter card dans `ConfigPage.tsx`

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
api_keys    = true   # masque les clés API/tokens
email       = false  # masque les adresses email
phone       = false  # masque les numéros de téléphone
```

---

## osmozzz-harvester — Sources de données (21 harvesters)

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
- Répertoires : `~/Desktop`, `~/Documents`, `~/Downloads`
- Extensions texte (8) : `md, mdx, txt, rst, org, tex, adoc, csv`
- PDF extrait et chunké si < 5 MB
- **Chunking** : `MAX_CHARS=1600`, `OVERLAP_CHARS=160`, `MAX_TEXT_BYTES=2MB`
- Dirs ignorés (24) : `node_modules, .git, target, __pycache__, .cargo, dist, build, .next, .nuxt, vendor, .build, Pods, DerivedData, .gradle, .idea, venv, .venv, env, .tox, .osmozzz, Library, Temp, obj, Logs`

**`watcher.rs` — FSEvents monitor** :
- `DEBOUNCE_MS=500`, `STARTUP_GRACE_SECS=15`
- Scrute : `~/Desktop`, `~/Documents`

### Sources cloud (nécessitent `~/.osmozzz/*.toml`)

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

### Modèle d'embedding
- **Modèle** : all-MiniLM-L6-v2 (SentenceTransformers)
- **Dimension** : 384 float32 · **Normalisation** : L2 · **Distance** : cosine → score `1.0 - dist/2.0`
- **Chemins** : `~/.osmozzz/models/all-MiniLM-L6-v2.onnx` + `~/.osmozzz/models/tokenizer.json`

---

## osmozzz-api — REST API & Dashboard

### ActionQueue (`src/action_queue.rs`)
- `push(action)` → ajoute + notifie SSE
- `pending()` → filtre Pending, marque Expired si > 5min
- `approve(id)` / `reject(id)` → statut + SSE
- `subscribe()` → `broadcast::Receiver<ActionEvent>`

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
GET  /api/status
GET  /api/search?q=...&source=...&from=...&to=...
GET  /api/recent?source=...&q=...&from=...&to=...&limit=200&offset=0
GET  /api/config
```

#### Configuration des connecteurs (26 routes)
```
POST /api/config/gmail        POST /api/config/notion       POST /api/config/github
POST /api/config/linear       POST /api/config/jira         POST /api/config/slack
POST /api/config/trello       POST /api/config/todoist      POST /api/config/gitlab
POST /api/config/airtable     POST /api/config/cloudflare   POST /api/config/sentry
POST /api/config/obsidian     POST /api/config/supabase     POST /api/config/vercel
POST /api/config/railway      POST /api/config/render       POST /api/config/google
POST /api/config/stripe       POST /api/config/hubspot      POST /api/config/posthog
POST /api/config/resend       POST /api/config/discord      POST /api/config/twilio
POST /api/config/figma
```

#### Documents & Blacklist
```
GET  /api/open?url=...
GET  /api/messages/contacts
GET  /api/messages/conversation?contact=...
POST /api/ban · POST /api/unban · GET /api/blacklist
POST /api/compact
POST /api/reindex/imessage
GET  /api/files/search?q=...
GET  /api/index/preview · GET /api/index/progress · POST /api/index
GET  /api/privacy · POST /api/privacy
GET  /api/aliases · POST /api/aliases
```

#### Actions (workflow approbation)
```
GET  /api/actions             GET  /api/actions/pending
GET  /api/actions/stream      GET  /api/actions/:id
POST /api/actions/:id/approve POST /api/actions/:id/reject
GET  /api/permissions         POST /api/permissions
GET  /api/source-access       POST /api/source-access
GET  /api/audit
```

#### Réseau P2P
```
GET    /api/network/peers
POST   /api/network/invite
POST   /api/network/connect
DELETE /api/network/peers/:peer_id
GET    /api/network/permissions/:peer_id
POST   /api/network/permissions/:peer_id
GET    /api/network/history
```

---

## osmozzz-cli — Interface utilisateur

### Commandes CLI
```bash
osmozzz index   --source chrome|safari|gmail|imessage|notes|calendar|terminal|files|
                          notion|github|linear|jira|slack|trello|todoist|gitlab|airtable|obsidian|contacts|arc
                [--path PATH] [--reset] [--batch-size N]

osmozzz search  QUERY [--source FILTER] [--limit N] [--format text|json]
osmozzz status
osmozzz compact
osmozzz daemon
osmozzz mcp
osmozzz serve   [--socket PATH]
osmozzz install
osmozzz verify  --sig S --source S --url U --content C --ts T
```

### MCP Tools — Vue complète

**Fichier principal** : `crates/osmozzz-cli/src/commands/mcp.rs` (3132 lignes)

#### Tools natifs mcp.rs (31 tools)

**Recherche sémantique** :
- `search_memory` — vectoriel ONNX blended (global + emails forcés)

**Gmail** (7) :
`gmail_search`, `gmail_recent`, `gmail_read`, `gmail_by_sender`, `gmail_send`, `gmail_reply`, `gmail_stats`

**Sources locales** (7) :
`search_messages`, `search_notes`, `search_terminal`, `get_upcoming_events`, `search_calendar`, `search_contacts`, `search_arc`

**Filesystem** (4) :
`find_file`, `fetch_content`, `get_recent_files`, `list_directory`

**Actions (approbation dashboard)** (11) :
`act_create_todoist_task`, `act_create_trello_card`, `act_create_gitlab_issue`, `act_send_imessage`, `act_create_calendar_event`, `act_delete_calendar_event`, `act_delete_note`, `act_create_folder`, `act_rename_file`, `act_delete_file`, `act_run_command`

**Workflow** (1) :
`osmozzz_resume_action`

#### Connectors natifs Rust (262 tools — `connectors/`)

**Linear** (17) :
`linear_search_issues`, `linear_get_issue`, `linear_create_issue`, `linear_update_issue`, `linear_add_comment`, `linear_list_teams`, `linear_list_issues`, `linear_list_projects`, `linear_list_workflow_states`, `linear_list_labels`, `linear_list_members`, `linear_archive_issue`, `linear_get_viewer`, `linear_create_project`, `linear_list_cycles`, `linear_get_cycle`, `linear_delete_comment`

**Jira** (23) :
`jira_search_issues`, `jira_get_issue`, `jira_create_issue`, `jira_update_issue`, `jira_add_comment`, `jira_get_comments`, `jira_transition_issue`, `jira_list_transitions`, `jira_assign_issue`, `jira_list_projects`, `jira_get_issue_types`, `jira_list_priorities`, `jira_search_users`, `jira_add_worklog`, `jira_list_boards`, `jira_list_sprints`, `jira_delete_issue`, `jira_link_issues`, `jira_list_link_types`, `jira_get_current_user`, `jira_list_versions`, `jira_move_to_sprint`, `jira_get_fields`

**GitLab natif** (23) :
`gitlab_list_issues`, `gitlab_get_issue`, `gitlab_create_issue`, `gitlab_update_issue`, `gitlab_close_issue`, `gitlab_add_comment`, `gitlab_get_comments`, `gitlab_assign_issue`, `gitlab_list_labels`, `gitlab_list_mrs`, `gitlab_get_mr`, `gitlab_create_mr`, `gitlab_merge_mr`, `gitlab_add_mr_comment`, `gitlab_list_pipelines`, `gitlab_get_pipeline`, `gitlab_retry_pipeline`, `gitlab_cancel_pipeline`, `gitlab_list_projects`, `gitlab_get_project`, `gitlab_list_members`, `gitlab_get_current_user`, `gitlab_list_branches`

**Vercel** (15) :
`vercel_list_projects`, `vercel_get_project`, `vercel_list_deployments`, `vercel_get_deployment`, `vercel_list_domains`, `vercel_list_env`, `vercel_cancel_deployment`, `vercel_list_teams`, `vercel_check_alias`, `vercel_get_build_logs`, `vercel_redeploy`, `vercel_delete_project`, `vercel_add_domain_to_project`, `vercel_remove_domain_from_project`, `vercel_get_project_members`

**Railway** (14) :
`railway_list_projects`, `railway_get_project`, `railway_list_services`, `railway_list_deployments`, `railway_get_logs`, `railway_get_variables`, `railway_trigger_deploy`, `railway_list_environments`, `railway_get_service`, `railway_build_logs`, `railway_restart_deployment`, `railway_create_project`, `railway_delete_project`, `railway_get_usage`

**Render** (14) :
`render_list_services`, `render_get_service`, `render_list_deploys`, `render_get_deploy`, `render_trigger_deploy`, `render_list_env_vars`, `render_put_env_var`, `render_suspend_service`, `render_resume_service`, `render_get_logs`, `render_list_custom_domains`, `render_add_custom_domain`, `render_delete_custom_domain`, `render_scale_service`

**Google Calendar** (12) :
`gcal_upcoming`, `gcal_today`, `gcal_this_week`, `gcal_search`, `gcal_list_calendars`, `gcal_create_event`, `gcal_delete_event`, `gcal_update_event`, `gcal_get_event`, `gcal_get_free_busy`, `gcal_add_attendee`, `gcal_list_upcoming_for_calendar`

**Stripe** (27) :
`stripe_get_balance`, `stripe_list_customers`, `stripe_get_customer`, `stripe_create_customer`, `stripe_list_payment_intents`, `stripe_get_payment_intent`, `stripe_list_subscriptions`, `stripe_get_subscription`, `stripe_list_invoices`, `stripe_get_invoice`, `stripe_list_events`, `stripe_get_event`, `stripe_list_webhooks`, `stripe_get_webhook`, `stripe_create_webhook`, `stripe_delete_webhook`, `stripe_list_payouts`, `stripe_get_payout`, `stripe_search_customers`, `stripe_update_customer`, `stripe_list_products`, `stripe_create_product`, `stripe_list_prices`, `stripe_create_price`, `stripe_create_subscription`, `stripe_create_payment_link`, `stripe_create_checkout_session`

**HubSpot** (26) :
`hubspot_list_contacts`, `hubspot_get_contact`, `hubspot_create_contact`, `hubspot_update_contact`, `hubspot_search_contacts`, `hubspot_delete_contact`, `hubspot_list_companies`, `hubspot_get_company`, `hubspot_create_company`, `hubspot_update_company`, `hubspot_search_companies`, `hubspot_list_deals`, `hubspot_get_deal`, `hubspot_create_deal`, `hubspot_update_deal`, `hubspot_move_deal_stage`, `hubspot_search_deals`, `hubspot_list_tickets`, `hubspot_get_ticket`, `hubspot_create_ticket`, `hubspot_update_ticket`, `hubspot_create_note`, `hubspot_create_task`, `hubspot_log_call`, `hubspot_list_pipelines`, `hubspot_list_pipeline_stages`

**PostHog** (18) :
`posthog_capture_event`, `posthog_query_events`, `posthog_get_event_definitions`, `posthog_list_persons`, `posthog_get_person`, `posthog_search_persons`, `posthog_delete_person`, `posthog_list_feature_flags`, `posthog_get_feature_flag`, `posthog_create_feature_flag`, `posthog_update_feature_flag`, `posthog_toggle_feature_flag`, `posthog_list_insights`, `posthog_get_insight`, `posthog_create_trend_insight`, `posthog_list_cohorts`, `posthog_list_dashboards`, `posthog_list_projects`

**Resend** (14) :
`resend_send_email`, `resend_get_email`, `resend_cancel_email`, `resend_list_domains`, `resend_get_domain`, `resend_create_domain`, `resend_verify_domain`, `resend_delete_domain`, `resend_list_api_keys`, `resend_create_api_key`, `resend_delete_api_key`, `resend_list_audiences`, `resend_create_audience`, `resend_delete_audience`

**Twilio** (16) :
`twilio_send_sms`, `twilio_send_whatsapp`, `twilio_list_messages`, `twilio_get_message`, `twilio_create_call`, `twilio_list_calls`, `twilio_get_call`, `twilio_list_numbers`, `twilio_search_available_numbers`, `twilio_purchase_number`, `twilio_release_number`, `twilio_create_verify_service`, `twilio_list_verify_services`, `twilio_send_verification`, `twilio_check_verification`, `twilio_lookup_phone_number`

**Figma** (15) :
`figma_get_file`, `figma_get_file_nodes`, `figma_list_file_versions`, `figma_get_comments`, `figma_post_comment`, `figma_delete_comment`, `figma_get_team_components`, `figma_get_component`, `figma_get_component_sets`, `figma_get_team_projects`, `figma_get_project_files`, `figma_get_local_variables`, `figma_get_published_variables`, `figma_export_images`, `figma_list_webhooks`

**Discord** (28) :
`discord_send_message`, `discord_edit_message`, `discord_delete_message`, `discord_get_message`, `discord_list_messages`, `discord_list_channels`, `discord_get_channel`, `discord_create_channel`, `discord_edit_channel`, `discord_delete_channel`, `discord_list_members`, `discord_get_member`, `discord_kick_member`, `discord_list_roles`, `discord_create_role`, `discord_add_role_to_member`, `discord_remove_role_from_member`, `discord_list_webhooks`, `discord_create_webhook`, `discord_send_webhook_message`, `discord_create_thread`, `discord_list_active_threads`, `discord_get_guild`, `discord_get_onboarding`, `discord_update_onboarding`, `discord_get_welcome_screen`, `discord_update_welcome_screen`, `discord_get_member_verification`

#### MCP Proxies (via bunx subprocess)
GitHub (~40), Notion (~25), Slack (~50), Supabase (~38), Sentry (~21), Cloudflare (~89), GitLab proxy (~115)

**Total général : ~600+ tools MCP disponibles**

---

## Dashboard web (3 pages actives)

Interface React embarquée dans le binaire (`include_dir!` dans `server.rs`).

| Page              | Route      | Description                                          |
| ----------------- | ---------- | ---------------------------------------------------- |
| **Dashboard**     | `#status`  | Counts par source, métriques disk/RAM/vecteurs       |
| **Actions MCP**   | `#actions` | File d'approbation SSE temps réel                    |
| **Connecteurs**   | `#config`  | Configuration des 26 connecteurs (actifs + disponibles) |
| ~~Réseau~~        | commenté   | P2P peers, invitations, permissions (désactivé UI)   |

**Connecteurs affichés dans ConfigPage** :
Gmail, Notion, GitHub, Linear, Jira, GitLab, Supabase, Sentry, Cloudflare, Vercel, Railway, Render, Google Calendar, Stripe, HubSpot, PostHog, Resend, Discord, Twilio, Figma

---

## osmozzz-p2p — Réseau mesh

### Architecture
```
crates/osmozzz-p2p/src/
├── identity.rs    ← clé Ed25519 par machine (~/.osmozzz/identity.toml)
├── node.rs        ← nœud iroh (QUIC + relay) + gestion connexions
├── protocol.rs    ← messages JSON (Ping/Pong, Hello/Welcome, Search/SearchResult, Info)
├── permissions.rs ← contrôle d'accès par source par peer
├── store.rs       ← peers.toml (liste persistée + permissions)
└── history.rs     ← log JSONL (~/.osmozzz/query_history.jsonl)
```

- **Protocol** : iroh QUIC (UDP), ALPN `b"osmozzz/1"`, relay n0.computer
- **Invitation** : lien base64 = endpoint iroh encodé

---

## Sécurité & Confidentialité

### 1. Filtre de confidentialité (`~/.osmozzz/privacy.toml`)
```toml
api_keys    = true
email       = false
phone       = false
```

### 2. Alias d'identité (`~/.osmozzz/aliases.toml`)
Remplace vrais noms par alias avant envoi au client IA.

### 3. Blacklist (`~/.osmozzz/blacklist.toml`)
Exclut documents ou sources entières. Vérifié à l'indexation ET à la recherche.

### 4. Journal d'accès (`~/.osmozzz/audit.jsonl`)
Log append-only de tous les appels MCP.
```jsonl
{"ts":1710000000,"tool":"search_memory","query":"budget","results":13,"blocked":false}
```

---

## Fichiers de configuration (`~/.osmozzz/`)

```
config.toml          privacy.toml         blacklist.toml       aliases.toml
audit.jsonl          identity.toml        peers.toml           query_history.jsonl

# Harvesters cloud
gmail.toml           notion.toml          github.toml          linear.toml
jira.toml            slack.toml           trello.toml          todoist.toml
gitlab.toml          airtable.toml        obsidian.toml

# MCP Proxies
cloudflare.toml      sentry.toml          supabase.toml

# Connectors natifs
vercel.toml          railway.toml         render.toml          google.toml
stripe.toml          hubspot.toml         posthog.toml         resend.toml
discord.toml         twilio.toml          figma.toml

vault/               ← LanceDB (fichiers parquet)
models/              ← all-MiniLM-L6-v2.onnx + tokenizer.json
```

### Formats des fichiers de config cloud

```toml
# gmail.toml
username = "user@gmail.com"
app_password = "xxxx xxxx xxxx xxxx"

# notion.toml
token = "secret_xxx"

# github.toml
token = "ghp_xxx"
repos = ["owner/repo1"]

# linear.toml
api_key = "lin_api_xxx"

# jira.toml
base_url = "https://monentreprise.atlassian.net"
email    = "user@company.com"
token    = "ATATT3xxx"

# slack.toml
token    = "xoxp-xxx"
team_id  = "TXXXXXXXX"
channels = ["general"]

# gitlab.toml
token    = "glpat-xxx"
base_url = "https://gitlab.com"
groups   = ["my-group"]

# vercel.toml
token   = "xxx"
team_id = "xxx"   # optionnel

# railway.toml
token = "xxx"

# render.toml
api_key = "rnd_xxx"

# google.toml (Google Calendar CalDAV)
email         = "user@gmail.com"
app_password  = "xxxx xxxx xxxx xxxx"

# stripe.toml
secret_key = "sk_live_xxx"   # ou sk_test_xxx

# hubspot.toml
access_token = "pat-xxx"

# posthog.toml
api_key     = "phx_xxx"
project_id  = "12345"
host        = "https://app.posthog.com"   # optionnel

# resend.toml
api_key = "re_xxx"

# discord.toml
bot_token = "xxx"
guild_id  = "123456789"

# twilio.toml
account_sid  = "ACxxx"
auth_token   = "xxx"
from_number  = "+1234567890"

# figma.toml
access_token = "figd_xxx"
team_id      = "xxx"   # optionnel

# cloudflare.toml
api_token  = "xxx"
account_id = "xxx"

# sentry.toml
token = "sntrys_xxx"
host  = "https://sentry.io"   # optionnel

# supabase.toml
access_token = "sbp_xxx"
project_id   = "xxx"   # optionnel
```

---

## Build & Deploy

### Build rapide (développement) — TOUJOURS utiliser
```bash
./build.sh
~/.cargo/bin/osmozzz daemon
```

### Release publique (GitHub Actions)
**Fichier** : `.github/workflows/release.yml` — déclenché sur tag `v*`
```bash
git tag v1.2.3
git push --tags
```
**NE PAS** modifier le workflow. Le `.pkg` s'installe dans `/usr/local/bin/osmozzz`.

### Variables d'environnement
```bash
ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib   # OBLIGATOIRE pour osmozzz mcp
```

**Config Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`) :
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

## Règles globales

0. **INTERDIT : commandes git sans demande explicite** — ne jamais exécuter `git add`, `git commit`, `git push`, `git reset`, ni aucune commande git destructive ou de publication sans que l'utilisateur le demande explicitement.

1. **Jamais de données hors du Mac** — tout est local, les peers P2P ne reçoivent que des résultats filtrés
2. **Philosophie "moins c'est plus"** — pas de sur-ingénierie
3. **Build toujours dans l'ordre** : `npm run build` → `touch server.rs` → `cargo build --release` → `cp`
4. **Pas de breaking changes** sur le schéma LanceDB sans migration
5. **Un sous-agent par domaine** — ne pas mélanger les responsabilités
6. **Configuration utilisateur** : uniquement via le dashboard
7. **ORT_DYLIB_PATH** : toujours injecter dans les configs du client IA
8. **Privacy filter** : s'applique TOUJOURS avant envoi à un peer P2P
9. **Permissions P2P** : rechargées à chaque requête
10. **Audit** : historique append-only, jamais modifié

---

## Pattern d'ajout d'un nouveau harvester

1. Créer `crates/osmozzz-harvester/src/nom_source.rs` (impl trait `Harvester`)
2. Exporter depuis `lib.rs`
3. Ajouter variante dans `SourceType` (osmozzz-core)
4. Ajouter dans `osmozzz-cli/src/commands/index.rs`
5. Ajouter dans `osmozzz-cli/src/commands/daemon.rs` (auto-sync interval)
6. Ajouter MCP tool `search_nom` dans `mcp.rs`
7. Ajouter dans `routes.rs` (get_status + get_config)
8. Ajouter config POST route si cloud connector
9. Ajouter card dans dashboard (ConfigPage)

---

## Roadmap

### Fait ✅

- 21 harvesters (locaux + cloud)
- 7 MCP Proxies (GitHub, Notion, Slack, Supabase, GitLab, Sentry, Cloudflare)
- 14 Connectors natifs Rust (Linear, Jira, GitLab, Vercel, Railway, Render, GCal, Stripe, HubSpot, PostHog, Resend, Twilio, Figma, Discord) — 262 tools
- 31 tools MCP natifs (search, gmail, filesystem, actions, workflow)
- ~600+ tools MCP au total
- Dashboard 3 pages (Dashboard, Actions MCP, Connecteurs)
- REST API complète (Axum) avec 26 routes config
- Daemon auto-sync par source
- Blacklist / ban documents & sources
- Alias d'identité / pseudonymisation
- Journal d'accès MCP (audit.jsonl)
- Compact LanceDB
- Système d'actions avec workflow approbation (ActionQueue + Executor + 16 actions)
- Filtre de confidentialité
- FSEvents watcher
- P2P mesh iroh (QUIC + relay + Ed25519 + permissions + audit)
- Proof of Context (HMAC-SHA256)
- Support PDF (extraction + chunking)

### À faire 🔲

#### Nouveaux connecteurs à implémenter

| Connecteur | Type | Notes |
|---|---|---|
| **Airtable** | Connector natif | Déjà harvester — ajouter actions complètes |
| **Base44** | Connector natif | Plateforme no-code FR |
| **Calendly** | Connector natif | Rendez-vous & planning |
| **Canva** | Connector natif | Design — REST API officielle |
| **ClickUp** | Connector natif | Gestion de projet |
| **Cloudinary** | Connector natif | Gestion médias & images |
| **Postman** | MCP Proxy | API testing & collections |
| **Slack** | Connector natif | Déjà harvester — ajouter actions (send, create channel…) |
| **WordPress** | Connector natif | REST API WP — posts, pages, médias |
| **n8n** | Connector natif | Automation workflows — déclencher des flows |
| **Contrôle Chrome** | Connecteur natif macOS | CDP (Chrome DevTools Protocol) — onglets, navigation, screenshots |
| **Contrôle Mac** | Connecteur natif macOS | AppleScript / Accessibility API — apps, fenêtres, clavier |
| **Notes (R/W)** | Connecteur natif macOS | Déjà en lecture — ajouter création/modification via AppleScript |
| **WhatsApp Business** | Connector natif | API Cloud officielle Meta |
| **Shopify** | Connector natif | E-commerce — commandes, produits, clients |
| **Salesforce** | Connector natif | CRM enterprise — contacts, opportunités, leads |
