#!/usr/bin/env bash
# Copy Ollama runtime (llama-server + libs) next to the sidecar inside the .app bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME="$ROOT/src-tauri/binaries/runtime"

if [[ ! -f "$RUNTIME/llama-server" ]]; then
  echo "copy-ollama-runtime: runtime missing — run pnpm run fetch-ollama first." >&2
  exit 1
fi

APP="$(find "$ROOT/src-tauri/target/release/bundle" -maxdepth 3 -name '*.app' -type d 2>/dev/null | head -1)"
if [[ -z "$APP" ]]; then
  echo "copy-ollama-runtime: no .app bundle found under target/release/bundle" >&2
  exit 1
fi

MACOS="$APP/Contents/MacOS"
echo "Copying Ollama runtime into $MACOS ..."
cp -R "$RUNTIME"/* "$MACOS/"
chmod +x "$MACOS/llama-server" "$MACOS/llama-quantize" 2>/dev/null || true
echo "Done — llama-server is bundled next to the Ollama sidecar."
