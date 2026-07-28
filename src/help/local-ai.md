## Local AI & System Requirements

Loremetry includes **bundled local AI** (Ollama) on macOS. You can also use **TokenMix** cloud models — switch anytime in **Settings → AI Models**.

### Minimum requirements

| | Minimum | Recommended |
|---|---|---|
| RAM | 8 GB | 16 GB+ |
| Free disk | 8 GB (app + one model) | 20 GB |
| macOS | 10.15+ | Apple Silicon or Intel 2018+ with 16 GB RAM |
| Chip | Intel or Apple Silicon | Apple Silicon (much faster local inference) |

### What to expect

- **Local AI** powers every feature: analyzer, writing assistant, and all reports.
- **First launch** requires internet once to download the default model (~2.5 GB). After that, local AI works offline.
- **Full-manuscript analysis on 8 GB Intel Macs** may take hours on CPU. Switch to TokenMix in Settings when you want faster runs.
- Downloaded models are stored in `~/Library/Application Support/com.davidallennorris.loremetry-desktop/ollama/models/`.
- The Ollama runtime ships inside the app — you do not need to install Ollama separately.

### Provider comparison

| | Local (included) | TokenMix (cloud) |
|---|---|---|
| Privacy | Runs on your Mac | Sends text to cloud API |
| Cost | Free (no API fees) | Per-token API pricing |
| Speed | Depends on your hardware | Generally faster |
| Setup | Download model on first run | API key in Settings |
