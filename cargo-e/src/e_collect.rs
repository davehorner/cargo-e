use crate::{e_workspace, prelude::*};
use crate::{Example, TargetKind};
use std::process::Output;
/// Helper function that runs a Cargo command with a given manifest path.
/// If it detects the workspace error, it temporarily patches the manifest (by
/// appending an empty `[workspace]` table), re-runs the command, and then restores
/// the original file.
fn run_cargo_with_opt_out(args: &[&str], manifest_path: &Path) -> Result<Output, Box<dyn Error>> {
    // Run the initial command.
    let output = Command::new("cargo")
        .args(args)
        .arg("--manifest-path")
        .arg(manifest_path)
        .output()?;

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let workspace_error_marker = "current package believes it's in a workspace when it's not:";

    // If we detect the workspace error, patch the manifest.
    if stderr_str.contains(workspace_error_marker) {
        // Backup the original manifest.
        let original = fs::read_to_string(manifest_path)?;

        // Only patch if the manifest doesn't already opt out.
        if !original.contains("[workspace]") {
            // Append an empty [workspace] table.
            let patched = format!("{}\n[workspace]\n", original);
            fs::write(manifest_path, &patched)?;

            // Re-run the command with the patched manifest.
            let patched_output = Command::new("cargo")
                .args(args)
                .arg("--manifest-path")
                .arg(manifest_path)
                .output()?;

            // Restore the original manifest.
            fs::write(manifest_path, original)?;

            return Ok(patched_output);
        }
    }

    Ok(output)
}

/// Runs `cargo run --bin` (without specifying a binary name) so that Cargo prints an error with
/// a list of available binary targets. Then parses that list to return a vector of Example instances,
/// using the provided prefix.
pub fn collect_binaries(
    prefix: &str,
    manifest_path: &Path,
    extended: bool,
) -> Result<Vec<Example>, Box<dyn Error>> {
    // Run the Cargo command using our helper.
    let output = run_cargo_with_opt_out(&["run", "--bin"], manifest_path)?;
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    let bin_names = crate::parse_available(&stderr_str, "binaries");

    let binaries = bin_names
        .into_iter()
        .map(|name| {
            let display_name = if prefix.starts_with('$') {
                format!(
                    "{} > binary > {} {}",
                    prefix,
                    name,
                    manifest_path.to_string_lossy().into_owned()
                )
            } else if extended {
                format!("{} {}", prefix, name)
            } else if prefix.starts_with("builtin") {
                format!("builtin binary: {}", name)
            } else {
                format!("{} {}", prefix, name)
                // name.clone()
            };

            Example {
                name: name.clone(),
                display_name,
                manifest_path: manifest_path.to_string_lossy().to_string(),
                kind: if extended {
                    TargetKind::ExtendedBinary
                } else {
                    TargetKind::Binary
                },
                extended,
            }
        })
        .collect();

    Ok(binaries)
}

/// Runs `cargo run --example` so that Cargo lists available examples,
/// then parses the stderr output to return a vector of Example instances.
pub fn collect_examples(
    prefix: &str,
    manifest_path: &Path,
    extended: bool,
) -> Result<Vec<Example>, Box<dyn Error>> {
    // Run the Cargo command using our helper.
    let output = run_cargo_with_opt_out(&["run", "--example"], manifest_path)?;
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    debug!("DEBUG: stderr (examples) = {:?}", stderr_str);

    let names = crate::parse_available(&stderr_str, "examples");
    debug!("DEBUG: example names = {:?}", names);

    let examples = names
        .into_iter()
        .map(|name| {
            let display_name = if prefix.starts_with('$') {
                format!("{} > example > {}", prefix, name)
            } else if extended {
                format!("{} {}", prefix, name)
            } else if prefix.starts_with("builtin") {
                format!("builtin example: {}", name)
            } else {
                format!("{} {}", prefix, name)
                // name.clone()
            };

            Example {
                name: name.clone(),
                display_name,
                manifest_path: manifest_path.to_string_lossy().to_string(),
                kind: if extended {
                    TargetKind::ExtendedExample
                } else {
                    TargetKind::Example
                },
                extended,
            }
        })
        .collect();

    Ok(examples)
}

// --- Concurrent or sequential collection ---
pub fn collect_samples(
    workspace_mode: bool,
    manifest_infos: Vec<(String, PathBuf, bool)>,
    __max_concurrency: usize,
) -> Result<Vec<Example>, Box<dyn Error>> {
    let mut all_samples = Vec::new();

    #[cfg(feature = "concurrent")]
    {
        use threadpool::ThreadPool;
        let pool = ThreadPool::new(__max_concurrency);
        let (tx, rx) = mpsc::channel();

        let start_concurrent = Instant::now();
        for (prefix, manifest_path, extended) in manifest_infos {
            let tx = tx.clone();
            let prefix_clone = prefix.clone();
            let manifest_clone = manifest_path.clone();
            pool.execute(move || {
                let mut results = Vec::new();
                if let Ok(mut ex) = collect_examples(&prefix_clone, &manifest_clone, extended) {
                    results.append(&mut ex);
                }
                if let Ok(mut bins) = collect_binaries(&prefix_clone, &manifest_clone, extended) {
                    results.append(&mut bins);
                }
                let etargets =
                    crate::e_discovery::discover_targets(manifest_clone.parent().unwrap()).unwrap();
                for target in etargets {
                    // Check if the target is extended and that its name is not already present in results.
                    if target.extended && !results.iter().any(|r| r.name == target.name) {
                        let manifest_path = Path::new(&target.manifest_path);
                        let new_prefix = format!("examples/{}", &prefix_clone);
                        // Collect extended examples.
                        if let Ok(ex_ext) = collect_examples(&new_prefix, manifest_path, true) {
                            for ex in ex_ext {
                                if !results.iter().any(|r| r.name == ex.name) {
                                    results.push(ex);
                                }
                            }
                        }

                        // Collect extended binaries.
                        if let Ok(bins_ext) = collect_binaries(&new_prefix, manifest_path, true) {
                            for bin in bins_ext {
                                if !results.iter().any(|r| r.name == bin.name) {
                                    results.push(bin);
                                }
                            }
                        }
                        // Derive the subproject directory from the extended target's manifest path.

                        // // Convert CargoTarget to Example by mapping the fields appropriately.
                        // let example = Example {
                        //     name: target.name,
                        //     display_name: target.display_name,
                        //     manifest_path: target.manifest_path,
                        //     kind: TargetKind::Extended, // Ensure this field is compatible with Example's type.
                        //     extended: target.extended,
                        // };
                        // results.push(example);
                    }
                }
                // for e in etargets {
                //    println!("{} found", e.name);
                // }
                tx.send(results).expect("Failed to send results");
            });
        }
        drop(tx);
        pool.join(); // Wait for all tasks to finish.
        let duration_concurrent = start_concurrent.elapsed();
        debug!(
            "timing: {} threads took {:?}",
            __max_concurrency, duration_concurrent
        );

        for samples in rx {
            all_samples.extend(samples);
        }
    }

    // Sequential fallback: process one manifest at a time.
    #[cfg(not(feature = "concurrent"))]
    {
        let start_seq = Instant::now();
        for (prefix, manifest_path, extended) in manifest_infos {
            if let Ok(mut ex) = collect_examples(&prefix, &manifest_path, extended) {
                all_samples.append(&mut ex);
            }
            if let Ok(mut bins) = collect_binaries(&prefix, &manifest_path, extended) {
                all_samples.append(&mut bins);
            }
            let manifest_path = Path::new(&manifest_path);
            let new_prefix = format!("examples/{}", &prefix);
            // Collect extended examples.
            if let Ok(ex_ext) = collect_examples(&new_prefix, manifest_path, true) {
                for ex in ex_ext {
                    if !all_samples.iter().any(|r| r.name == ex.name) {
                        all_samples.push(ex);
                    }
                }
            }

            // Collect extended binaries.
            if let Ok(bins_ext) = collect_binaries(&new_prefix, manifest_path, true) {
                for bin in bins_ext {
                    if !all_samples.iter().any(|r| r.name == bin.name) {
                        all_samples.push(bin);
                    }
                }
            }
        }
        let duration_seq = start_seq.elapsed();
        debug!("timing: Sequential processing took {:?}", duration_seq);
    }
    let mut target_map: std::collections::HashMap<String, crate::Example> =
        std::collections::HashMap::new();

    // Group targets by name and choose one based on workspace_mode:
    for target in all_samples {
        target_map
            .entry(target.name.clone())
            .and_modify(|existing| {
                if workspace_mode {
                    // In workspace mode, extended targets override builtins.
                    if target.extended && !existing.extended {
                        *existing = target.clone();
                    }
                } else {
                    // In normal mode, builtin targets (non-extended) override extended.
                    if !target.extended && existing.extended {
                        *existing = target.clone();
                    }
                }
            })
            .or_insert(target.clone());
    }

    let mut combined = Vec::new();
    for target in target_map.into_values() {
        combined.push(target);
    }

    Ok(combined)
    // Ok(all_samples)
}

pub fn collect_all_targets(
    use_workspace: bool,
    max_concurrency: usize,
) -> Result<Vec<Example>, Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    let mut manifest_infos: Vec<(String, PathBuf, bool)> = Vec::new();

    // Locate the package manifest in the current directory.
    let bi = PathBuf::from(crate::locate_manifest(false)?);
    // We're in workspace mode if the flag is set or if the current Cargo.toml is a workspace manifest.
    let in_workspace = use_workspace || e_workspace::is_workspace_manifest(bi.as_path());

    if in_workspace {
        // Use an explicit workspace manifest if requested; otherwise, assume the current Cargo.toml is the workspace manifest.
        let ws = if use_workspace {
            PathBuf::from(crate::locate_manifest(true)?)
        } else {
            bi.clone()
        };
        // Get workspace members (each member's Cargo.toml) using your helper.
        let ws_members =
            e_workspace::get_workspace_member_manifest_paths(ws.as_path()).unwrap_or_default();
        // Build a numbered list of member names (using just the member directory name).
        let member_displays: Vec<String> = ws_members
            .iter()
            .enumerate()
            .map(|(i, (member, _))| format!("{}. {}", i + 1, member))
            .collect();
        // Print the workspace line: "<workspace_root>/<package>/Cargo.toml [1. member, 2. member, ...]"
        println!(
            "workspace: {} [{}]",
            format_workspace(&ws, &bi),
            member_displays.join(", ")
        );
        // Always print the package line.
        println!("package: {}", format_package(&bi));
        manifest_infos.push(("-".to_string(), bi.clone(), false));
        for (member, member_manifest) in ws_members {
            manifest_infos.push((format!("${}", member), member_manifest, true));
        }
    } else {
        // Not in workspace mode: simply print the package manifest.
        println!("package: {}", format_package(&bi));
        manifest_infos.push(("-".to_string(), bi.clone(), false));
    }

    let samples = collect_samples(use_workspace, manifest_infos, max_concurrency)?;
    Ok(samples)
}

// Formats the package manifest as "<package>/Cargo.toml"
fn format_package(manifest: &Path) -> String {
    let pkg = manifest
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    let file = manifest.file_name().unwrap().to_string_lossy();
    format!("{}/{}", pkg, file)
}

// Formats the workspace manifest as "<workspace_root>/<package>/Cargo.toml"
fn format_workspace(ws_manifest: &Path, bi: &Path) -> String {
    let ws_root = ws_manifest.parent().expect("No workspace root");
    let ws_name = ws_root
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    let bi_pkg = bi
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    format!(
        "{}/{}/{}",
        ws_name,
        bi_pkg,
        ws_manifest.file_name().unwrap().to_string_lossy()
    )
}
