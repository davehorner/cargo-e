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
/// Parses available binaries and examples from a given input string (e.g., from stdin),
/// and returns a vector of CargoTarget instances for each found binary and example.
pub fn collect_stdin_available(
    prefix: &str,
    manifest_path: &Path,
    input: &str,
    extended: bool,
) -> Vec<CargoTarget> {
    let bin_names = crate::parse_available(input, "binaries");
    let example_names = crate::parse_available(input, "examples");

    let mut targets = Vec::new();

    targets.extend(bin_names.into_iter().map(|name| {
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
        };
        CargoTarget {
            name: name.clone(),
            display_name,
            manifest_path: manifest_path.into(),
            kind: target_kind,
            extended,
            toml_specified: true,
            origin: Some(TargetOrigin::TomlSpecified(manifest_path.to_path_buf())),
        }
    }));

    targets.extend(example_names.into_iter().map(|name| {
        let target_kind = if let Some(parent) = manifest_path.parent() {
            if parent.file_name().and_then(|s| s.to_str()) == Some("src-tauri") {
                TargetKind::ManifestTauri
            } else if extended {
                TargetKind::ExtendedExample
            } else {
                TargetKind::Example
            }
        } else if extended {
            TargetKind::ExtendedExample
        } else {
            TargetKind::Example
        };
        let display_name = name.clone();
        CargoTarget {
            name: name.clone(),
            display_name,
            manifest_path: manifest_path.into(),
            kind: target_kind,
            extended,
            toml_specified: true,
            origin: Some(TargetOrigin::TomlSpecified(manifest_path.to_path_buf())),
        }
    }));

    targets
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
            // let target_kind = TargetKind::Binary;

            // let display_name = name.clone();
            //  format!("builtin binary: {}", name);
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
                toml_specified: true,
                origin: Some(TargetOrigin::TomlSpecified(manifest_path.to_path_buf())),
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
            // let target_kind = if extended {
            //     TargetKind::ExtendedExample
            // } else {
            //     TargetKind::Example
            // };
            // let target_kind = TargetKind::Example;
            let target_kind = if let Some(parent) = manifest_path.parent() {
                if parent.file_name().and_then(|s| s.to_str()) == Some("src-tauri") {
                    TargetKind::ManifestTauri
                } else if extended {
                    TargetKind::ExtendedExample
                } else {
                    TargetKind::Example
                }
            } else if extended {
                TargetKind::ExtendedExample
            } else {
                TargetKind::Example
            };

            let display_name = name.clone();
            //  format!("builtin example: {}", name);
            // if prefix.starts_with('$') {
            //     format!("{} > example > {}", prefix, name)
            // } else if extended {
            //     format!("{} {}", prefix, name)
            // } else if prefix.starts_with("builtin") {
            //     format!("builtin example: {}", name)
            // } else {
            //     format!("{} {}", prefix, name)
            // };
            let target = CargoTarget {
                name: name.clone(),
                display_name,
                manifest_path: manifest_path.into(),
                kind: target_kind,
                extended,
                toml_specified: true,
                origin: Some(TargetOrigin::TomlSpecified(manifest_path.to_path_buf())),
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
                let prefix_clone = _prefix.clone(); // Define prefix_clone here
                                                    // 1. Collect the builtin stuff
                let mut builtin_examples = Vec::new();
                if let Ok(mut ex) =
                    collect_examples(&prefix_clone, &manifest_clone, _workspace_mode)
                {
                    builtin_examples.append(&mut ex);
                }
                let mut builtin_bins = Vec::new();
                if let Ok(mut bins) =
                    collect_binaries(&prefix_clone, &manifest_clone, _workspace_mode)
                {
                    builtin_bins.append(&mut bins);
                }
                debug!(
                    "DEBUG: {} builtin examples = {:?}",
                    &manifest_clone.display(),
                    builtin_examples
                );
                debug!(
                    "DEBUG: {} builtin binaries = {:?}",
                    &manifest_clone.display(),
                    builtin_bins
                );
                // 2. Get the “official” runnable ones
                let (runnable_bins, runnable_examples, benches, tests) =
                    crate::e_manifest::get_runnable_targets(&manifest_clone).unwrap_or_default();

                // if nothing at all, skip
                if runnable_bins.is_empty()
                    && runnable_examples.is_empty()
                    && builtin_bins.is_empty()
                    && builtin_examples.is_empty()
                {
                    return;
                }

                // 1) Start with all the builtins in order
                let mut bins = builtin_bins;

                // 2) For each runnable:
                //    – if it matches a builtin by name, overwrite that slot
                //    – otherwise push to the end
                for runnable in runnable_bins {
                    if let Some(idx) = bins.iter().position(|b| b.name == runnable.name) {
                        bins[idx] = runnable;
                    } else {
                        bins.push(runnable);
                    }
                }

                let mut examples = builtin_examples;
                for runnable in runnable_examples {
                    if let Some(idx) = examples.iter().position(|e| e.name == runnable.name) {
                        examples[idx] = runnable;
                    } else {
                        examples.push(runnable);
                    }
                }

                // // 3. Merge bins: start with runnable, then override/insert builtin by name
                // let mut bins_map: HashMap<_, _> = runnable_bins
                //     .into_iter()
                //     .map(|bin| (bin.name.clone(), bin))
                //     .collect();
                // // for bin in builtin_bins {
                // //     bins_map.insert(bin.name.clone(), bin);
                // // }
                // let bins: Vec<_> = bins_map.into_values().collect();

                // // 4. Same for examples
                // let mut ex_map: HashMap<_, _> = runnable_examples
                //     .into_iter()
                //     .map(|ex| (ex.name.clone(), ex))
                //     .collect();
                // // for ex in builtin_examples {
                // //     ex_map.insert(ex.name.clone(), ex);
                // // }
                // let examples: Vec<_> = ex_map.into_values().collect();

                debug!("DEBUG: merged examples = {:#?}", examples);
                // 5. Now combine everything
                let all_targets = bins
                    .into_iter()
                    .chain(examples)
                    .chain(benches)
                    .chain(tests)
                    .collect::<Vec<_>>();
                //                 let mut results = Vec::new();
                //                 let prefix_clone = _prefix.clone(); // Define prefix_clone here
                //                 if let Ok(mut ex) = collect_examples(&prefix_clone, &manifest_clone, _workspace_mode) {
                //                      results.append(&mut ex);
                //                 }
                //                 if let Ok(mut bins) = collect_binaries(&prefix_clone, &manifest_clone, _workspace_mode) {
                //                      results.append(&mut bins);
                //                 }
                //                 // Retrieve the runnable targets from the manifest.
                //                 let (bins, examples, benches, tests) =
                //                     crate::e_manifest::get_runnable_targets(&manifest_clone).unwrap_or_default();

                //                 // If there are no examples or binaries, return early.
                //                 if bins.is_empty() && examples.is_empty() {
                //                     return;
                //                 }

                //                 // Combine all targets.
                //                 let all_targets = bins
                //                     .into_iter()
                //                     .chain(results)
                //                     .chain(examples)
                //                     .chain(benches)
                //                     .chain(tests)
                //                     .collect::<Vec<_>>();
                // log::debug!("DEBUG: all_targets = {:?}", all_targets);
                // Now refine each target using the new pure method.
                // If you implemented it as an associated method `refined()`:

                let refined_targets: Vec<_> = all_targets
                    .into_iter()
                    .map(|t| CargoTarget::refined_target(&t))
                    .collect();
                tx.send(refined_targets).expect("Failed to send results");
                // tx.send(all_targets).expect("Failed to send results");
                //  let mut results = Vec::new();
                //  results.extend(bins);
                //  results.extend(examples);
                //  results.extend(benches);
                //  results.extend(tests);
                //  tx.send(results).expect("Failed to send results");

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
            debug!("DEBUG: samples = {:#?}", samples);
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
    // for target in initial_targets.into_iter() {
    //     let key = CargoTarget::target_key(&target);
    //     targets_map.entry(key).or_insert(target);
    // }
    for target in initial_targets {
        let key = CargoTarget::target_key(&target);
        targets_map
            .entry(key)
            .and_modify(|existing| {
                // inline match‐upgrading, wrapped in dbg! to print old, new and result
                existing.kind = match (&existing.kind, &target.kind) {
                    (TargetKind::Example, TargetKind::ExtendedExample) => {
                        // you had Example, new is ExtendedExample → upgrade
                        target.kind.clone()
                    }
                    (TargetKind::Binary, TargetKind::ExtendedBinary) => target.kind.clone(),
                    (_, TargetKind::ManifestTauriExample) | (_, TargetKind::ManifestTauri) => {
                        // println!("DEBUG: Tauri {}", target.name);
                        target.kind.clone()
                    }
                    // your custom case: anything → Dioxius
                    (_, TargetKind::ManifestDioxus) => {
                        // println!("DEBUG: Dioxus {}", target.name);
                        target.kind.clone()
                    }
                    (_, TargetKind::ManifestDioxusExample) => {
                        // println!("DEBUG: DioxusExample {}", target.name);
                        target.kind.clone()
                    }
                    // else keep old
                    (old_kind, _) => old_kind.clone(),
                };
            })
            .or_insert(target);
    }

    // Expand subprojects in place.
    CargoTarget::expand_subprojects_in_place(&mut targets_map)?;

    // Finally, collect all unique targets.
    let refined_targets: Vec<CargoTarget> = targets_map.into_values().collect();
    // Now do an additional deduplication pass based on origin and name.
    let deduped_targets = crate::e_target::dedup_targets(refined_targets);
    Ok(deduped_targets)
    //  return Ok(refined_targets);

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

use std::fs;
use std::path::Path;

/// Returns the "depth" of a path (number of components)
pub fn path_depth(path: &Path) -> usize {
    path.components().count()
}

/// Deduplicates targets by their canonicalized origin, gives priority to `toml_specified`,
/// and ensures single-file targets override default binaries when appropriate.
pub fn dedup_single_file_over_default_binary(targets: Vec<CargoTarget>) -> Vec<CargoTarget> {
    let mut map: HashMap<Option<String>, CargoTarget> = HashMap::new();

    for target in targets {
        // Compute canonical origin key
        let origin_key = target.origin.as_ref().and_then(|origin| match origin {
            TargetOrigin::SingleFile(path)
            | TargetOrigin::DefaultBinary(path)
            | TargetOrigin::SubProject(path) => fs::canonicalize(path)
                .ok()
                .map(|p| p.to_string_lossy().into_owned()),
            _ => None,
        });

        // Use Some(key) or None as map key
        let entry_key = origin_key.clone();

        if let Some(existing) = map.get(&entry_key) {
            // 1) Prioritize toml_specified
            if target.toml_specified && !existing.toml_specified {
                map.insert(entry_key.clone(), target);
                continue;
            }
            if !target.toml_specified && existing.toml_specified {
                // keep existing
                continue;
            }

            // 2) If one is SingleFile and other DefaultBinary, keep SingleFile
            match (&target.origin, &existing.origin) {
                (Some(TargetOrigin::SingleFile(_)), Some(TargetOrigin::DefaultBinary(_))) => {
                    map.insert(entry_key.clone(), target);
                    continue;
                }
                (Some(TargetOrigin::DefaultBinary(_)), Some(TargetOrigin::SingleFile(_))) => {
                    continue;
                }
                _ => {}
            }

            // 3) Otherwise, choose deeper manifest path
            let current_depth = path_depth(&target.manifest_path);
            let existing_depth = path_depth(&existing.manifest_path);
            if current_depth > existing_depth {
                map.insert(entry_key.clone(), target);
            }
        } else {
            // No collision yet, insert
            map.insert(entry_key, target);
        }
    }

    map.into_values().collect()
}

#[cfg(feature = "concurrent")]
pub fn collect_all_targets_parallel(
    manifest_paths: Vec<Option<std::path::PathBuf>>,
    use_workspace: bool,
    max_concurrency: usize,
    be_silent: bool,
) -> Result<Vec<CargoTarget>, Box<dyn std::error::Error>> {
    use std::sync::mpsc;
    use threadpool::ThreadPool;

    let pool = ThreadPool::new(max_concurrency);
    let (tx, rx) = mpsc::channel();

    for manifest_path in manifest_paths {
        let tx = tx.clone();
        pool.execute(move || {
            let result = collect_all_targets(
                manifest_path,
                use_workspace,
                max_concurrency,
                be_silent,
                true,
            );
            if let Ok(targets) = result {
                tx.send(targets).expect("Failed to send targets");
            }
        });
    }

    drop(tx);
    pool.join();

    let mut all_targets = Vec::new();
    for targets in rx {
        all_targets.extend(targets);
    }

    Ok(all_targets)
}

pub fn collect_all_targets(
    manifest_path: Option<std::path::PathBuf>,
    use_workspace: bool,
    max_concurrency: usize,
    be_silent: bool,
    print_parent: bool,
) -> Result<Vec<CargoTarget>, Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    let mut manifest_infos: Vec<(String, PathBuf, bool)> = Vec::new();

    // Locate the package manifest in the current directory.
    let bi = if let Some(ref path) = manifest_path {
        path.clone()
    } else {
        PathBuf::from(crate::locate_manifest(false)?)
    };
    // We're in workspace mode if the flag is set or if the current Cargo.toml is a workspace manifest.
    let in_workspace = use_workspace || e_workspace::is_workspace_manifest(bi.as_path());

    // let manifest_path_cloned = manifest_path.clone();
    // if manifest_path_cloned.is_some() {
    //     bi.clone_from(&manifest_path_cloned.as_ref().unwrap());
    // }
    // Print some info about the manifest and workspace state
    if in_workspace {
        // Use an explicit workspace manifest if requested; otherwise, assume the current Cargo.toml is the workspace manifest.
        let ws = if manifest_path.is_some() {
            manifest_path.unwrap()
        } else if use_workspace {
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
        if !be_silent {
            println!(
                "workspace: {} [{}]",
                format_workspace(&ws, &bi),
                member_displays.join(", ")
            );
            // Always print the package line.
            println!("package: {}", format_package(&bi));
        }
        manifest_infos.push(("-".to_string(), bi.clone(), false));
        for (member, member_manifest) in ws_members {
            debug!("  member: {}", format_package(&member_manifest));
            manifest_infos.push((format!("${}", member), member_manifest, true));
        }
    } else {
        // Not in workspace mode: simply print the package manifest.
        if !be_silent {
            if print_parent {
                let parent_str = bi.display().to_string();
                let parent_str = parent_str
                    .trim_start_matches(".\\")
                    .trim_start_matches("./");
                let parent_str = parent_str
                    .replace("\\", std::path::MAIN_SEPARATOR_STR)
                    .replace("/", std::path::MAIN_SEPARATOR_STR);
                println!("package: {}", parent_str);
            } else {
                println!("package: {}", format_package(&bi));
            }
        }
        manifest_infos.push(("-".to_string(), bi.clone(), false));
    }

    let samples = collect_samples(use_workspace, manifest_infos, max_concurrency)?;
    // Deduplicate targets: if a SingleFile and DefaultBinary share the same origin, keep only the SingleFile.
    // let deduped_samples = dedup_single_file_over_default_binary(samples);
    // Ok(deduped_samples)
    Ok(samples)
}

/// Same as `collect_all_targets` but does not print workspace/package debug info.
pub fn collect_all_targets_silent(
    use_workspace: bool,
    max_concurrency: usize,
) -> Result<Vec<CargoTarget>, Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    let mut manifest_infos: Vec<(String, PathBuf, bool)> = Vec::new();

    // Locate the package manifest
    let bi = PathBuf::from(crate::locate_manifest(false)?);
    let in_workspace = use_workspace || e_workspace::is_workspace_manifest(bi.as_path());

    if in_workspace {
        let ws = if use_workspace {
            PathBuf::from(crate::locate_manifest(true)?)
        } else {
            bi.clone()
        };
        let ws_members =
            e_workspace::get_workspace_member_manifest_paths(ws.as_path()).unwrap_or_default();
        manifest_infos.push(("-".to_string(), bi.clone(), false));
        for (member, member_manifest) in ws_members {
            manifest_infos.push((format!("${}", member), member_manifest, true));
        }
    } else {
        manifest_infos.push(("-".to_string(), bi.clone(), false));
    }

    let samples = collect_samples(use_workspace, manifest_infos, max_concurrency)?;
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
