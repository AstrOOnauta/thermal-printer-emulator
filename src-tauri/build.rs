fn main() {
    // Listing the app commands makes each one need an explicit `allow-*` permission in
    // capabilities/, instead of being callable by default from every window.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "app_locale",
            "get_receipts",
            "get_receipt",
            "get_listener_status",
        ]),
    ))
    .expect("failed to run tauri-build");
}
