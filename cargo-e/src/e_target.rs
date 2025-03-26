// src/e_target.rs
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub enum TargetOrigin {
    DefaultBinary(PathBuf),
    SingleFile(PathBuf),
    MultiFile(PathBuf),
    SubProject(PathBuf),
    Named(OsString),
}

#[derive(Debug, Clone, PartialEq, Hash, Eq, Copy)]
pub enum TargetKind {
    Unknown,
    Example,
    ExtendedExample,
    Binary,
    ExtendedBinary,
    Bench,
    Test,
    Manifest, // For browsing the entire Cargo.toml or package-level targets.
    ManifestTauri,
    ManifestTauriExample,
    ManifestDioxusExample,
    ManifestDioxus,
}

#[derive(Debug, Clone)]
pub struct CargoTarget {
    pub name: String,
    pub display_name: String,
    pub manifest_path: PathBuf,
    pub kind: TargetKind,
    pub extended: bool,
    pub origin: Option<TargetOrigin>,
}

impl CargoTarget {
    /// Constructs a CargoTarget from a source file.
    ///
    /// Reads the file at `file_path` and determines the target kind based on:
    /// - Tauri configuration (e.g. if the manifest's parent is "src-tauri" or a Tauri config exists),
    /// - Dioxus markers in the file contents,
    /// - And finally, if the file contains "fn main", using its parent directory (examples vs bin) to decide.
    ///
    /// If none of these conditions are met, returns None.
    pub fn from_source_file(
        stem: &std::ffi::OsStr,
        file_path: &Path,
        manifest_path: &Path,
        example: bool,
        extended: bool,
    ) -> Option<Self> {
        let file_path = fs::canonicalize(&file_path).unwrap_or(file_path.to_path_buf());
        let file_contents = std::fs::read_to_string(&file_path).unwrap_or_default();
        let (kind, new_manifest) = crate::e_discovery::determine_target_kind_and_manifest(
            manifest_path,
            &file_path,
            &file_contents,
            example,
            extended,
            None,
        );
        if kind == TargetKind::Unknown {
            return None;
        }
        let name = stem.to_string_lossy().to_string();
        Some(CargoTarget {
            name: name.clone(),
            display_name: name,
            manifest_path: new_manifest.to_path_buf(),
            kind,
            extended,
            origin: Some(TargetOrigin::SingleFile(file_path.to_path_buf())),
        })
    }

    /// Constructs a CargoTarget from a folder by trying to locate a runnable source file.
    ///
    /// The function attempts the following candidate paths in order:
    /// 1. A file named `<folder_name>.rs` in the folder.
    /// 2. `src/main.rs` inside the folder.
    /// 3. `main.rs` at the folder root.
    /// 4. Otherwise, it scans the folder for any `.rs` file containing `"fn main"`.
    ///
    /// Once a candidate is found, it reads its contents and calls `determine_target_kind`
    /// to refine the target kind based on Tauri or Dioxus markers. The `extended` flag
    /// indicates whether the target should be marked as extended (for instance, if the folder
    /// is a subdirectory of the primary "examples" or "bin" folder).
    ///
    /// Returns Some(CargoTarget) if a runnable file is found, or None otherwise.
    pub fn from_folder(
        folder: &Path,
        manifest_path: &Path,
        example: bool,
        _extended: bool,
    ) -> Option<Self> {
        // If the folder contains its own Cargo.toml, treat it as a subproject.
        let sub_manifest = folder.join("Cargo.toml");
        if sub_manifest.exists() {
            // Use the folder's name as the candidate target name.
            let folder_name = folder.file_name()?.to_string_lossy().to_string();
            // Determine the display name from the parent folder.
            let display_name = if let Some(parent) = folder.parent() {
                let parent_name = parent.file_name()?.to_string_lossy();
                if parent_name == folder_name {
                    // If the parent's name equals the folder's name, try using the grandparent.
                    if let Some(grandparent) = parent.parent() {
                        grandparent.file_name()?.to_string_lossy().to_string()
                    } else {
                        folder_name.clone()
                    }
                } else {
                    parent_name.to_string()
                }
            } else {
                folder_name.clone()
            };

            let sub_manifest =
                fs::canonicalize(&sub_manifest).unwrap_or(sub_manifest.to_path_buf());
            println!("Subproject found: {}", sub_manifest.display());
            println!("{}", &folder_name);
            return Some(CargoTarget {
                name: folder_name.clone(),
                display_name,
                manifest_path: sub_manifest.clone(),
                // For a subproject, we initially mark it as Manifest;
                // later refinement may resolve it further.
                kind: TargetKind::Manifest,
                extended: true,
                origin: Some(TargetOrigin::SubProject(sub_manifest)),
            });
        }
        // Extract the folder's name.
        let folder_name = folder.file_name()?.to_str()?;

        /// Returns Some(candidate) only if the file exists and its contents contain "fn main".
        fn candidate_with_main(candidate: PathBuf) -> Option<PathBuf> {
            if candidate.exists() {
                let contents = fs::read_to_string(&candidate).unwrap_or_default();
                if contents.contains("fn main") {
                    return Some(candidate);
                }
            }
            None
        }

        // In your from_folder function, for example:
        let candidate = if let Some(candidate) =
            candidate_with_main(folder.join(format!("{}.rs", folder_name)))
        {
            candidate
        } else if let Some(candidate) = candidate_with_main(folder.join("src/main.rs")) {
            candidate
        } else if let Some(candidate) = candidate_with_main(folder.join("main.rs")) {
            candidate
        } else {
            // Otherwise, scan the folder for any .rs file containing "fn main"
            let mut found = None;
            if let Ok(entries) = fs::read_dir(folder) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                        if let Some(candidate) = candidate_with_main(path) {
                            found = Some(candidate);
                            break;
                        }
                    }
                }
            }
            found?
        };

        // // First candidate: folder/<folder_name>.rs
        // let candidate = if folder.join(format!("{}.rs", folder_name)).exists() {
        //     folder.join(format!("{}.rs", folder_name))
        // } else if folder.join("src/main.rs").exists() {
        //     folder.join("src/main.rs")
        // } else if folder.join("main.rs").exists() {
        //     folder.join("main.rs")
        // } else {
        //     // Otherwise, scan the folder for any .rs file containing "fn main"
        //     let mut found = None;
        //     if let Ok(entries) = fs::read_dir(folder) {
        //         for entry in entries.flatten() {
        //             let path = entry.path();
        //             if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
        //                 let contents = fs::read_to_string(&path).unwrap_or_default();
        //                 if contents.contains("fn main") {
        //                     found = Some(path);
        //                     break;
        //                 }
        //             }
        //         }
        //     }
        //     found?
        // };

        let candidate = fs::canonicalize(&candidate).unwrap_or(candidate.to_path_buf());
        // Compute the extended flag based on the candidate file location.
        let extended = crate::e_discovery::is_extended_target(manifest_path, &candidate);

        // // Determine a fallback target kind based on the folder name.
        // let fallback_kind = if folder_name.to_lowercase() == "examples" {
        //     if extended {
        //         TargetKind::ExtendedExample
        //     } else {
        //         TargetKind::Example
        //     }
        // } else if folder_name.to_lowercase() == "bin" {
        //     if extended {
        //         TargetKind::ExtendedBinary
        //     } else {
        //         TargetKind::Binary
        //     }
        // } else {
        //     TargetKind::Example
        // };

        // Read the candidate file's contents.
        let file_contents = std::fs::read_to_string(&candidate).unwrap_or_default();

        // Use our helper to determine if any special configuration applies.
        let (kind, new_manifest) = crate::e_discovery::determine_target_kind_and_manifest(
            manifest_path,
            &candidate,
            &file_contents,
            example,
            extended,
            None,
        );
        if kind == TargetKind::Unknown {
            return None;
        }

        // Determine the candidate file's stem in lowercase.
        let candidate_stem = candidate.file_stem()?.to_str()?.to_lowercase();
        let name = if candidate_stem == "main" {
            candidate
                .parent()
                .and_then(|p| p.parent())
                .and_then(|gp| {
                    gp.file_name().and_then(|s| s.to_str()).and_then(|s| {
                        if s.to_lowercase() == "examples" {
                            // Use candidate's parent folder's name.
                            candidate
                                .parent()
                                .and_then(|p| p.file_name())
                                .and_then(|s| s.to_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or(candidate_stem)
        } else {
            candidate_stem
        };
        Some(CargoTarget {
            name: name.clone(),
            display_name: name,
            manifest_path: new_manifest.to_path_buf(),
            kind,
            extended,
            origin: Some(TargetOrigin::SingleFile(candidate)),
        })
    }
    /// Returns a refined CargoTarget based on its file contents and location.
    /// This function is pure; it takes an immutable CargoTarget and returns a new one.
    /// If the target's origin is either SingleFile or DefaultBinary, it reads the file and uses
    /// `determine_target_kind` to update the kind accordingly.
    pub fn refined_target(target: &CargoTarget) -> CargoTarget {
        let mut refined = target.clone();

        // Operate only if the target has a file to inspect.
        let file_path = match &refined.origin {
            Some(TargetOrigin::SingleFile(path)) | Some(TargetOrigin::DefaultBinary(path)) => path,
            _ => return refined,
        };

        let file_path = fs::canonicalize(&file_path).unwrap_or(file_path.to_path_buf());
        let file_contents = std::fs::read_to_string(&file_path).unwrap_or_default();

        let (new_kind, new_manifest) = crate::e_discovery::determine_target_kind_and_manifest(
            &refined.manifest_path,
            &file_path,
            &file_contents,
            refined.is_example(),
            refined.extended,
            Some(refined.kind),
        );
        refined.kind = new_kind;
        refined.manifest_path = new_manifest;
        refined
    }

    /// Expands a subproject CargoTarget into multiple runnable targets.
    ///
    /// If the given target's origin is a subproject (i.e. its Cargo.toml is in a subfolder),
    /// this function loads that Cargo.toml and uses `get_runnable_targets` to discover its runnable targets.
    /// It then flattens and returns them as a single `Vec<CargoTarget>`.
    pub fn expand_subproject(target: &CargoTarget) -> Result<Vec<CargoTarget>> {
        // Ensure the target is a subproject.
        if let Some(TargetOrigin::SubProject(sub_manifest)) = &target.origin {
            // Use get_runnable_targets to get targets defined in the subproject.
            let (bins, examples, benches, tests) =
                crate::e_manifest::get_runnable_targets(sub_manifest).with_context(|| {
                    format!(
                        "Failed to get runnable targets from {}",
                        sub_manifest.display()
                    )
                })?;
            let mut sub_targets = Vec::new();
            sub_targets.extend(bins);
            sub_targets.extend(examples);
            sub_targets.extend(benches);
            sub_targets.extend(tests);

            // Optionally mark these targets as extended.
            for t in &mut sub_targets {
                t.extended = true;
                match t.kind {
                    TargetKind::Example => t.kind = TargetKind::ExtendedExample,
                    TargetKind::Binary => t.kind = TargetKind::ExtendedBinary,
                    _ => {} // For other kinds, you may leave them unchanged.
                }
            }
            Ok(sub_targets)
        } else {
            // If the target is not a subproject, return an empty vector.
            Ok(vec![])
        }
    }

    /// Expands subproject targets in the given map.
    /// For every target with a SubProject origin, this function removes the original target,
    /// expands it using `expand_subproject`, and then inserts the expanded targets.
    /// The expanded targets have their display names modified to include the original folder name as a prefix.
    /// This version replaces any existing target with the same key.
    pub fn expand_subprojects_in_place(
        targets_map: &mut HashMap<(String, String), CargoTarget>,
    ) -> Result<()> {
        // Collect keys for targets that are subprojects.
        let sub_keys: Vec<(String, String)> = targets_map
            .iter()
            .filter_map(|(key, target)| {
                if let Some(TargetOrigin::SubProject(_)) = target.origin {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        for key in sub_keys {
            if let Some(sub_target) = targets_map.remove(&key) {
                // Expand the subproject target.
                let expanded_targets = Self::expand_subproject(&sub_target)?;
                for mut new_target in expanded_targets {
                    // Update the display name to include the subproject folder name.
                    // For example, if sub_target.display_name was "foo" and new_target.name is "bar",
                    // the new display name becomes "foo > bar".
                    new_target.display_name =
                        format!("{} > {}", sub_target.display_name, new_target.name);
                    // Create a key for the expanded target.
                    let new_key = Self::target_key(&new_target);
                    // Replace any existing target with the same key.
                    targets_map.insert(new_key, new_target);
                }
            }
        }
        Ok(())
    }
    // /// Expands subproject targets in `targets`. Any target whose origin is a SubProject
    // /// is replaced by the targets returned by `expand_subproject`. If the expansion fails,
    // /// you can choose to log the error and keep the original target, or remove it.
    // pub fn expand_subprojects_in_place(
    //     targets_map: &mut HashMap<(String, String), CargoTarget>
    // ) -> anyhow::Result<()> {
    //     // Collect keys for subproject targets.
    //     let sub_keys: Vec<(String, String)> = targets_map
    //         .iter()
    //         .filter_map(|(key, target)| {
    //             if let Some(crate::e_target::TargetOrigin::SubProject(_)) = target.origin {
    //                 Some(key.clone())
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect();

    //     // For each subproject target, remove it from the map, expand it, and insert the new targets.
    //     for key in sub_keys {
    //         if let Some(sub_target) = targets_map.remove(&key) {
    //             let expanded = Self::expand_subproject(&sub_target)?;
    //             for new_target in expanded {
    //                 let new_key = CargoTarget::target_key(&new_target);
    //                 targets_map.entry(new_key).or_insert(new_target);
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    /// Creates a unique key for a target based on its manifest path and name.
    pub fn target_key(target: &CargoTarget) -> (String, String) {
        let manifest = target
            .manifest_path
            .canonicalize()
            .unwrap_or_else(|_| target.manifest_path.clone())
            .to_string_lossy()
            .into_owned();
        let name = target.name.clone();
        (manifest, name)
    }

    /// Expands a subproject target into multiple targets and inserts them into the provided HashMap,
    /// using (manifest, name) as a key to avoid duplicates.
    pub fn expand_subproject_into_map(
        target: &CargoTarget,
        map: &mut std::collections::HashMap<(String, String), CargoTarget>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Only operate if the target is a subproject.
        if let Some(crate::e_target::TargetOrigin::SubProject(sub_manifest)) = &target.origin {
            // Discover targets in the subproject.
            let (bins, examples, benches, tests) =
                crate::e_manifest::get_runnable_targets(sub_manifest)?;
            let mut new_targets = Vec::new();
            new_targets.extend(bins);
            new_targets.extend(examples);
            new_targets.extend(benches);
            new_targets.extend(tests);
            // Mark these targets as extended.
            for t in &mut new_targets {
                t.extended = true;
            }
            // Insert each new target if not already present.
            for new in new_targets {
                let key = CargoTarget::target_key(&new);
                map.entry(key).or_insert(new.clone());
                println!("Inserted subproject target: {}", new.name);
            }
        }
        Ok(())
    }

    /// Returns true if the target is an example.
    pub fn is_example(&self) -> bool {
        matches!(
            self.kind,
            TargetKind::Example
                | TargetKind::ExtendedExample
                | TargetKind::ManifestDioxusExample
                | TargetKind::ManifestTauriExample
        )
    }
}

/// Returns the "depth" of a path, i.e. the number of components.
pub fn path_depth(path: &Path) -> usize {
    path.components().count()
}

/// Deduplicates targets that share the same (name, origin key). If duplicates are found,
/// the target with the manifest path of greater depth is kept.
pub fn dedup_targets(targets: Vec<CargoTarget>) -> Vec<CargoTarget> {
    let mut grouped: HashMap<(String, Option<String>), CargoTarget> = HashMap::new();

    for target in targets {
        // We'll group targets by (target.name, origin_key)
        // Create an origin key if available by canonicalizing the origin path.
        let origin_key = target.origin.as_ref().and_then(|origin| match origin {
            TargetOrigin::SingleFile(path)
            | TargetOrigin::DefaultBinary(path)
            | TargetOrigin::SubProject(path) => path
                .canonicalize()
                .ok()
                .map(|p| p.to_string_lossy().into_owned()),
            _ => None,
        });
        let key = (target.name.clone(), origin_key);

        grouped
            .entry(key)
            .and_modify(|existing| {
                let current_depth = path_depth(&target.manifest_path);
                let existing_depth = path_depth(&existing.manifest_path);
                // If the current target's manifest path is deeper, replace the existing target.
                if current_depth > existing_depth {
                    *existing = target.clone();
                }
            })
            .or_insert(target);
    }

    grouped.into_values().collect()
}
