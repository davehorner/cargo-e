use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Recursively collects files from the given `source_folder` and returns a mapping of relative paths to file contents.
pub fn gather_files(source_folder: &Path) -> Result<HashMap<PathBuf, String>> {
    let mut files = HashMap::new();
    println!("[TRACE] Traversing folder: {:?}", source_folder);

    for entry in WalkDir::new(source_folder)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if ["target", ".git", ".aipack", ".github"].contains(&name) {
                    println!("[TRACE] Skipping directory: {}", name);
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
            {
                println!("[TRACE] Skipping file: {}", file_name);
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
                println!("[TRACE] Skipping binary file: {}", file_name);
                continue;
            }
            let rel_path = path
                .strip_prefix(source_folder)
                .with_context(|| format!("Failed to get relative path for {:?}", path))?
                .to_path_buf();
            println!("[TRACE] Processing file: {:?} as {:?}", path, rel_path);
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read file {:?}", path))?;
            files.insert(rel_path, content);
        }
    }

    println!("[TRACE] Total files gathered: {}", files.len());
    Ok(files)
}
