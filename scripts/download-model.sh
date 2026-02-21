#!/usr/bin/env bash
# Downloads all-MiniLM-L6-v2 ONNX model for OSMOzzz
set -euo pipefail

MODELS_DIR="$(cd "$(dirname "$0")/.." && pwd)/models"
mkdir -p "$MODELS_DIR"

BASE_URL="https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main"
ONNX_URL="https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx"
TOK_URL="${BASE_URL}/tokenizer.json"

echo "Downloading all-MiniLM-L6-v2 ONNX model..."
echo "Target: $MODELS_DIR"

# ONNX model (~90MB)
if [ ! -f "$MODELS_DIR/all-MiniLM-L6-v2.onnx" ]; then
    echo ""
    echo "  Downloading model (~90 MB)..."
    curl -L --progress-bar "$ONNX_URL" -o "$MODELS_DIR/all-MiniLM-L6-v2.onnx"
    echo "  Model downloaded."
else
    echo "  Model already present, skipping."
fi

# Tokenizer (~~500 KB)
if [ ! -f "$MODELS_DIR/tokenizer.json" ]; then
    echo ""
    echo "  Downloading tokenizer..."
    curl -L --progress-bar "$TOK_URL" -o "$MODELS_DIR/tokenizer.json"
    echo "  Tokenizer downloaded."
else
    echo "  Tokenizer already present, skipping."
fi

echo ""
echo "Done! Models saved to: $MODELS_DIR"
echo "You can now run: cargo build --release && osmozzz index --source chrome"
