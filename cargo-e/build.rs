// build.rs

mod build_docs;
mod build_readme;

fn main() {
    build_readme::update_readme();
    // --- Documentation Copying ---
    // Call our documentation helper to copy media files.
    build_docs::copy_doc_media();
}
