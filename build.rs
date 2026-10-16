use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = include_str!("app.manifest");
    let mut resource = winresource::WindowsResource::new();
    resource.set_manifest(manifest);
    resource.compile().expect("embed the asInvoker manifest");
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
    println!("cargo:rerun-if-changed=app.manifest");
}
