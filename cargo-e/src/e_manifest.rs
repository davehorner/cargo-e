use crate::e_target::TargetKind;
use anyhow::{anyhow, Result};
use log::trace;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

/// Locate the Cargo.toml by invoking `cargo locate-project --message-format plain`.
/// If `workspace` is true, the `--workspace` flag is added so that the manifest
/// for the workspace root is returned.
pub fn locate_manifest(workspace: bool) -> Result<String, Box<dyn Error>> {
    let mut args = vec!["locate-project", "--message-format", "plain"];
    if workspace {
        args.push("--workspace");
    }

    let output = Command::new("cargo").args(&args).output()?;
    if !output.status.success() {
        return Err("cargo locate-project failed".into());
    }

    let manifest = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if manifest.is_empty() {
        return Err("No Cargo.toml found".into());
    }
    Ok(manifest)
}

/// Parses the workspace manifest (in TOML format) to return a vector of workspace member names and
/// their corresponding manifest paths. The workspace manifest is expected to have a \[workspace\]
/// table with a "members" array. Each member is joined with the workspace root directory.
pub fn collect_workspace_members(
    workspace_manifest: &str,
) -> Result<Vec<(String, PathBuf)>, Box<dyn Error>> {
    let manifest_path = Path::new(workspace_manifest);
    let workspace_root = manifest_path
        .parent()
        .ok_or("Cannot determine workspace root")?;
    let manifest_contents = fs::read_to_string(workspace_manifest)?;
    let value: Value = manifest_contents.parse::<Value>()?;
    let mut members = Vec::new();

    if let Some(ws) = value.get("workspace") {
        if let Some(member_array) = ws.get("members").and_then(|v| v.as_array()) {
            for member in member_array {
                if let Some(member_str) = member.as_str() {
                    // Strip any trailing glob patterns like "/*".
                    let member_clean = if member_str.contains('*') {
                        member_str.trim_end_matches("/*")
                    } else {
                        member_str
                    };
                    let member_path = workspace_root.join(member_clean);
                    let member_manifest = member_path.join("Cargo.toml");
                    if member_manifest.exists() {
                        members.push((member_clean.to_string(), member_manifest));
                    }
                }
            }
        }
    }
    Ok(members)
}

/// Checks whether the manifest at `manifest_path` would trigger the workspace error.
/// If so, it patches the file by appending an empty `[workspace]` table, returning the original content.
/// Otherwise, returns None.
#[allow(dead_code)]
pub(crate) fn maybe_patch_manifest_for_run(manifest_path: &Path) -> Result<Option<String>> {
    // Run a lightweight command (cargo metadata) to see if the manifest is affected.
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--manifest-path"])
        .arg(manifest_path)
        .output()?;
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let workspace_error_marker = "current package believes it's in a workspace when it's not:";

    if stderr_str.contains(workspace_error_marker) {
        // Read the original manifest content.
        let original = fs::read_to_string(manifest_path)?;
        // If not already opting out, patch it.
        if !original.contains("[workspace]") {
            let patched = format!("{}\n[workspace]\n", original);
            fs::write(manifest_path, &patched)?;
            return Ok(Some(original));
        }
    }
    Ok(None)
}

/// Search upward from the current directory for Cargo.toml.
pub fn find_manifest_dir() -> std::io::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("Cargo.toml").exists() {
            return Ok(dir);
        }
        // Stop if we cannot go any higher.
        if !dir.pop() {
            break;
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not locate Cargo.toml in the current or parent directories.",
    ))
}

/// Searches upward from the given starting directory for a Cargo.toml file
/// and returns the directory containing it.
pub fn find_manifest_dir_from(start: &std::path::Path) -> std::io::Result<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        trace!(
            "{:?} {:?}",
            start.display(),
            dir.join("Cargo.toml").display()
        );
        if dir.join("Cargo.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not locate Cargo.toml in the current or parent directories.",
    ))
}

/// Returns a comma‑separated list of required features for a given target,
/// based on its manifest, target kind, and name. If the target is not found
/// in the given manifest and the manifest is a workspace, its members are searched.
pub fn get_required_features_from_manifest(
    manifest_path: &Path,
    kind: &TargetKind,
    target_name: &str,
) -> Option<String> {
    // Read and parse the manifest file.
    let content = fs::read_to_string(manifest_path).ok()?;
    let value: Value = content.parse().ok()?;

    // Map the TargetKind to the corresponding section in the manifest.
    let section = match kind {
        TargetKind::Example | TargetKind::ExtendedExample => "example",
        TargetKind::Binary | TargetKind::ExtendedBinary => "bin",
        TargetKind::ManifestTauri => "bin",
        TargetKind::ManifestTauriExample => "example",
        TargetKind::ManifestDioxus => "bin",
        TargetKind::ManifestDioxusExample => "example",
        TargetKind::ManifestLeptos => "bin",
        TargetKind::Test => "test",
        TargetKind::Bench => "bench",
        TargetKind::Unknown => "",
        TargetKind::Manifest => "",
    };
    if section.is_empty() {
        return None;
    }
    // Look for the target in the specified section.
    if let Some(targets) = value.get(section).and_then(|v| v.as_array()) {
        for entry in targets {
            if let Some(name) = entry.get("name").and_then(|v| v.as_str()) {
                if name == target_name {
                    if let Some(req_feats) =
                        entry.get("required-features").and_then(|v| v.as_array())
                    {
                        let feats = req_feats
                            .iter()
                            .filter_map(|f| f.as_str())
                            .collect::<Vec<_>>()
                            .join(",");
                        if !feats.is_empty() {
                            return Some(feats);
                        }
                    }
                }
            }
        }
    }

    // If not found and the manifest has a [workspace] table, check each workspace member.
    if value.get("workspace").is_some() {
        // Convert the manifest_path to a &str.
        if let Some(manifest_str) = manifest_path.to_str() {
            if let Ok(members) = collect_workspace_members(manifest_str) {
                for (_, member_manifest_path) in members {
                    if let Some(feats) = get_required_features_from_manifest(
                        &member_manifest_path,
                        kind,
                        target_name,
                    ) {
                        return Some(feats);
                    }
                }
            }
        }
    }
    None
}

/// Finds a candidate name from the manifest using the specified table key.
/// For example, if `table_key` is "bin", it checks for `[[bin]]` entries; if it is "example",
/// it checks for `[[example]]` entries.
pub fn find_candidate_name(
    manifest_toml: &Value,
    table_key: &str,
    candidate: &Path,
    manifest_path: &Path,
) -> Option<String> {
    manifest_toml
        .get(table_key)
        .and_then(|v| v.as_array())
        .map(|entries| {
            let manifest_parent = manifest_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""));
            let candidate_abs = std::fs::canonicalize(candidate).ok();
            // First, try to find an explicit match using the provided "path"
            entries
                .iter()
                .find_map(|entry| {
                    entry
                        .get("path")
                        .and_then(|p| p.as_str())
                        .and_then(|rel_path_str| {
                            entry.get("name").and_then(|n| n.as_str()).and_then(|name| {
                                candidate_abs.as_ref().and_then(|candidate_abs| {
                                    std::fs::canonicalize(manifest_parent.join(rel_path_str))
                                        .ok()
                                        .and_then(|expected_path| {
                                            trace!(
                                                "\nCandidate: {}\nExpected: {:?}\nActual: {:?}",
                                                candidate.display(),
                                                expected_path,
                                                candidate_abs
                                            );
                                            if expected_path == *candidate_abs {
                                                trace!(
                                                    "{} Found matching {} with name: {}",
                                                    candidate.display(),
                                                    table_key,
                                                    name
                                                );
                                                Some(name.to_string())
                                            } else {
                                                None
                                            }
                                        })
                                })
                            })
                        })
                })
                // If no explicit match is found, use the last entry with no "path" as the default
                .or_else(|| {
                    entries
                        .iter()
                        .filter(|entry| entry.get("path").is_none())
                        .filter_map(|entry| {
                            entry.get("name").and_then(|n| n.as_str()).map(String::from)
                        })
                        .last()
                })
        })
        .flatten()
}

/// Returns the runnable targets (bins, examples, benches, and tests) from the Cargo.toml.
/// For tests, it uses `scan_tests_directory` to list integration test files.
use crate::e_target::{CargoTarget, TargetOrigin};

// /// Returns the runnable targets (bins, examples, benches, and tests) from the Cargo.toml.
// /// For examples and tests, it also scans the corresponding directories for files that contain
// /// a main function (for examples) and returns their file paths in the target origin.
// pub fn get_runnable_targets(
//     manifest_path: &Path,
// ) -> Result<(Vec<CargoTarget>, Vec<CargoTarget>, Vec<CargoTarget>, Vec<CargoTarget>), Box<dyn Error>> {
//     // Read and parse the Cargo.toml manifest.
//     let content = fs::read_to_string(manifest_path)?;
//     let value: Value = content.parse()?;

//     // Determine the project root from the manifest's parent directory.
//     let project_root = manifest_path.parent().ok_or("Unable to determine project root")?;

//     // Determine if the manifest is inside an "examples" folder.
//     let is_extended = project_root
//         .parent()
//         .and_then(|p| p.file_name())
//         .map(|s| s.to_string_lossy().eq_ignore_ascii_case("examples"))
//         .unwrap_or(false);

//     // For targets discovered in an extended (examples) context,
//     // use ExtendedExample instead of Binary or Example.
//     let mut bin_kind = if is_extended { TargetKind::ExtendedBinary } else { TargetKind::Binary };
//     let example_kind = if is_extended { TargetKind::ExtendedExample } else { TargetKind::Example };

//     // --- Binaries ---
//     // Start with any explicit [[bin]] targets defined in the manifest.
//     let mut bins: Vec<CargoTarget> = value
//     .get("bin")
//     .and_then(|v| v.as_array())
//     .map(|arr| {
//         arr.iter()
//             .filter_map(|entry| {
//                 // Get the target name.
//                 let name = entry.get("name").and_then(|n| n.as_str())?;
//                 // Use the "path" from the TOML if provided.
//                 let relative_path = entry.get("path").and_then(|p| p.as_str()).unwrap_or("");
//                 Some((name, relative_path))
//             })
//             .map(|(name, relative_path)| {
//                 // Compute the full path to the binary file.
//                 let full_path = if !relative_path.is_empty() {
//                     project_root.join(relative_path)
//                 } else {
//                     // Fallback: assume "src/{name}.rs" if no path is specified.
//                     project_root.join("src").join(format!("{}.rs", name))
//                 };
//                 // Check for Tauri and Dioxus configuration.
//                 let tauri_folder = project_root.join("src-tauri");
//                 let tauri_config = project_root.join("tauri.conf.json");
//                 let dioxus_config = project_root.join("Dioxus.toml");
//                 let mut target_kind = bin_kind; // by default, use bin_kind.
//                 if manifest_path.parent()
//                     .and_then(|p| p.file_name())
//                     .map(|s| s.to_string_lossy().eq_ignore_ascii_case("src-tauri"))
//                     .unwrap_or(false)
//                 {
//                     target_kind = TargetKind::ManifestTauri;
//                 } else if tauri_folder.exists() || tauri_config.exists() {
//                     target_kind = TargetKind::ManifestTauri;
//                 } else if dioxus_config.exists() {
//                     target_kind = TargetKind::ManifestDioxus;
//                 }
//             // Read the file's contents.
//             let file_contents = fs::read_to_string(&full_path).unwrap_or_default();
//             // Determine the target kind based on the file contents.
//             target_kind = if file_contents.contains("LaunchBuilder::new") ||  file_contents.contains("dioxus::LaunchBuilder") || file_contents.contains("dioxus::launch") {
//                 TargetKind::ManifestDioxus
//             } else {
//                 target_kind
//             };

//                 CargoTarget {
//                     name: name.to_string(),
//                     display_name: name.to_string(),
//                     // We keep the manifest_path as the package's manifest.
//                     manifest_path: manifest_path.to_path_buf(),
//                     kind: target_kind,
//                     extended: is_extended,
//                     origin: Some(TargetOrigin::SingleFile(full_path)),
//                 }
//             })
//             .collect::<Vec<_>>()
//     })
//     .unwrap_or_default();

//     // Check for the default binary.
//     // First try "src/main.rs", then fallback to "main.rs" at the project root.
//     let default_bin_path = if project_root.join("src").join("main.rs").exists() {
//         project_root.join("src").join("main.rs")
//     } else if project_root.join("main.rs").exists() {
//         project_root.join("main.rs")
//     } else {
//         PathBuf::new()
//     };
//     if !default_bin_path.as_os_str().is_empty() {
//         if let Some(pkg) = value.get("package") {
//             if let Some(pkg_name) = pkg.get("name").and_then(|v| v.as_str()) {
//             // Determine the default target kind.
//             let tauri_folder = project_root.join("src-tauri");
//             let tauri_config = project_root.join("tauri.conf.json");
//             let dioxus_config = project_root.join("Dioxus.toml");

//             // Start with fallback kind.
//             let mut default_kind = bin_kind;

//             // If the parent directory of the manifest is named "src-tauri", use ManifestTauri.
//             if manifest_path.parent()
//                 .and_then(|p| p.file_name())
//                 .map(|s| s.to_string_lossy().eq_ignore_ascii_case("src-tauri"))
//                 .unwrap_or(false)
//             {
//                 default_kind = TargetKind::ManifestTauri;
//             } else if tauri_folder.exists() || tauri_config.exists() {
//                 default_kind = TargetKind::ManifestTauri;
//             } else if dioxus_config.exists() {
//                 default_kind = TargetKind::ManifestDioxus;
//             }

//             // Read the file's contents.
//             let file_contents = fs::read_to_string(&default_bin_path).unwrap_or_default();
//             // Determine the target kind based on the file contents.
//             default_kind = if file_contents.contains("LaunchBuilder::new") || file_contents.contains("dioxus::LaunchBuilder") || file_contents.contains("dioxus::launch") {
//                 TargetKind::ManifestDioxus
//             } else {
//                 default_kind
//             };

//             // Add the default target if it isn’t already in the bins vector.
//             if !bins.iter().any(|t| t.name == pkg_name) {
//                 bins.push(CargoTarget {
//                     name: pkg_name.to_string(),
//                     display_name: pkg_name.to_string(),
//                     manifest_path: manifest_path.to_path_buf(),
//                     kind: default_kind,
//                     extended: is_extended,
//                     origin: Some(TargetOrigin::DefaultBinary(default_bin_path)),
//                 });
//             }
//             }
//         }
//     }

//     // Also scan the src/bin directory for additional binary targets.
//     let bin_dir = project_root.join("src").join("bin");
//     if bin_dir.exists() && bin_dir.is_dir() {
//         for entry in fs::read_dir(&bin_dir)? {
//             let entry = entry?;
//             let path = entry.path();
//             if path.is_file() {
//                 if let Some(ext) = path.extension() {
//                     if ext == "rs" {
//                         if let Some(stem) = path.file_stem() {
//                             let bin_name = stem.to_string_lossy().to_string();

//                             // Read the file's contents.
//                             let file_contents = fs::read_to_string(&path).unwrap_or_default();
//                             // Determine the target kind based on the file contents.
//                             bin_kind = if file_contents.contains("LaunchBuilder::new") || file_contents.contains("dioxus::LaunchBuilder") || file_contents.contains("dioxus::launch") {
//                                 TargetKind::ManifestDioxus
//                             } else {
//                                 bin_kind
//                             };

//                             // Only add it if a target with the same name isn't already present.
//                             if !bins.iter().any(|t| t.name == bin_name) {
//                                 bins.push(CargoTarget {
//                                     name: bin_name.clone(),
//                                     display_name: bin_name,
//                                     manifest_path: manifest_path.to_path_buf(),
//                                     kind: bin_kind,
//                                     extended: is_extended,
//                                     origin: Some(TargetOrigin::SingleFile(path)),
//                                 });
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     // --- Examples ---
//     // Get any explicit [[example]] targets from the manifest.
//     let mut examples: Vec<CargoTarget> = value
//     .get("example")
//     .and_then(|v| v.as_array())
//     .map(|arr| {
//         arr.iter()
//             .filter_map(|entry| {
//                 // Get the target name.
//                 let name = entry.get("name").and_then(|n| n.as_str())?;
//                 // Use the "path" field if provided; otherwise assume "examples/{name}.rs"
//                 let relative_path_str = if let Some(p) = entry.get("path").and_then(|p| p.as_str()) {
//                     p.to_string()
//                 } else {
//                     format!("examples/{}.rs", name)
//                 };
//                 let full_path = project_root.join(&relative_path_str);
//                 // Read the file's contents (if the file exists).
//                 let file_contents = fs::read_to_string(&full_path).unwrap_or_default();
//                 // Start with the default example kind.
//                 let mut target_kind = example_kind;
//                 // Check for Dioxus markers.
//                 if file_contents.contains("dioxus::LaunchBuilder")
//                     || file_contents.contains("dioxus::launch")
//                 {
//                     target_kind = TargetKind::ManifestDioxusExample;
//                 } else {
//                     // Check for Tauri configuration in the workspace root.
//                     let tauri_folder = project_root.join("src-tauri");
//                     let tauri_config = project_root.join("tauri.conf.json");
//                     if tauri_folder.exists() || tauri_config.exists() {
//                         target_kind = TargetKind::ManifestTauri;
//                     }
//                 }
//                 Some(CargoTarget {
//                     name: name.to_string(),
//                     display_name: name.to_string(),
//                     manifest_path: manifest_path.to_path_buf(),
//                     kind: target_kind,
//                     extended: is_extended,
//                     origin: Some(TargetOrigin::SingleFile(full_path)),
//                 })
//             })
//             .collect::<Vec<_>>()
//     })
//     .unwrap_or_default();

//     // Scan the examples/ directory for example targets.
//     let scanned_examples = crate::e_discovery::scan_examples_directory(manifest_path)?;
//     for ex in scanned_examples {
//         if !examples.iter().any(|t| t.name == ex.name) {
//             // If our manifest is inside an examples directory, mark as extended.
//             let mut target = ex;
//             if is_extended {
//                 target.kind = TargetKind::ExtendedExample;
//                 target.extended = true;
//             }
//             examples.push(target);
//         }
//     }

//     // --- Benches ---
//     let benches: Vec<CargoTarget> = value
//         .get("bench")
//         .and_then(|v| v.as_array())
//         .map(|arr| {
//             arr.iter()
//                 .filter_map(|entry| entry.get("name").and_then(|n| n.as_str()))
//                 .map(|name| CargoTarget {
//                     name: name.to_string(),
//                     display_name: name.to_string(),
//                     manifest_path: manifest_path.to_path_buf(),
//                     kind: TargetKind::Bench,
//                     extended: false,
//                     origin: None,
//                 })
//                 .collect::<Vec<_>>()
//         })
//         .unwrap_or_default();

//     // --- Tests ---
//     let scanned_tests = crate::e_discovery::scan_tests_directory(manifest_path)?;
//     let mut tests: Vec<CargoTarget> = Vec::new();
//     for test_name in scanned_tests {
//         let test_path = project_root.join("tests").join(format!("{}.rs", test_name));
//         tests.push(CargoTarget {
//             name: test_name.clone(),
//             display_name: test_name,
//             manifest_path: manifest_path.to_path_buf(),
//             kind: TargetKind::Test,
//             extended: false,
//             origin: Some(TargetOrigin::SingleFile(test_path)),
//         });
//     }

//     Ok((bins, examples, benches, tests))
// }

/// Returns the runnable targets (bins, examples, benches, and tests) from the Cargo.toml.
/// This version uses the new associated constructors on CargoTarget:
/// - `from_source_file`: builds a target from a candidate file (that contains "fn main" and/or special markers).
/// - `from_folder`: builds a target by scanning a folder for a candidate source file.
pub fn get_runnable_targets(
    manifest_path: &Path,
) -> anyhow::Result<(
    Vec<CargoTarget>,
    Vec<CargoTarget>,
    Vec<CargoTarget>,
    Vec<CargoTarget>,
)> {
    // Read and parse the Cargo.toml manifest.
    let content = fs::read_to_string(manifest_path)?;
    let value: Value = content.parse()?;

    // Determine the project root from the manifest's parent directory.
    let project_root = manifest_path
        .parent()
        .ok_or(anyhow!("Unable to determine project root"))?;

    // Determine if the manifest is inside an "examples" folder.
    let is_extended = project_root
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().eq_ignore_ascii_case("examples"))
        .unwrap_or(false);

    // --- Binaries ---
    let mut bins = Vec::new();
    if let Some(bin_array) = value.get("bin").and_then(|v| v.as_array()) {
        for entry in bin_array {
            if let Some(name) = entry.get("name").and_then(|v| v.as_str()) {
                // If a "path" field is provided, use it.
                let target_opt = if let Some(path_str) = entry.get("path").and_then(|v| v.as_str())
                {
                    let candidate = project_root.join(path_str);
                    if candidate.is_file() {
                        CargoTarget::from_source_file(
                            OsStr::new(name),
                            &candidate,
                            manifest_path,
                            false,
                            false,
                        )
                    } else if candidate.is_dir() {
                        CargoTarget::from_folder(&candidate, manifest_path, false, true)
                    } else {
                        None
                    }
                } else {
                    // Fallback: assume the file is at "src/{name}.rs"
                    let candidate = project_root.join("src").join(format!("{}.rs", name));
                    CargoTarget::from_source_file(
                        OsStr::new(name),
                        &candidate,
                        manifest_path,
                        false,
                        false,
                    )
                };
                if let Some(target) = target_opt {
                    bins.push(target);
                }
            }
        }
    }

    // Default binary: if no explicit bin exists with the package name.
    if let Some(pkg) = value
        .get("package")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
    {
        if !bins.iter().any(|t| t.name == pkg) {
            // Candidate: try "src/main.rs", then "main.rs".
            let candidate = if project_root.join("src").join("main.rs").exists() {
                project_root.join("src").join("main.rs")
            } else if project_root.join("main.rs").exists() {
                project_root.join("main.rs")
            } else {
                PathBuf::new()
            };
            if !candidate.as_os_str().is_empty() {
                let candidate = fs::canonicalize(&candidate).unwrap_or(candidate.to_path_buf());
                if let Some(mut target) = CargoTarget::from_source_file(
                    OsStr::new(pkg),
                    &candidate,
                    manifest_path,
                    false,
                    false,
                ) {
                    // Mark this as a default binary.
                    // target.name = pkg.to_string();
                    // target.display_name = pkg.to_string();
                    target.origin = Some(TargetOrigin::DefaultBinary(candidate));
                    bins.push(target);
                }
            }
        }
    }

    // Also, scan the "src/bin" directory for additional binaries.
    let bin_dir = project_root.join("src").join("bin");
    if bin_dir.exists() && bin_dir.is_dir() {
        for entry in fs::read_dir(&bin_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Some(stem) = path.file_stem() {
                    let name = stem.to_string_lossy().to_string();
                    if !bins.iter().any(|t| t.name == name) {
                        if let Some(target) =
                            CargoTarget::from_source_file(stem, &path, manifest_path, false, false)
                        {
                            bins.push(target);
                        }
                    }
                }
            }
        }
    }

    // --- Examples ---
    let mut examples = Vec::new();
    if let Some(example_array) = value.get("example").and_then(|v| v.as_array()) {
        for entry in example_array {
            if let Some(name) = entry.get("name").and_then(|v| v.as_str()) {
                let target_opt = if let Some(path_str) = entry.get("path").and_then(|v| v.as_str())
                {
                    let candidate = project_root.join(path_str);
                    CargoTarget::from_source_file(
                        OsStr::new(name),
                        &candidate,
                        manifest_path,
                        true,
                        false,
                    )
                } else {
                    let candidate = project_root.join(format!("examples/{}.rs", name));
                    CargoTarget::from_source_file(
                        OsStr::new(name),
                        &candidate,
                        manifest_path,
                        true,
                        false,
                    )
                };
                if let Some(target) = target_opt {
                    examples.push(target);
                }
            }
        }
    }
    // Scan the examples directory for additional example targets.
    let scanned_examples = crate::e_discovery::scan_examples_directory(manifest_path, "examples")?;
    for ex in scanned_examples {
        if !examples.iter().any(|t| t.name == ex.name) {
            let mut t = ex;
            if is_extended {
                t.kind = TargetKind::ExtendedExample;
                t.extended = true;
            }
            examples.push(t);
        }
    }
    let scanned_examples =
        crate::e_discovery::scan_examples_directory(manifest_path, "experiments")?;
    for ex in scanned_examples {
        if !examples.iter().any(|t| t.name == ex.name) {
            let mut t = ex;
            if is_extended {
                t.kind = TargetKind::ExtendedExample;
                t.extended = true;
            }
            examples.push(t);
        }
    }

    // --- Benches ---
    let mut benches = Vec::new();
    if let Some(bench_array) = value.get("bench").and_then(|v| v.as_array()) {
        for entry in bench_array {
            if let Some(name) = entry.get("name").and_then(|v| v.as_str()) {
                benches.push(CargoTarget {
                    name: name.to_string(),
                    display_name: name.to_string(),
                    manifest_path: manifest_path.to_path_buf(),
                    kind: TargetKind::Bench,
                    extended: false,
                    toml_specified: false,
                    origin: None,
                });
            }
        }
    }

    // --- Tests ---
    let mut tests = Vec::new();
    let scanned_tests = crate::e_discovery::scan_tests_directory(manifest_path)?;
    for test_name in scanned_tests {
        let candidate = project_root.join("tests").join(format!("{}.rs", test_name));
        tests.push(CargoTarget {
            name: test_name.clone(),
            display_name: test_name,
            manifest_path: manifest_path.to_path_buf(),
            kind: TargetKind::Test,
            extended: false,
            toml_specified: false,
            origin: Some(TargetOrigin::SingleFile(candidate)),
        });
    }

    Ok((bins, examples, benches, tests))
}
