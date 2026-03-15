#!/bin/bash
# Build rapide OSMOzzz — utilise le cache Rust (incremental)
# Usage: ./build.sh
# Au lieu de `cargo install` (~7 min), utilise `cargo build --release` (~30s si peu de changements)

set -e
WORKSPACE="$(cd "$(dirname "$0")" && pwd)"

# ── 1. Frontend : rebuild seulement si src/ a changé ──────────────────────────
DIST="$WORKSPACE/dashboard/dist/index.html"
CHANGED=$(find "$WORKSPACE/dashboard/src" -type f -newer "$DIST" 2>/dev/null | wc -l | tr -d ' ')

if [ ! -f "$DIST" ] || [ "$CHANGED" -gt 0 ]; then
    echo "[build] Frontend modifié — npm run build..."
    cd "$WORKSPACE/dashboard" && npm run build
    touch "$WORKSPACE/crates/osmozzz-api/src/server.rs"
    echo "[build] Frontend OK"
else
    echo "[build] Frontend inchangé — skip"
fi

# ── 2. Rust : compilation incrementale (cache) ────────────────────────────────
cd "$WORKSPACE"
echo "[build] Compilation Rust..."
cargo build --release -p osmozzz-cli

# ── 3. Copie le binaire dans ~/.cargo/bin ─────────────────────────────────────
cp "$WORKSPACE/target/release/osmozzz" ~/.cargo/bin/osmozzz

# ── 4. Copie les modèles dans ~/.osmozzz/models/ (fonctionne après reboot) ────
mkdir -p ~/.osmozzz/models
if [ -f "$WORKSPACE/models/all-MiniLM-L6-v2.onnx" ]; then
    cp "$WORKSPACE/models/all-MiniLM-L6-v2.onnx" ~/.osmozzz/models/
    cp "$WORKSPACE/models/tokenizer.json" ~/.osmozzz/models/
    echo "[build] Modèles copiés dans ~/.osmozzz/models/"
fi

echo "[build] Done — relance: osmozzz daemon"
