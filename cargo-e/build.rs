// build.rs

use std::env;
use std::path::Path;

// Pull in our custom build modules.
mod build_docs;

fn main() {
    // --- Documentation Copying ---
    // Call our documentation helper to copy media files.
    build_docs::copy_doc_media();
}
