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
    echo "[build] Frontend OK"
else
    echo "[build] Frontend inchangé — skip"
fi

# Toujours forcer l'embed du dashboard dans le binaire Rust
touch "$WORKSPACE/crates/osmozzz-api/src/server.rs"

# ── 2. Rust : compilation incrementale (cache) ────────────────────────────────
cd "$WORKSPACE"
echo "[build] Compilation Rust..."
cargo build -p osmozzz-cli

# ── 3. Copie le binaire dans ~/.cargo/bin ─────────────────────────────────────
cp "$WORKSPACE/target/debug/osmozzz" ~/.cargo/bin/osmozzz
# Re-signe le binaire (macOS 26 rejette les binaires avec flag linker-signed de cargo)
codesign --force --sign - ~/.cargo/bin/osmozzz

# ── 4. Kill le daemon actif (l'utilisateur relance manuellement) ──────────────
pkill -f "osmozzz daemon" 2>/dev/null && echo "[build] Daemon stoppé — relance: osmozzz daemon" || echo "[build] Done — relance: osmozzz daemon"
