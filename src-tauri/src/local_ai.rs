// local_ai.rs — Bundled Ollama sidecar lifecycle

use serde::Serialize;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

pub const DEFAULT_LOCAL_MODEL: &str = "phi4-mini";

static LOCAL_BASE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static LOCAL_READY: AtomicBool = AtomicBool::new(false);

struct LocalAiProcess {
    #[allow(dead_code)]
    child: tauri_plugin_shell::process::CommandChild,
}

pub struct LocalAiState {
    process: Mutex<Option<LocalAiProcess>>,
}

#[derive(Clone, Serialize)]
pub struct LocalAiStatus {
    pub running: bool,
    pub ready: bool,
    pub base_url: String,
    pub models: Vec<String>,
    pub default_model_installed: bool,
}

pub fn is_ready() -> bool {
    LOCAL_READY.load(Ordering::SeqCst)
}

pub fn base_url() -> Option<&'static str> {
    LOCAL_BASE.get().map(|s| s.as_str())
}

fn pick_port() -> Result<u16, String> {
    for port in [11434u16, 11435, 11436, 11437, 11438, 11439, 11440] {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    Err("No free port available for local AI.".to_string())
}

async fn wait_until_ready(base: &str, max_secs: u64) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let url = format!("{}/api/tags", base.trim_end_matches('/'));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(max_secs);
    while std::time::Instant::now() < deadline {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                LOCAL_READY.store(true, Ordering::SeqCst);
                return true;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    false
}

pub async fn fetch_installed_models(base: &str) -> Vec<String> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let url = format!("{}/api/tags", base.trim_end_matches('/'));
    let Ok(resp) = client.get(&url).send().await else {
        return Vec::new();
    };
    let Ok(json) = resp.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    json.get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn model_installed(models: &[String], name: &str) -> bool {
    let base = name.split(':').next().unwrap_or(name);
    models.iter().any(|m| {
        m == name
            || m.starts_with(&format!("{}:", base))
            || m.split(':').next() == Some(base)
    })
}

/// Directory containing llama-server and inference libraries from the Ollama darwin tarball.
fn ollama_library_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    #[cfg(debug_assertions)]
    {
        let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/runtime");
        if dev.join("llama-server").exists() {
            return Ok(dev);
        }
    }

    if let Ok(resource) = app.path().resource_dir() {
        let bundled = resource.join("ollama-runtime");
        if bundled.join("llama-server").exists() {
            return Ok(bundled);
        }
    }

    Err(
        "Ollama runtime not found (llama-server missing). Run: pnpm run fetch-ollama"
            .to_string(),
    )
}

fn bundled_models_source(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    #[cfg(debug_assertions)]
    {
        let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/models");
        if dev.join("manifests").is_dir() {
            return Ok(dev);
        }
    }

    if let Ok(resource) = app.path().resource_dir() {
        let bundled = resource.join("ollama-models");
        if bundled.join("manifests").is_dir() {
            return Ok(bundled);
        }
    }

    Err(
        "Bundled Ollama models not found. Run: pnpm run fetch-ollama before building."
            .to_string(),
    )
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Use bundled models. Dev reads directly from binaries/models; release seeds app data once from the bundle.
fn ollama_models_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    #[cfg(debug_assertions)]
    {
        let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries/models");
        if dev.join("manifests").is_dir() {
            return Ok(dev);
        }
    }

    let app_models = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("ollama")
        .join("models");

    if app_models.join("manifests").is_dir() {
        return Ok(app_models);
    }

    let bundled = bundled_models_source(app)?;
    std::fs::create_dir_all(&app_models).map_err(|e| e.to_string())?;
    copy_dir_all(&bundled, &app_models).map_err(|e| {
        format!("Failed to install bundled models to app data: {}", e)
    })?;
    Ok(app_models)
}

pub fn start_local_ai(app: &AppHandle) -> Result<(), String> {
    let models_dir = ollama_models_path(app)?;

    let port = pick_port()?;
    let host = format!("127.0.0.1:{}", port);
    let base = format!("http://{}", host);
    let library_path = ollama_library_path(app)?;

    let sidecar = app
        .shell()
        .sidecar("ollama")
        .map_err(|e| format!("Failed to locate Ollama sidecar: {}", e))?;

    let (mut rx, child) = sidecar
        .args(["serve"])
        .env("OLLAMA_HOST", &host)
        .env("OLLAMA_MODELS", models_dir.to_string_lossy().as_ref())
        .env("OLLAMA_LIBRARY_PATH", library_path.to_string_lossy().as_ref())
        .spawn()
        .map_err(|e| format!("Failed to start Ollama: {}", e))?;

    let app_log = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let CommandEvent::Stderr(line) = event {
                let text = String::from_utf8_lossy(&line);
                if !text.trim().is_empty() {
                    let _ = app_log.emit("local-ai:log", text.to_string());
                }
            }
        }
    });

    let _ = LOCAL_BASE.set(base.clone());

    app.manage(LocalAiState {
        process: Mutex::new(Some(LocalAiProcess { child })),
    });

    let app_wait = app.clone();
    tauri::async_runtime::spawn(async move {
        if wait_until_ready(&base, 45).await {
            let _ = app_wait.emit("local-ai:ready", ());
        } else {
            let _ = app_wait.emit(
                "local-ai:error",
                "Local AI server did not become ready in time.".to_string(),
            );
        }
    });

    Ok(())
}

pub fn shutdown_local_ai(app: &AppHandle) {
    if let Some(state) = app.try_state::<LocalAiState>() {
        let mut guard = state.process.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }
    LOCAL_READY.store(false, Ordering::SeqCst);
}

/// Retry starting local AI (e.g. after a failed first attempt or app resume).
#[tauri::command]
pub fn restart_local_ai(app: AppHandle) -> Result<(), String> {
    shutdown_local_ai(&app);
    start_local_ai(&app)
}

#[tauri::command]
pub async fn local_ai_status(app: AppHandle) -> LocalAiStatus {
    let base = LOCAL_BASE.get().cloned().unwrap_or_default();
    let running = base.is_empty() == false && app.try_state::<LocalAiState>().is_some();
    let ready = is_ready();
    let models = if ready && !base.is_empty() {
        fetch_installed_models(&base).await
    } else {
        Vec::new()
    };
    let default_model_installed = if model_installed(&models, DEFAULT_LOCAL_MODEL) {
        true
    } else {
        bundled_models_source(&app)
            .ok()
            .is_some_and(|p| p.join("manifests").is_dir())
    };
    LocalAiStatus {
        running,
        ready,
        base_url: base,
        models,
        default_model_installed,
    }
}

#[tauri::command]
pub async fn test_local_ai_connection() -> Result<String, String> {
    let base = LOCAL_BASE
        .get()
        .ok_or_else(|| "Local AI is not running.".to_string())?;
    if !is_ready() {
        return Err("Local AI is not ready.".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": DEFAULT_LOCAL_MODEL,
        "max_tokens": 32,
        "messages": [
            {"role": "user", "content": "Reply with exactly: OK"}
        ]
    });
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Connection test failed: {}", e))?;
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    if let Some(err) = json.get("error") {
        return Err(format!(
            "Local AI error: {}",
            err["message"].as_str().unwrap_or("unknown")
        ));
    }
    let reply = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok(reply)
}
