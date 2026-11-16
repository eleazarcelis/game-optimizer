fn main() {
    println!("cargo:rerun-if-changed=../package.json");
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var("PROFILE").as_deref() != Ok("release") {
        tauri_build::build();
        return;
    }
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest")),
    ))
    .expect("failed to build Tauri application");
}
