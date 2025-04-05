// src/e_discovery.rs
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::e_target::{CargoTarget, TargetKind};
use anyhow::{anyhow, Context, Result};

pub fn scan_tests_directory(manifest_path: &Path) -> Result<Vec<String>> {
    // Determine the project root from the manifest's parent directory.
    let project_root = manifest_path
        .parent()
        .ok_or_else(|| anyhow!("Unable to determine project root from manifest"))?;

    // Construct the path to the tests directory.
    let tests_dir = project_root.join("tests");
    let mut tests = Vec::new();

    // Only scan if the tests directory exists and is a directory.
    if tests_dir.exists() && tests_dir.is_dir() {
        for entry in fs::read_dir(tests_dir)? {
            let entry = entry?;
            let path = entry.path();
            // Only consider files with a `.rs` extension.
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        if let Some(stem) = path.file_stem() {
                            tests.push(stem.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(tests)
}

pub fn scan_examples_directory(manifest_path: &Path) -> Result<Vec<CargoTarget>> {
    // Determine the project root from the manifest's parent directory.
    let project_root = manifest_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Unable to determine project root"))?;
    let examples_dir = project_root.join("examples");
    let mut targets = Vec::new();

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
                            if let Some(target) = CargoTarget::from_source_file(
                                stem,
                                &path,
                                manifest_path,
                                true,
                                false,
                            ) {
                                targets.push(target);
                            }
                        }
                    }
                }
            } else if path.is_dir() {
                if let Some(target) = CargoTarget::from_folder(&path, &manifest_path, true, true) {
                    if target.kind == TargetKind::Unknown {
                        continue;
                    }
                    targets.push(target);
                }
                // If the directory contains a Cargo.toml, treat it as an extended subproject.
                // let sub_manifest = path.join("Cargo.toml");
                // if sub_manifest.exists() {
                //     // Look for a Tauri or Dioxus configuration.
                //     let tauri_folder = path.join("src-tauri");
                //     let tauri_config = path.join("tauri.conf.json");
                //     let dioxus_config = path.join("Dioxus.toml");

                //     let target_kind = if tauri_folder.exists() || tauri_config.exists() {
                //         TargetKind::ManifestTauri
                //     } else if dioxus_config.exists() {
                //         TargetKind::ManifestDioxus
                //     } else {
                //         // Skip directories that don't match known subproject configurations.
                //         continue;
                //     };

                //     if let Some(name) = path.file_name() {
                //         targets.push(CargoTarget {
                //             name: name.to_string_lossy().to_string(),
                //             display_name: format!("-examples/ {}", name.to_string_lossy()),
                //             manifest_path: sub_manifest.clone(),
                //             kind: target_kind,
                //             extended: true,
                //             origin: Some(TargetOrigin::SubProject(sub_manifest)),
                //         });
                //     }
                // }
            }
        }
    }

    Ok(targets)
}

/// Determines the target kind and (optionally) an updated manifest path based on:
/// - Tauri configuration: If the parent directory of the original manifest contains a
///   "tauri.conf.json", and also a Cargo.toml exists in that same directory, then update the manifest path
///   and return ManifestTauri.
/// - Dioxus markers: If the file contents contain any Dioxus markers, return either ManifestDioxusExample
///   (if `example` is true) or ManifestDioxus.
/// - Otherwise, if the file contains "fn main", decide based on the candidate's parent folder name.
///   If the parent is "examples" (or "bin"), return the corresponding Example/Binary (or extended variant).
/// - If none of these conditions match, return Example as a fallback.
///
/// Returns a tuple of (TargetKind, updated_manifest_path).
pub fn determine_target_kind_and_manifest(
    manifest_path: &Path,
    candidate: &Path,
    file_contents: &str,
    example: bool,
    extended: bool,
    incoming_kind: Option<TargetKind>,
) -> (TargetKind, PathBuf) {
    // Start with the original manifest path.
    let mut new_manifest = manifest_path.to_path_buf();

    // If the incoming kind is already known (Test or Bench), return it.
    if let Some(kind) = incoming_kind {
        if kind == TargetKind::Test || kind == TargetKind::Bench {
            return (kind, new_manifest);
        }
    }
    // Tauri detection: check if the manifest's parent or candidate's parent contains tauri config.
    let tauri_detected = manifest_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().eq_ignore_ascii_case("src-tauri"))
        .unwrap_or(false)
        || manifest_path
            .parent()
            .map(|p| p.join("tauri.conf.json"))
            .map_or(false, |p| p.exists())
        || manifest_path
            .parent()
            .map(|p| p.join("src-tauri"))
            .map_or(false, |p| p.exists())
        || candidate
            .parent()
            .map(|p| p.join("tauri.conf.json"))
            .map_or(false, |p| p.exists());

    if tauri_detected {
        if example {
            return (TargetKind::ManifestTauriExample, new_manifest);
        }
        // If the candidate's parent contains tauri.conf.json, update the manifest path if there's a Cargo.toml there.
        if let Some(candidate_parent) = candidate.parent() {
            let candidate_manifest = candidate_parent.join("Cargo.toml");
            if candidate_manifest.exists() {
                new_manifest = candidate_manifest;
            }
        }
        return (TargetKind::ManifestTauri, new_manifest);
    }

    // Dioxus detection
    if file_contents.contains("dioxus::") {
        let kind = if example {
            TargetKind::ManifestDioxusExample
        } else {
            TargetKind::ManifestDioxus
        };
        return (kind, new_manifest);
    }

    // leptos detection
    if file_contents.contains("leptos::") {
        return (TargetKind::ManifestLeptos, new_manifest);
    }

    // Check if the file contains "fn main"
    if file_contents.contains("fn main") {
        let kind = if example {
            if extended {
                TargetKind::ExtendedExample
            } else {
                TargetKind::Example
            }
        } else if extended {
            TargetKind::ExtendedBinary
        } else {
            TargetKind::Binary
        };
        return (kind, new_manifest);
    }
    // Check if the file contains a #[test] attribute; if so, mark it as a test.
    if file_contents.contains("#[test]") {
        return (TargetKind::Test, new_manifest);
    }

    // Default fallback.
    (TargetKind::Unknown, "errorNOfnMAIN".into())
}

/// Returns true if the candidate file is not located directly in the project root.
pub fn is_extended_target(manifest_path: &Path, candidate: &Path) -> bool {
    if let Some(project_root) = manifest_path.parent() {
        // If the candidate's parent is not the project root, it's nested (i.e. extended).
        candidate
            .parent()
            .map(|p| p != project_root)
            .unwrap_or(false)
    } else {
        false
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::fs;
//     use tempfile::tempdir;

//     #[test]
//     fn test_discover_targets_no_manifest() {
//         let temp = tempdir().unwrap();
//         // With no Cargo.toml, we expect an empty list.
//         let targets = discover_targets(temp.path()).unwrap();
//         assert!(targets.is_empty());
//     }

//     #[test]
//     fn test_discover_targets_with_manifest_and_example() {
//         let temp = tempdir().unwrap();
//         // Create a dummy Cargo.toml.
//         let manifest_path = temp.path().join("Cargo.toml");
//         fs::write(&manifest_path, "[package]\nname = \"dummy\"\n").unwrap();

//         // Create an examples directory with a dummy example file.
//         let examples_dir = temp.path().join("examples");
//         fs::create_dir(&examples_dir).unwrap();
//         let example_file = examples_dir.join("example1.rs");
//         fs::write(&example_file, "fn main() {}").unwrap();

//         let targets = discover_targets(temp.path()).unwrap();
//         // Expect at least two targets: one for the manifest and one for the example.
//         assert!(targets.len() >= 2);

//         let example_target = targets
//             .iter()
//             .find(|t| t.kind == TargetKind::Example && t.name == "example1");
//         assert!(example_target.is_some());
//     }
// }
