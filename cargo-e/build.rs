// build.rs

mod build_docs;

fn main() {
    // --- Documentation Copying ---
    // Call our documentation helper to copy media files.
    build_docs::copy_doc_media();
    // --- Fortune Data Path ---
    // If consuming crate enables "funny-docs" (which pulls in the "fortune" feature),
    // set the path to a static fortunes file via env var.
    if std::env::var("CARGO_FEATURE_FUNNY_DOCS").is_ok() {
        if std::env::var("E_CRATE_FORTUNE_PATH").is_err() {
            let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
            let default = format!("{}/build_data/fortunes.txt", manifest);
            println!("cargo:rustc-env=E_CRATE_FORTUNE_PATH={}", default);
        }
    }
}
