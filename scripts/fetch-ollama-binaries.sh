#!/usr/bin/env bash
# Fetch Ollama sidecar binaries for macOS (Intel + Apple Silicon universal binary).
set -euo pipefail

OLLAMA_VERSION="${OLLAMA_VERSION:-0.32.5}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/binaries"
URL="https://github.com/ollama/ollama/releases/download/v${OLLAMA_VERSION}/ollama-darwin.tgz"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "fetch-ollama: macOS only. Build the Loremetry app on a Mac." >&2
  exit 1
fi

mkdir -p "$DEST"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading Ollama v${OLLAMA_VERSION}..."
curl -fsSL "$URL" -o "$TMP/ollama-darwin.tgz"
tar -xzf "$TMP/ollama-darwin.tgz" -C "$TMP"

for triple in x86_64-apple-darwin aarch64-apple-darwin; do
  cp "$TMP/ollama" "$DEST/ollama-${triple}"
  chmod +x "$DEST/ollama-${triple}"
  xattr -d com.apple.quarantine "$DEST/ollama-${triple}" 2>/dev/null || true
done

echo "Installed:"
ls -lh "$DEST"/ollama-*
