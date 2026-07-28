#!/usr/bin/env bash
# Fetch Ollama runtime + default model for macOS builds.
# Run before packaging: pnpm run fetch-ollama
set -euo pipefail

OLLAMA_VERSION="${OLLAMA_VERSION:-0.32.5}"
OLLAMA_MODEL="${OLLAMA_MODEL:-phi4-mini}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/binaries"
RUNTIME="$DEST/runtime"
MODELS_DIR="$DEST/models"
URL="https://github.com/ollama/ollama/releases/download/v${OLLAMA_VERSION}/ollama-darwin.tgz"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "fetch-ollama: macOS only. Build the Loremetry app on a Mac." >&2
  exit 1
fi

mkdir -p "$DEST" "$MODELS_DIR"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading Ollama v${OLLAMA_VERSION}..."
curl -fsSL "$URL" -o "$TMP/ollama-darwin.tgz"
tar -xzf "$TMP/ollama-darwin.tgz" -C "$TMP"

echo "Installing runtime (llama-server + libraries)..."
rm -rf "$RUNTIME"
mkdir -p "$RUNTIME"
for item in "$TMP"/*; do
  base="$(basename "$item")"
  [[ "$base" == "ollama-darwin.tgz" ]] && continue
  cp -R "$item" "$RUNTIME/"
done
chmod +x "$RUNTIME"/ollama "$RUNTIME"/llama-server "$RUNTIME"/llama-quantize 2>/dev/null || true
xattr -dr com.apple.quarantine "$RUNTIME" 2>/dev/null || true

if [[ ! -f "$RUNTIME/llama-server" ]]; then
  echo "fetch-ollama: llama-server missing after extract." >&2
  exit 1
fi

echo "Installing sidecar binaries..."
for triple in x86_64-apple-darwin aarch64-apple-darwin; do
  cp "$RUNTIME/ollama" "$DEST/ollama-${triple}"
  chmod +x "$DEST/ollama-${triple}"
  xattr -d com.apple.quarantine "$DEST/ollama-${triple}" 2>/dev/null || true
done

model_present() {
  [[ -d "$MODELS_DIR/manifests" ]] && find "$MODELS_DIR/manifests" -maxdepth 1 -name "*${OLLAMA_MODEL}*" -print -quit | grep -q .
}

if model_present; then
  echo "Model ${OLLAMA_MODEL} already in ${MODELS_DIR}, skipping pull."
else
  echo "Downloading model ${OLLAMA_MODEL} (~2–4 GB, one-time at build)..."
  mkdir -p "$MODELS_DIR"
  SERVE_PORT=11599
  while nc -z 127.0.0.1 "$SERVE_PORT" 2>/dev/null; do
    SERVE_PORT=$((SERVE_PORT + 1))
  done
  SERVE_HOST="127.0.0.1:${SERVE_PORT}"

  OLLAMA_HOST="$SERVE_HOST" \
    OLLAMA_MODELS="$MODELS_DIR" \
    OLLAMA_LIBRARY_PATH="$RUNTIME" \
    "$RUNTIME/ollama" serve >/tmp/loremetry-ollama-serve.log 2>&1 &
  SERVE_PID=$!
  trap 'kill "$SERVE_PID" 2>/dev/null || true; rm -rf "$TMP"' EXIT

  ready=0
  for _ in $(seq 1 90); do
    if curl -sf "http://${SERVE_HOST}/api/tags" >/dev/null 2>&1; then
      ready=1
      break
    fi
    if ! kill -0 "$SERVE_PID" 2>/dev/null; then
      echo "fetch-ollama: ollama serve exited during startup. Log:" >&2
      tail -30 /tmp/loremetry-ollama-serve.log >&2 || true
      exit 1
    fi
    sleep 1
  done
  if [[ "$ready" -ne 1 ]]; then
    echo "fetch-ollama: Ollama did not become ready for model pull." >&2
    exit 1
  fi

  OLLAMA_HOST="$SERVE_HOST" \
    OLLAMA_MODELS="$MODELS_DIR" \
    OLLAMA_LIBRARY_PATH="$RUNTIME" \
    "$RUNTIME/ollama" pull "$OLLAMA_MODEL"

  kill "$SERVE_PID" 2>/dev/null || true
  wait "$SERVE_PID" 2>/dev/null || true
  trap 'rm -rf "$TMP"' EXIT

  if ! model_present; then
    echo "fetch-ollama: model pull finished but ${OLLAMA_MODEL} not found in manifests." >&2
    exit 1
  fi
fi

echo ""
echo "Installed sidecars:"
ls -lh "$DEST"/ollama-*
echo "Runtime: $(du -sh "$RUNTIME" | cut -f1)"
echo "Models (${OLLAMA_MODEL}): $(du -sh "$MODELS_DIR" | cut -f1)"
echo "Ready to package — no post-install downloads required."
