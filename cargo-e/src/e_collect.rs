use crate::e_target::{CargoTarget, TargetKind, TargetOrigin};
use crate::{e_workspace, prelude::*};
use std::collections::HashMap;
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
) -> Result<Vec<CargoTarget>, Box<dyn Error>> {
    // Run the Cargo command using our helper.
    let output = run_cargo_with_opt_out(&["run", "--bin"], manifest_path)?;
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    debug!("DEBUG {} {} ", prefix, manifest_path.display());
    debug!("DEBUG: stderr (binaries) = {:?}", stderr_str);

    let bin_names = crate::parse_available(&stderr_str, "binaries");

    let binaries = bin_names
        .into_iter()
        .map(|name| {
            let target_kind = if let Some(parent) = manifest_path.parent() {
                if parent.file_name().and_then(|s| s.to_str()) == Some("src-tauri") {
                    TargetKind::ManifestTauri
                } else if extended {
                    TargetKind::ExtendedBinary
                } else {
                    TargetKind::Binary
                }
            } else if extended {
                TargetKind::ExtendedBinary
            } else {
                TargetKind::Binary
            };

            let display_name = if prefix.starts_with('$') {
                format!("{} > binary > {}", prefix, name)
            } else if extended {
                format!("{} {}", prefix, name)
            } else if prefix.starts_with("builtin") {
                format!("builtin binary: {}", name)
            } else {
                format!("{} {}", prefix, name)
                // name.clone()
            };
            CargoTarget {
                name: name.clone(),
                display_name,
                manifest_path: manifest_path.into(),
                kind: target_kind,
                extended,
                toml_specified: false,
                origin: Some(TargetOrigin::SubProject(manifest_path.to_path_buf())),
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
) -> Result<Vec<CargoTarget>, Box<dyn Error>> {
    // Run the Cargo command using our helper.
    let output = run_cargo_with_opt_out(&["run", "--example"], manifest_path)?;
    debug!("DEBUG {} {} ", prefix, manifest_path.display());
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    debug!("DEBUG: stderr (examples) = {:?}", stderr_str);

    let names = crate::parse_available(&stderr_str, "examples");
    if names.len() > 0 {
        debug!("DEBUG: example names = {:?}", names);
    }

    let examples = names
        .into_iter()
        .map(|name| {
            let target_kind = if extended {
                TargetKind::ExtendedExample
            } else {
                TargetKind::Example
            };

            let display_name = if prefix.starts_with('$') {
                format!("{} > example > {}", prefix, name)
            } else if extended {
                format!("{} {}", prefix, name)
            } else if prefix.starts_with("builtin") {
                format!("builtin example: {}", name)
            } else {
                format!("{} {}", prefix, name)
            };
            let target = CargoTarget {
                name: name.clone(),
                display_name,
                manifest_path: manifest_path.into(),
                kind: target_kind,
                extended,
                toml_specified: true,
                origin: Some(TargetOrigin::SubProject(manifest_path.to_path_buf())),
            };
            target
        })
        .collect();

    Ok(examples)
}

// --- Concurrent or sequential collection ---
pub fn collect_samples(
    _workspace_mode: bool,
    manifest_infos: Vec<(String, PathBuf, bool)>,
    __max_concurrency: usize,
) -> Result<Vec<CargoTarget>, Box<dyn Error>> {
    let mut all_samples = Vec::new();

    #[cfg(feature = "concurrent")]
    {
        use threadpool::ThreadPool;
        let pool = ThreadPool::new(__max_concurrency);
        let (tx, rx) = mpsc::channel();

        let start_concurrent = Instant::now();
        for (_prefix, manifest_path, _extended) in manifest_infos {
            let tx = tx.clone();
            let manifest_clone = manifest_path.clone();
            pool.execute(move || {
                // Retrieve the runnable targets from the manifest.
                let (bins, examples, benches, tests) =
                    crate::e_manifest::get_runnable_targets(&manifest_clone).unwrap_or_default();

                crate::e_manifest::get_runnable_targets(&manifest_clone).unwrap_or_default();
                // If there are no examples or binaries, return early.
                if bins.is_empty() && examples.is_empty() {
                    return;
                }

                // Combine all targets.
                let all_targets = bins
                    .into_iter()
                    .chain(examples)
                    .chain(benches)
                    .chain(tests)
                    .collect::<Vec<_>>();

                // Now refine each target using the new pure method.
                // If you implemented it as an associated method `refined()`:

                let refined_targets: Vec<_> = all_targets
                    .into_iter()
                    .map(|t| CargoTarget::refined_target(&t))
                    .collect();
                tx.send(refined_targets).expect("Failed to send results");
                //  let mut results = Vec::new();
                //  results.extend(bins);
                //  results.extend(examples);
                //  results.extend(benches);
                //  results.extend(tests);
                //  tx.send(results).expect("Failed to send results");

                // let mut results = Vec::new();
                // if let Ok(mut ex) = collect_examples(&prefix_clone, &manifest_clone, extended) {
                //     results.append(&mut ex);
                // }
                // if let Ok(mut bins) = collect_binaries(&prefix_clone, &manifest_clone, extended) {
                //     results.append(&mut bins);
                // }
                // let etargets =
                // crate::e_discovery::discover_targets(manifest_clone.parent().unwrap()).unwrap();
                // for target in etargets {
                //     let m = target.manifest_path.clone();
                //     // let manifest_path = Path::new(&m);
                //     // if target.name.contains("html") {
                //     //     std::process::exit(0);
                //     // }
                //     if let Some(existing) = results.iter_mut().find(|r| r.name == target.name) {
                //        debug!("REPLACING {}",target.name);
                //         *existing = target;
                //     } else {
                //         debug!("ADDING {}",target.name);
                //         results.push(target);
                //     }

                //     // Check if the target is extended and that its name is not already present in results.
                //     // if target.extended {
                //     //     let new_prefix = format!("examples/{}", &prefix_clone);
                //     //     // Collect extended examples.
                //     //     if let Ok(ex_ext) = collect_examples(&new_prefix, manifest_path, true) {
                //     //         for ex in ex_ext {
                //     //             if !results.iter().any(|r| r.name == ex.name) {
                //     //                 results.push(ex);
                //     //             }
                //     //         }
                //     //     }

                //     //     // Collect extended binaries.
                //     //     if let Ok(bins_ext) = collect_binaries(&new_prefix, manifest_path, true) {
                //     //         for bin in bins_ext {
                //     //             if !results.iter().any(|r| r.name == bin.name) {
                //     //                 results.push(bin);
                //     //             }
                //     //         }
                //     //     }
                //     // }
                // }
                // tx.send(results).expect("Failed to send results");
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
        for (_prefix, manifest_path, _extended) in manifest_infos {
            let (bins, examples, benches, tests) =
                crate::e_manifest::get_runnable_targets(&manifest_path).unwrap_or_default();

            // Merge all targets into one collection.
            all_samples.extend(bins);
            all_samples.extend(examples);
            all_samples.extend(benches);
            all_samples.extend(tests);

            // if let Ok(mut ex) = collect_examples(&prefix, &manifest_path, extended) {
            //     all_samples.append(&mut ex);
            // }
            // if let Ok(mut bins) = collect_binaries(&prefix, &manifest_path, extended) {
            //     all_samples.append(&mut bins);
            // }
            // let manifest_path = Path::new(&manifest_path);
            // let new_prefix = format!("examples/{}", &prefix);
            // // Collect extended examples.
            // if let Ok(ex_ext) = collect_examples(&new_prefix, manifest_path, true) {
            //     for ex in ex_ext {
            //         if !all_samples.iter().any(|r| r.name == ex.name) {
            //             all_samples.push(ex);
            //         }
            //     }
            // }

            // Collect extended binaries.
            // if let Ok(bins_ext) = collect_binaries(&new_prefix, manifest_path, true) {
            //     for bin in bins_ext {
            //         if !all_samples.iter().any(|r| r.name == bin.name) {
            //             all_samples.push(bin);
            //         }
            //     }
            // }
        }
        let duration_seq = start_seq.elapsed();
        debug!("timing: Sequential processing took {:?}", duration_seq);
    }
    // First, refine all collected targets.
    let initial_targets: Vec<_> = all_samples
        .into_iter()
        .map(|t| CargoTarget::refined_target(&t))
        .collect();

    // Build a HashMap keyed by (manifest, name) to deduplicate targets.
    let mut targets_map: HashMap<(String, String), CargoTarget> = HashMap::new();
    for target in initial_targets.into_iter() {
        let key = CargoTarget::target_key(&target);
        targets_map.entry(key).or_insert(target);
    }

    // Expand subprojects in place.
    CargoTarget::expand_subprojects_in_place(&mut targets_map)?;

    // Finally, collect all unique targets.
    let refined_targets: Vec<CargoTarget> = targets_map.into_values().collect();
    // Now do an additional deduplication pass based on origin and name.
    let deduped_targets = crate::e_target::dedup_targets(refined_targets);
    return Ok(deduped_targets);
    //    return Ok(refined_targets);

    // let mut target_map: std::collections::HashMap<String, CargoTarget> =
    //     std::collections::HashMap::new();

    // // Group targets by name and choose one based on workspace_mode:
    // for target in all_samples {
    //     target_map
    //         .entry(target.name.clone())
    //         .and_modify(|existing| {
    //             if workspace_mode {
    //                 // In workspace mode, extended targets override builtins.
    //                 if target.extended && !existing.extended {
    //                     *existing = target.clone();
    //                 }
    //             } else {
    //                 // In normal mode, builtin targets (non-extended) override extended.
    //                 if !target.extended && existing.extended {
    //                     *existing = target.clone();
    //                 }
    //             }
    //         })
    //         .or_insert(target.clone());
    // }

    // let mut combined = Vec::new();
    // for target in target_map.into_values() {
    //     combined.push(target);
    // }

    // Ok(combined)
    // Ok(all_samples)
}

use log::warn;
use std::fs;
use std::path::Path;

/// Returns the "depth" of a path (number of components)
pub fn path_depth(path: &Path) -> usize {
    path.components().count()
}

/// Deduplicates targets by their canonicalized origin.
/// In particular, if two targets share the same origin key and one is a SingleFile
/// and the other is a DefaultBinary, only the SingleFile target is kept.
pub fn dedup_single_file_over_default_binary(targets: Vec<CargoTarget>) -> Vec<CargoTarget> {
    let mut map: HashMap<Option<String>, CargoTarget> = HashMap::new();

    for target in targets {
        // Try to get a canonicalized string for the target's origin.
        let origin_key = target.origin.as_ref().and_then(|origin| match origin {
            TargetOrigin::SingleFile(path)
            | TargetOrigin::DefaultBinary(path)
            | TargetOrigin::SubProject(path) => fs::canonicalize(path)
                .ok()
                .map(|p| p.to_string_lossy().into_owned()),
            _ => None,
        });

        if let Some(key) = origin_key.clone() {
            if let Some(existing) = map.get(&Some(key.clone())) {
                // If one target is SingleFile and the other is DefaultBinary, keep the SingleFile.
                match (&target.origin, &existing.origin) {
                    (Some(TargetOrigin::SingleFile(_)), Some(TargetOrigin::DefaultBinary(_))) => {
                        map.insert(Some(key), target);
                    }
                    (Some(TargetOrigin::DefaultBinary(_)), Some(TargetOrigin::SingleFile(_))) => {
                        // Do nothing: keep the existing SingleFile.
                    }
                    _ => {
                        // Otherwise, choose the target with a deeper manifest path.
                        let current_depth = path_depth(&target.manifest_path);
                        let existing_depth = path_depth(&existing.manifest_path);
                        if current_depth > existing_depth {
                            map.insert(Some(key), target);
                        }
                    }
                }
            } else {
                map.insert(Some(key), target);
            }
        } else {
            // For targets with no origin key, use None.
            if let Some(existing) = map.get(&None) {
                // Optionally, compare further; here we simply warn.
                warn!(
                    "Duplicate target with no origin: Existing: {:?} vs New: {:?}",
                    existing, target
                );
            } else {
                map.insert(None, target);
            }
        }
    }

    map.into_values().collect()
}

pub fn collect_all_targets(
    use_workspace: bool,
    max_concurrency: usize,
) -> Result<Vec<CargoTarget>, Box<dyn std::error::Error>> {
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
    // Deduplicate targets: if a SingleFile and DefaultBinary share the same origin, keep only the SingleFile.
    let deduped_samples = dedup_single_file_over_default_binary(samples);
    Ok(deduped_samples)
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
