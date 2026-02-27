# OSMOzzz — Chef de Projet

## Vision

OSMOzzz est le hub de données local pour les IAs externes (Claude, etc.).
Objectif : donner à Claude un accès complet à toutes les données personnelles locales
(emails, fichiers, messages, historique, notes, calendrier) sans jamais envoyer de données
à l'extérieur. Tout tourne sur le Mac de l'utilisateur.

## Stack technique

- Rust 2021 · LanceDB 0.14 · ONNX Runtime (ort v2) · all-MiniLM-L6-v2 (384d)
- Transport MCP : stdin/stdout JSON-RPC 2.0
- DB : `~/.osmozzz/vault/` · Socket UDS : `~/.osmozzz/osmozzz.sock`

## Architecture des crates

```
osmozzz-core      → types partagés (Document, SearchResult, traits Harvester/Embedder)
osmozzz-harvester → sources de données (Chrome, Files, Gmail, iMessage, Notes, Calendar...)
osmozzz-embedder  → ONNX + LanceDB (OnnxEmbedder, VectorStore, Vault)
osmozzz-bridge    → serveur UDS
osmozzz-cli       → CLI + serveur MCP
```

## Roadmap actuelle

### Niveau 1 — Nouvelles sources (EN COURS)

- [ ] iMessage (~/Library/Messages/chat.db)
- [ ] Safari history (~/Library/Safari/History.db)
- [ ] Apple Notes (SQLite local)
- [ ] Apple Calendar (SQLite local)
- [ ] Terminal history (~/.zsh_history)

### Niveau 2 — Meilleure recherche

- [ ] Recherche hybride BM25 + vecteurs

### Niveau 3 — Visibilité

- [ ] Dashboard web local
- [ ] REST API

## Règles globales

1. **Jamais de données hors du Mac** — tout est local
2. **Philosophie "moins c'est plus"** — pas de sur-ingénierie
3. **Build always release** : `cargo build --release` (Claude Desktop utilise target/release/)
4. **Toujours rebuilder en release** après chaque modification de code
5. **Redémarrer Claude Desktop** après chaque build release
6. **Pas de breaking changes** sur le schéma LanceDB sans migration
7. **Un sous-agent par domaine** — ne pas mélanger les responsabilités

## Sous-agents disponibles

| Agent           | CLAUDE.md                            | Domaine                        |
| --------------- | ------------------------------------ | ------------------------------ |
| Harvester Agent | `crates/osmozzz-harvester/CLAUDE.md` | Nouvelles sources de données   |
| MCP Tools Agent | `crates/osmozzz-cli/CLAUDE.md`       | Interface Claude (tools MCP)   |
| Storage Agent   | `crates/osmozzz-embedder/CLAUDE.md`  | LanceDB, recherche, embeddings |

## Pattern d'un nouveau harvester

1. Créer `crates/osmozzz-harvester/src/nom_source.rs`
2. Implémenter le trait `Harvester` de osmozzz-core
3. Exporter depuis `crates/osmozzz-harvester/src/lib.rs`
4. Ajouter CLI dans `crates/osmozzz-cli/src/commands/index.rs`
5. Ajouter MCP tool dans `crates/osmozzz-cli/src/commands/mcp.rs`
6. Build release + restart Claude Desktop

---

## Architecture MCP — Comment fonctionne le système Claude ↔ OSMOzzz

### Principe fondamental

**OSMOzzz fait TOUT le travail de recherche et de filtrage. Claude ne voit que des résultats déjà triés.**
Claude ne peut jamais accéder directement à LanceDB, aux emails, aux fichiers. Il appelle un outil
et reçoit du texte formaté. Aucune donnée ne sort du Mac.

### Protocole : stdin/stdout JSON-RPC 2.0

```
Claude Desktop ──► osmozzz (process Rust)
                   stdin : {"method":"tools/call","params":{"name":"search_emails","arguments":{…}}}
                   stdout: {"result":{"content":[{"type":"text","text":"EMAIL #1 | Objet: …"}]}}
```

Au démarrage : Claude envoie `tools/list` → osmozzz répond avec les 12 tools + leurs descriptions.
Claude lit les descriptions et décide SEUL quel tool appeler. C'est tout ce qu'il "connaît" d'osmozzz.

### Les 3 moteurs de recherche dans osmozzz

**1. Vectoriel ONNX → LanceDB (sémantique)**

- Tool : `search_memory`
- Flux : query → OnnxEmbedder.embed() → vecteur 384d → LanceDB ANN (cosine) → top-N résultats
- Blended : global top-N + 3 résultats email forcés (car emails ont des scores faibles 0.27-0.35 vs Chrome 0.72-0.85)
- Bon pour : concepts généraux, sujets vagues

**2. Keyword scan `.contains()` (exact)**

- Tools : `search_emails`, `search_messages`, `search_notes`, `search_terminal`, `search_calendar`
- Flux : LanceDB scan de 100k docs en mémoire Rust → filtre `.contains()` → tri par date
- Aucun ONNX, aucun vecteur
- Bon pour : noms propres (Revolut, Jean-Luc…), recherche exacte

**3. Filesystem direct**

- `find_file` : walkdir sur ~/Desktop, ~/Documents, ~/code — métadonnées filesystem
- `fetch_content` sans query : lit le fichier brut (offset/length) → retourne texte brut à Claude
- `fetch_content` avec query : ONNX score chaque bloc du fichier → retourne le meilleur bloc + carte de navigation
- `get_recent_files` : walkdir + filtre mtime
- `list_directory` : std::fs::read_dir()

### Qui fait quoi — règle absolue

| Tâche                           | Qui la fait                            |
| ------------------------------- | -------------------------------------- |
| Choisir quel tool appeler       | Claude (lit les descriptions)          |
| Recherche / filtrage / ranking  | OSMOzzz (Rust)                         |
| Embedding ONNX                  | OSMOzzz (local, all-MiniLM-L6-v2 384d) |
| Stockage et requêtes            | LanceDB local (~/.osmozzz/vault/)      |
| Interpréter le texte reçu       | Claude (dans sa fenêtre de contexte)   |
| Accès direct aux données brutes | IMPOSSIBLE pour Claude                 |

### Cas particulier : fetch_content

C'est le seul cas où Claude "voit" du contenu brut :

- Sans `query` : osmozzz retourne un bloc de texte brut → Claude peut raisonner dessus
- Avec `query` : osmozzz+ONNX score les blocs et retourne le meilleur → Claude reçoit directement le résultat pertinent
- Dans les deux cas, osmozzz intermédie toujours. Claude ne peut pas `open()` un fichier seul.

### Les 12 tools MCP actuels

| Tool                 | Moteur                           | Source         |
| -------------------- | -------------------------------- | -------------- |
| `search_memory`      | ONNX + LanceDB vectoriel         | Toutes sources |
| `search_emails`      | keyword `.contains()`            | email          |
| `get_emails_by_date` | filtre date LanceDB              | email          |
| `read_email`         | LanceDB par URL                  | email          |
| `search_messages`    | keyword `.contains()`            | imessage       |
| `search_notes`       | keyword `.contains()`            | notes          |
| `search_terminal`    | keyword `.contains()`            | terminal       |
| `search_calendar`    | keyword `.contains()`            | calendar       |
| `find_file`          | walkdir filesystem               | fichiers Mac   |
| `fetch_content`      | lecture fichier + ONNX optionnel | fichiers Mac   |
| `get_recent_files`   | walkdir + mtime                  | fichiers Mac   |
| `list_directory`     | std::fs::read_dir                | fichiers Mac   |
