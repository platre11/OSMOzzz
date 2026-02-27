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
| Agent | CLAUDE.md | Domaine |
|---|---|---|
| Harvester Agent | `crates/osmozzz-harvester/CLAUDE.md` | Nouvelles sources de données |
| MCP Tools Agent | `crates/osmozzz-cli/CLAUDE.md` | Interface Claude (tools MCP) |
| Storage Agent | `crates/osmozzz-embedder/CLAUDE.md` | LanceDB, recherche, embeddings |

## Pattern d'un nouveau harvester
1. Créer `crates/osmozzz-harvester/src/nom_source.rs`
2. Implémenter le trait `Harvester` de osmozzz-core
3. Exporter depuis `crates/osmozzz-harvester/src/lib.rs`
4. Ajouter CLI dans `crates/osmozzz-cli/src/commands/index.rs`
5. Ajouter MCP tool dans `crates/osmozzz-cli/src/commands/mcp.rs`
6. Build release + restart Claude Desktop
