use std::fs;
use std::path::PathBuf;

fn main() {
    let attributes = tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "shell_state",
            "gallery",
            "open_shot",
            "copy_shots",
            "open_folder",
            "open_log",
            "set_shortcut",
            "set_retention_days",
            "set_language",
        ]),
    );
    tauri_build::try_build(attributes).expect("tauri-build");
    copy_changelog();
}

fn copy_changelog() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let profile_dir = out_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect("profile directory above OUT_DIR");
    let source = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("CHANGELOG.md");
    let destination = profile_dir.join("CHANGELOG.md");
    fs::copy(&source, &destination).expect("copy CHANGELOG.md beside the exe");
    let deps = profile_dir.join("deps");
    fs::create_dir_all(&deps).expect("deps directory");
    fs::copy(&source, deps.join("CHANGELOG.md"))
        .expect("copy CHANGELOG.md beside test binaries");
    println!("cargo:rerun-if-changed=CHANGELOG.md");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=ui");
}
