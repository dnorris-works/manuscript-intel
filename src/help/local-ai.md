## Local AI & System Requirements

Loremetry includes **bundled local AI** (Ollama + default model) on macOS. You can also use **TokenMix** cloud models — switch anytime in **Settings → AI Models**.

### Minimum requirements

| | Minimum | Recommended |
|---|---|---|
| RAM | 8 GB | 16 GB+ |
| Free disk | 8 GB | 20 GB |
| macOS | 10.15+ | Apple Silicon or Intel 2018+ with 16 GB RAM |
| Chip | Intel or Apple Silicon | Apple Silicon (much faster local inference) |

### What to expect

- **Local AI** powers every feature: analyzer, writing assistant, and all reports.
- The **Ollama runtime and default model (`phi4-mini`)** are bundled in the installer — **no downloads after install**. Works fully offline.
- The installer is large (~3 GB) because the model is included at build time.
- **Full-manuscript analysis on 8 GB Intel Macs** may take hours on CPU. Switch to TokenMix in Settings when you want faster runs.
- Chapter summarization on **local AI** sends a ~2,000-word representative excerpt per chapter (not the full text) to keep runs practical on CPU hardware.
- The Ollama runtime ships inside the app — you do not need to install Ollama separately.

### Building from source

Before packaging, run once on a Mac:

```bash
pnpm run fetch-ollama   # downloads Ollama runtime + phi4-mini model
pnpm tauri:build        # fetch-ollama runs automatically
```

### Provider comparison

| | Local (included) | TokenMix (cloud) |
|---|---|---|
| Privacy | Runs on your Mac | Sends text to cloud API |
| Cost | Free (no API fees) | Per-token API pricing |
| Speed | Depends on your hardware | Generally faster |
| Setup | None — ready after install | API key in Settings |
