// src/e_discovery.rs
use std::{fs, path::Path};

use crate::e_target::{CargoTarget, TargetKind, TargetOrigin};
use anyhow::{Context, Result};

/// Discover targets in the given directory.
/// This function scans for a Cargo.toml and then looks for example files or subproject directories.
pub fn discover_targets(current_dir: &Path) -> Result<Vec<CargoTarget>> {
    let mut targets = Vec::new();
    let parent = current_dir.parent().expect("expected cwd to have a parent");
    // Check if a Cargo.toml exists in the current directory.
    let manifest_path = current_dir.join("Cargo.toml");
    if manifest_path.exists() {
        targets.push(CargoTarget {
            name: "default".to_string(),
            display_name: "Default Manifest".to_string(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            kind: TargetKind::Manifest,
            extended: false,
            origin: None,
        });
    }

    // Scan the "examples" directory for example targets.
    let examples_dir = current_dir.join("examples");
    if examples_dir.exists() && examples_dir.is_dir() {
        for entry in fs::read_dir(&examples_dir)
            .with_context(|| format!("Reading directory {:?}", examples_dir))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                // Assume that any .rs file in examples/ is an example.
                if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        if let Some(stem) = path.file_stem() {
                            targets.push(CargoTarget {
                                name: stem.to_string_lossy().to_string(),
                                display_name: stem.to_string_lossy().to_string(),
                                manifest_path: current_dir
                                    .join("Cargo.toml")
                                    .to_string_lossy()
                                    .to_string(),
                                kind: TargetKind::Example,
                                extended: false,
                                origin: Some(TargetOrigin::SingleFile(path)),
                            });
                        }
                    }
                }
            } else if path.is_dir() {
                // If the directory contains a Cargo.toml, treat it as an extended subproject.
                let sub_manifest = path.join("Cargo.toml");
                if sub_manifest.exists() {
                    if let Some(name) = path.file_name() {
                        targets.push(CargoTarget {
                            name: name.to_string_lossy().to_string(),
                            display_name: format!(
                                "parent{} {}",
                                parent.display(),
                                name.to_string_lossy()
                            ),
                            manifest_path: sub_manifest.to_string_lossy().to_string(),
                            kind: TargetKind::Example,
                            extended: true,
                            origin: Some(TargetOrigin::SubProject(sub_manifest)),
                        });
                    }
                }
            }
        }
    }

    // Additional discovery for binaries or tests can be added here.

    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_discover_targets_no_manifest() {
        let temp = tempdir().unwrap();
        // With no Cargo.toml, we expect an empty list.
        let targets = discover_targets(temp.path()).unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn test_discover_targets_with_manifest_and_example() {
        let temp = tempdir().unwrap();
        // Create a dummy Cargo.toml.
        let manifest_path = temp.path().join("Cargo.toml");
        fs::write(&manifest_path, "[package]\nname = \"dummy\"\n").unwrap();

        // Create an examples directory with a dummy example file.
        let examples_dir = temp.path().join("examples");
        fs::create_dir(&examples_dir).unwrap();
        let example_file = examples_dir.join("example1.rs");
        fs::write(&example_file, "fn main() {}").unwrap();

        let targets = discover_targets(temp.path()).unwrap();
        // Expect at least two targets: one for the manifest and one for the example.
        assert!(targets.len() >= 2);

        let example_target = targets
            .iter()
            .find(|t| t.kind == TargetKind::Example && t.name == "example1");
        assert!(example_target.is_some());
    }
}
