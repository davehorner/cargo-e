use anyhow::{Context, Result, bail};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Searches upward from `start` for a Cargo.toml file.
/// Returns the path to the Cargo.toml if found.
pub fn find_cargo_toml(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.exists() {
            println!("[TRACE] Found Cargo.toml at: {:?}", candidate);
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    println!("[TRACE] No Cargo.toml found.");
    None
}

/// Parses the Cargo.toml file to extract the crate name from the \[package\] section.
pub fn get_crate_name_from_cargo_toml(cargo_toml: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml).ok()?;
    let mut in_package = false;
    let re = Regex::new(r#"name\s*=\s*["'](.+?)["']"#).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[package]") {
            in_package = true;
        } else if trimmed.starts_with("[") && in_package {
            break;
        } else if in_package {
            if let Some(caps) = re.captures(trimmed) {
                return Some(caps.get(1)?.as_str().to_string());
            }
        }
    }
    None
}

/// Gathers files from a crate by starting at `crate_location`:
///   1. It performs an upward search for a Cargo.toml file.
///   2. If found, the crate root is set to the directory containing Cargo.toml.
///   3. Otherwise, it uses the provided `crate_location` as the root.
///   4. If `src_only` is true, the file gathering will occur in the "src" subfolder;
///      otherwise, it gathers files from the entire crate root.
/// This function is shared between the gen-script subcommands.
pub fn gather_files_from_crate(
    crate_location: &str,
    src_only: bool,
) -> Result<HashMap<PathBuf, String>> {
    // Use the file-gathering function from the file_gatherer module.
    use crate::file_gatherer::gather_files;

    let start_path = Path::new(crate_location);

    // Search upward for Cargo.toml
    let crate_root = if let Some(cargo_toml_path) = find_cargo_toml(start_path) {
        cargo_toml_path
            .parent()
            .expect("Cargo.toml must be in a directory")
            .to_path_buf()
    } else {
        // Fallback: assume the provided location is the crate root.
        start_path.to_path_buf()
    };

    let target_dir = if src_only {
        crate_root.join("src")
    } else {
        crate_root
    };

    if !target_dir.exists() {
        bail!("Target directory {:?} does not exist.", target_dir);
    }

    println!("[TRACE] Gathering files from {:?}", target_dir);
    let files = gather_files(&target_dir)
        .with_context(|| format!("Failed to gather files from {:?}", target_dir))?;
    Ok(files)
}
