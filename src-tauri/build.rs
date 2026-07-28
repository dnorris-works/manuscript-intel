use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
  fs::create_dir_all("binaries/models").expect("failed to create binaries/models");
  tauri_build::build();
  copy_ollama_runtime_to_target();
}

fn target_dir() -> Option<PathBuf> {
  let out = PathBuf::from(env::var_os("OUT_DIR")?);
  // OUT_DIR = target/{debug,release}/build/<pkg>/out
  out.ancestors().nth(3).map(Path::to_path_buf)
}

fn copy_dir_merge(src: &Path, dst: &Path) -> std::io::Result<()> {
  if !src.is_dir() {
    return Ok(());
  }
  fs::create_dir_all(dst)?;
  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let from = entry.path();
    let to = dst.join(entry.file_name());
    if from.is_dir() {
      copy_dir_merge(&from, &to)?;
    } else {
      fs::copy(&from, &to)?;
      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        if entry.file_name() == "llama-server"
          || entry.file_name() == "llama-quantize"
          || entry.file_name() == "ollama"
        {
          let mut perms = fs::metadata(&to)?.permissions();
          perms.set_mode(0o755);
          fs::set_permissions(&to, perms)?;
        }
      }
    }
  }
  Ok(())
}

fn copy_ollama_runtime_to_target() {
  let runtime = Path::new("binaries/runtime");
  if !runtime.join("llama-server").exists() {
    println!("cargo:warning=Ollama runtime missing — run: pnpm run fetch-ollama");
    return;
  }
  let Some(target) = target_dir() else {
    println!("cargo:warning=Could not resolve target dir for Ollama runtime copy");
    return;
  };
  if let Err(e) = copy_dir_merge(runtime, &target) {
    println!("cargo:warning=Failed to copy Ollama runtime to target: {}", e);
    return;
  }
  println!("cargo:rerun-if-changed=binaries/runtime");
}
