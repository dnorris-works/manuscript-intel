fn main() {
  // Tauri bundles binaries/models as a resource; ensure the path exists before build.
  std::fs::create_dir_all("binaries/models").expect("failed to create binaries/models");
  tauri_build::build()
}
