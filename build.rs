use std::fs;
use std::path::{Path, PathBuf};

/// This build script ensures documentation images are available in `cargo doc` output.
/// It copies `doc/media/` into `target/doc/cargo_e/doc/media/`
/// to maintain a consistent structure for documentation rendering.
fn main() {
    let src = Path::new("doc/media"); // Source images
    let dest = Path::new("target/doc/media"); // Destination inside crate docs

    println!("cargo:rerun-if-changed=doc/media"); // Ensure script runs if files change

    if !src.exists() {
        eprintln!("Warning: Source directory {:?} does not exist!", src);
        return;
    }

    // Ensure the destination directory exists
    if let Err(e) = fs::create_dir_all(dest) {
        eprintln!("Error: Could not create {:?}: {}", dest, e);
        return;
    }

    // Copy each image from `doc/media/` into `target/doc/cargo_e/doc/media/`
    for entry in src.read_dir().expect("Failed to read source directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        let dest_path = dest.join(path.file_name().unwrap());

        if let Err(e) = fs::copy(&path, &dest_path) {
            eprintln!("Warning: Failed to copy {:?} to {:?}: {}", path, dest_path, e);
        } else {
            println!("Copied {:?} to {:?}", path, dest_path);
        }
    }

    println!("✅ Image copying complete.");
}

