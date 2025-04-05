use anyhow::{Context, Result, bail};
use log::trace;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use which::which;

use toml::Value; // Add toml crate to Cargo.toml

/// Reads the Cargo.toml file and returns the crate name and version.
pub fn get_crate_name_and_version(
    crate_toml_path: &Path,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(crate_toml_path)?;

    // Parse the TOML content
    let toml: Value = toml::de::from_str(&content)?;

    // Try to get the [package] section
    let package = toml
        .get("package")
        .ok_or("Missing [package] section in Cargo.toml")?;

    // Safely extract name and version
    let name = package
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let version = package
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    Ok((name, version))
}

/// Checks if `rust-script` is installed and suggests installation if it's not.
pub fn check_rust_script_installed() {
    match which("rust-script") {
        Ok(_) => {
            // rust-script is installed
            println!("rust-script is installed.");
        }
        Err(_) => {
            // rust-script is not found in the PATH
            eprintln!("rust-script is not installed.");
            println!("Suggestion: To install rust-script, run the following command:");
            println!("cargo install rust-script");
        }
    }
}

/// Searches upward from `start` for a Cargo.toml file.
/// Returns the path to the Cargo.toml if found.
pub fn find_cargo_toml(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.exists() {
            trace!("Found Cargo.toml at: {:?}", candidate);
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    trace!("No Cargo.toml found {}.", current.display());
    None
}

/// Gathers files from a crate by starting at `crate_location`:
///   1. It performs an upward search for a Cargo.toml file.
///   2. If found, the crate root is set to the directory containing Cargo.toml.
///   3. Otherwise, it uses the provided `crate_location` as the root.
///   4. If `src_only` is true, the file gathering will occur in the "src" subfolder;
///      otherwise, it gathers files from the entire crate root.
pub fn gather_files_from_crate(
    crate_location: &str,
    src_only: bool,
) -> Result<HashMap<PathBuf, String>> {
    // Use the file-gathering function from the file_gatherer module.
    use crate::file_gatherer::gather_files;
    let crate_location = if crate_location.is_empty() {
        "."
    } else {
        crate_location
    };
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

    let files = gather_files(&target_dir)
        .with_context(|| format!("Failed to gather files from {:?}", target_dir))?;
    println!("Gathered {} files from {:?}", files.len(), target_dir);
    Ok(files)
}
