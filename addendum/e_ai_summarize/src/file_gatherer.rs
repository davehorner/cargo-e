use anyhow::{Context, Result}; // for context handling and error management
use log::{debug, info};
use path_slash::PathExt;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir; // Import logging macros

pub fn gather_files(source_folder: &Path) -> Result<HashMap<PathBuf, String>> {
    let mut files = HashMap::new();
    debug!("Traversing folder: {:?}", source_folder);

    for entry in WalkDir::new(source_folder)
        .into_iter()
        .filter_entry(|e| {
            // Skip any entry whose path contains one of the unwanted directory names.
            let excluded = ["target", ".git", ".aipack", ".github", "node_modules"];
            !e.path().components().any(|comp| {
                comp.as_os_str()
                    .to_str()
                    .is_some_and(|s| excluded.contains(&s))
            })
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if ["target", ".git", ".aipack", ".github"].contains(&name) {
                    debug!("Skipping directory: {}", name);
                    continue;
                }
            }
        } else if path.is_file() {
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name == ".gitignore"
                || file_name == "Cargo.lock"
                || file_name.ends_with(".bak")
                || file_name.ends_with("~")
                || file_name.starts_with("LICENSE")
                || file_name.starts_with("NOTICE")
                || file_name.eq("run_history.txt")
            {
                debug!("Skipping file: {}", file_name);
                continue;
            }
            // Skip files whose names contain "_recreate_"
            if file_name.contains("_recreate_") {
                debug!(
                    "Skipping file: {} because it contains '_recreate_'",
                    file_name
                );
                continue;
            }
            let lower = file_name.to_lowercase();
            if lower.ends_with(".webp")
                || lower.ends_with(".jpg")
                || lower.ends_with(".jpeg")
                || lower.ends_with(".png")
                || lower.ends_with(".pdb")
                || lower.ends_with(".exe")
            {
                debug!("Skipping binary file: {}", file_name);
                continue;
            }
            let rel_path = path
                .strip_prefix(source_folder)
                .with_context(|| format!("Failed to get relative path for {:?}", path))?;

            let slash_path: String = crate::sanitize(rel_path.to_slash_lossy());

            debug!("Processing file: {:?} as {:?}", path, slash_path);

            // Read the file contents
            let bytes =
                fs::read(path).with_context(|| format!("Failed to read file {:?}", path))?;
            let content = String::from_utf8_lossy(&bytes).to_string();
            let sanitized = crate::sanitize(content);
            let rust_literal = crate::emit_literal(&sanitized);
            // Insert the processed file into the HashMap
            files.insert(rel_path.to_path_buf(), rust_literal);
        }
    }

    info!("Total files gathered: {}", files.len());
    Ok(files)
}
