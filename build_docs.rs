// build_docs.rs

use std::fs;
use std::path::Path;

/// Copies documentation media files from the source directory into the documentation output directory.
///
/// This function looks for files under "documents/media" and copies them into "target/doc/media",
/// ensuring that when `cargo doc` is run, the images will appear with the expected structure.
pub fn copy_doc_media() {
    // Define source and destination paths.
    let src = Path::new("documents/media"); // Source images.
    let dest = Path::new("target/doc/media"); // Destination inside crate docs.

    // Instruct Cargo to re-run this script if files in the documents/media folder change.
    println!("cargo:rerun-if-changed=documents/media");

    if !src.exists() {
        eprintln!("Warning: Source directory {:?} does not exist!", src);
        return;
    }

    // Ensure that the destination directory exists.
    if let Err(e) = fs::create_dir_all(dest) {
        eprintln!("Error: Could not create destination {:?}: {}", dest, e);
        return;
    }

    // Copy each file from the source directory to the destination.
    for entry in src.read_dir().expect("Failed to read source directory") {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to read directory entry: {}", e);
                continue;
            }
        };
        let path = entry.path();
        let dest_path = dest.join(path.file_name().unwrap());

        if let Err(e) = fs::copy(&path, &dest_path) {
            eprintln!(
                "Warning: Failed to copy {:?} to {:?}: {}",
                path, dest_path, e
            );
        } else {
            println!("Copied {:?} to {:?}", path, dest_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    /// This test creates temporary directories and a dummy file to simulate the copy operation.
    /// Note: In a real test you might want to refactor copy_doc_media to accept custom paths.
    #[test]
    fn test_copy_doc_media_simulated() {
        let temp = tempdir().unwrap();
        let src = temp.path().join("media_src");
        let dest = temp.path().join("media_dest");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dest).unwrap();

        // Create a dummy file in src.
        let file_path = src.join("dummy.txt");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "dummy content").unwrap();

        // Perform a simulated copy.
        let dest_file = dest.join("dummy.txt");
        let result = fs::copy(&file_path, &dest_file);
        assert!(result.is_ok());
        let copied_content = fs::read_to_string(&dest_file).unwrap();
        assert_eq!(copied_content.trim(), "dummy content");
    }
}
