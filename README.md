# OSMOzzz

Local semantic memory for AI agents. Indexes your personal data (Chrome history, Markdown, text files) into a local vector database, enabling semantic search without sending data to the cloud.

## Architecture

```
[Sources]          [Core Engine]           [Consumers]
Chrome SQLite  ─┐
PDF/Markdown   ─┤─ Harvesters → Embedder → LanceDB
Slack (later)  ─┘                    │
                                     └──────→ UDS Bridge → OpenClaw
                                             (~/.osmozzz/osmozzz.sock)
```

## Prerequisites

1. **Rust** (1.75+):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **ONNX Runtime** (for `ort load-dynamic`):
   ```bash
   # macOS via Homebrew
   brew install onnxruntime
   # Or set ORT_DYLIB_PATH to a downloaded ORT library:
   # https://github.com/microsoft/onnxruntime/releases
   export ORT_DYLIB_PATH=$(brew --prefix onnxruntime)/lib/libonnxruntime.dylib
   ```

3. **ONNX model**: Run the download script (see below)

## Quick Start

```bash
# 1. Download the embedding model (~90 MB)
./scripts/download-model.sh

# 2. Build
cargo build --release

# 3. Index Chrome browsing history
./target/release/osmozzz index --source chrome

# 4. Search
./target/release/osmozzz search "kubernetes deployment config"

# 5. Index local Markdown / text files
./target/release/osmozzz index --source files --path ~/Documents

# 6. Start the UDS bridge for OpenClaw
./target/release/osmozzz serve

# 7. Check status
./target/release/osmozzz status
```

## CLI Reference

| Command | Description |
|---------|-------------|
| `osmozzz index --source chrome` | Index Chrome browsing history |
| `osmozzz index --source files --path ~/Documents` | Index local files (.md, .txt) |
| `osmozzz search "query" --limit 5` | Semantic search |
| `osmozzz search "query" --format json` | Search with JSON output |
| `osmozzz serve` | Start UDS bridge daemon |
| `osmozzz status` | Show stats and status |

## UDS Bridge Protocol (for OpenClaw)

The bridge listens at `~/.osmozzz/osmozzz.sock` (permissions 600).

**Request:**
```json
{"method": "search", "query": "k8s deployment", "limit": 3}
```

**Response:**
```json
{
  "results": [
    {"content": "...", "score": 0.94, "source": "chrome", "url": "https://..."}
  ]
}
```

Other methods: `{"method": "ping"}`, `{"method": "status"}`

## Data Layout

```
~/.osmozzz/
├── vault/          # LanceDB vector database
└── osmozzz.sock    # UDS socket (when daemon is running)

models/             # ONNX model files (not committed to git)
├── all-MiniLM-L6-v2.onnx
└── tokenizer.json
```

## Security

- **Shadow copy**: Chrome's History DB is copied to a temp file — OSMOzzz never writes to app databases
- **Local inference**: ONNX runtime runs entirely offline — no embeddings leave your machine
- **UDS only**: No TCP port, no network exposure; socket restricted to current user (mode 600)
- **Data stored locally**: Everything in `~/.osmozzz/` under your home directory

## Workspace Structure

```
osmozzz/
├── crates/
│   ├── osmozzz-core/       # Types, traits, errors
│   ├── osmozzz-harvester/  # Chrome + file harvesters
│   ├── osmozzz-embedder/   # ONNX + LanceDB vault
│   ├── osmozzz-bridge/     # UDS server
│   └── osmozzz-cli/        # Binary (clap)
├── models/                 # ONNX model (gitignored)
└── scripts/                # Utilities
```
