// src/e_target.rs
use anyhow::{Context, Result};
use log::{debug, trace};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use toml::Value;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetOrigin {
    DefaultBinary(PathBuf),
    SingleFile(PathBuf),
    MultiFile(PathBuf),
    SubProject(PathBuf),
    TomlSpecified(PathBuf),
    Named(OsString),
    /// A target provided by a plugin, storing plugin file and reported source path
    Plugin {
        plugin_path: PathBuf,
        reported: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Hash, Eq, Copy, PartialOrd, Ord)]
pub enum TargetKind {
    Unknown,
    UnknownExample,
    UnknownExtendedExample,
    UnknownBinary,
    UnknownExtendedBinary,
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
    ManifestLeptos,
    ScriptRustScript,
    ScriptScriptisto,
    /// A target provided by an external plugin (script, WASM, etc.)
    Plugin,
}
impl TargetKind {
    pub fn section_name(&self) -> &'static str {
        match self {
            // TargetKind::UnknownExample | TargetKind::UnknownExtendedExample => "?-example",
            // TargetKind::UnknownBinary | TargetKind::UnknownExtendedBinary => "?-bin",
            TargetKind::Example | TargetKind::ExtendedExample => "example",
            TargetKind::Binary | TargetKind::ExtendedBinary => "bin",
            TargetKind::ManifestTauri => "bin",
            // TargetKind::ScriptScriptisto => "scriptisto",
            // TargetKind::ScriptRustScript => "rust-script",
            TargetKind::ManifestTauriExample => "example",
            TargetKind::ManifestDioxus => "bin",
            TargetKind::ManifestDioxusExample => "example",
            TargetKind::ManifestLeptos => "bin",
            TargetKind::Test => "test",
            TargetKind::Bench => "bench",
            // All other kinds—including Plugin—do not have required-features sections
            _ => "",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            TargetKind::ScriptScriptisto => "scriptisto",
            TargetKind::ScriptRustScript => "rust-script",
            TargetKind::UnknownExample | TargetKind::UnknownExtendedExample => "?-ex.",
            TargetKind::UnknownBinary | TargetKind::UnknownExtendedBinary => "?-bin",
            TargetKind::Example => "ex.",
            TargetKind::ExtendedExample => "exx",
            TargetKind::Binary => "bin",
            TargetKind::ExtendedBinary => "binx",
            TargetKind::ManifestTauri => "tauri",
            TargetKind::ManifestTauriExample => "tauri-e",
            TargetKind::ManifestDioxus => "dioxus",
            TargetKind::ManifestDioxusExample => "dioxus-e",
            TargetKind::ManifestLeptos => "leptos",
            TargetKind::Bench => "bench",
            TargetKind::Test => "test",
            TargetKind::Manifest => "manifest",
            TargetKind::Plugin => "plugin",
            TargetKind::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CargoTarget {
    pub name: String,
    pub display_name: String,
    pub manifest_path: PathBuf,
    pub kind: TargetKind,
    pub extended: bool,
    pub toml_specified: bool,
    pub origin: Option<TargetOrigin>,
}

impl CargoTarget {
    /// Full display label, with a `*` suffix when toml_specified.
    pub fn display_label(&self) -> String {
        let mut label = self.kind.label().to_string();
        if self.toml_specified {
            label.push('*');
        }
        label
    }
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
            false,
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
            toml_specified: false,
            origin: Some(TargetOrigin::SingleFile(file_path.to_path_buf())),
        })
    }

    //     /// Updates the target's name and display_name by interrogating the candidate file and its manifest.
    //     pub fn figure_main_name(&mut self) {
    //         // Only operate if we have a candidate file path.
    //         let candidate = match &self.origin {
    //             Some(TargetOrigin::SingleFile(path)) | Some(TargetOrigin::DefaultBinary(path)) => path,
    //             _ => {
    //                 debug!("No candidate file found in target.origin; skipping name determination");
    //                 return;
    //             }
    //         };
    // println!("figure_main: {}", &candidate.display());
    //         // Get the candidate file's stem in lowercase.
    //         let candidate_stem = candidate
    //             .file_stem()
    //             .and_then(|s| s.to_str())
    //             .map(|s| s.to_lowercase())
    //             .unwrap_or_default();
    //         debug!("Candidate stem: {}", candidate_stem);

    //         // Start with folder-based logic.
    //         let mut name = if candidate_stem == "main"  {
    //             if let Some(parent_dir) = candidate.parent() {
    //                 if let Some(parent_name) = parent_dir.file_name().and_then(|s| s.to_str()) {
    //                     debug!("Candidate parent folder: {}", parent_name);
    //                     if parent_name.eq_ignore_ascii_case("src") {
    //                         // If candidate is src/main.rs, take the parent of "src".
    //                         parent_dir
    //                             .parent()
    //                             .and_then(|proj_dir| proj_dir.file_name())
    //                             .and_then(|s| s.to_str())
    //                             .map(|s| s.to_string())
    //                             .unwrap_or(candidate_stem.clone())
    //                     } else if parent_name.eq_ignore_ascii_case("examples") {
    //                         // If candidate is in an examples folder, use the candidate's parent folder's name.
    //                         candidate
    //                             .parent()
    //                             .and_then(|p| p.file_name())
    //                             .and_then(|s| s.to_str())
    //                             .map(|s| s.to_string())
    //                             .unwrap_or(candidate_stem.clone())
    //                     } else {
    //                         candidate_stem.clone()
    //                     }
    //                 } else {
    //                     candidate_stem.clone()
    //                 }
    //             } else {
    //                 candidate_stem.clone()
    //             }
    //         } else {
    //             candidate_stem.clone()
    //         };

    //         let mut package_manifest_name = String::new();
    //         // If the candidate stem is "main", interrogate the manifest.
    //         let manifest_contents = fs::read_to_string(&self.manifest_path).unwrap_or_default();
    //         if let Ok(manifest_toml) = manifest_contents.parse::<Value>() {
    //             if let Ok(manifest_toml) = manifest_contents.parse::<toml::Value>() {
    //                 // Then try to retrieve the bin section.
    //                 if let Some(bins) = manifest_toml.get("bin").and_then(|v| v.as_array()) {
    //                     debug!("Found {} [[bin]] entries {:?}", bins.len(), bins);
    //                 } else {
    //                     debug!("No [[bin]] array found in manifest");
    //                 }
    //             } else {
    //                 debug!("Failed to parse manifest TOML");
    //             }
    //             debug!("Opened manifest {:?}",&self.manifest_path);
    //             // Check for any [[bin]] entries.
    //             if let Some(bins) = manifest_toml.get("bin").and_then(|v| v.as_array()) {
    //                 debug!("Found {} [[bin]] entries", bins.len());
    //                 if let Some(bin_name) = bins.iter().find_map(|bin| {
    //                     if let Some(path_str) = bin.get("path").and_then(|p| p.as_str()) {
    //                         let bp = bin
    //                         .get("path")
    //                         .and_then(|n| n.as_str())
    //                         .map(|s| s.to_string());
    //                     let bn = bin
    //                                 .get("name")
    //                                 .and_then(|n| n.as_str())
    //                                 .map(|s| s.to_string());
    //                             debug!("Checking bin entry with path: {} {:?}", path_str, bp);
    //                             if bp.as_deref().unwrap_or("") == path_str
    //                             // && bn.as_deref().unwrap_or("") == candidate_stem
    //                         {
    //                             debug!("Found matching bin with name: {:?} {:?}=={:?}", bn,bp.as_deref().unwrap_or(""), path_str);
    //                             name = bn.clone().unwrap_or_default();
    //                             return bn.clone();
    //                         }
    //                     }
    //                     None
    //                 }) {
    //                     //debug!("Using bin name from manifest: {} as {} ", name, bin_name);
    //                     //name = bin_name;
    //                 } else if let Some(pkg) = manifest_toml.get("package") {
    //                     debug!("No matching [[bin]] entry; checking [package] section");
    //                     name = pkg
    //                         .get("name")
    //                         .and_then(|n| n.as_str())
    //                         .unwrap_or(&name)
    //                         .to_string();
    //                     debug!("Using package name from manifest: {}", name);
    //                 }
    //             } else if let Some(pkg) = manifest_toml.get("package") {
    //                 debug!("No [[bin]] section found; using [package] section");
    //                 package_manifest_name = pkg
    //                 .get("name")
    //                 .and_then(|n| n.as_str())
    //                 .unwrap_or(&name)
    //                 .to_string();
    //                 debug!("Using package name from manifest: {}", name);
    //             } else {
    //                 debug!(
    //                     "Manifest does not contain [[bin]] or [package] sections; keeping name: {}",
    //                     name
    //                 );
    //             }
    //         } else {
    //             debug!("Failed to open manifest {:?}",&self.manifest_path);
    //             debug!("Failed to parse manifest TOML; keeping name: {}", name);
    //         }

    //         debug!("Name after folder-based logic: {}", name);

    //         debug!("Final determined name: {}", name);
    //         if name.eq("main") {
    //             panic!("Name is main");
    //         }
    //         self.name = name.clone();
    //         self.display_name = name;
    //     }

    pub fn figure_main_name(&mut self) -> anyhow::Result<()> {
        let mut is_toml_specified = false;
        // if self.toml_specified {
        //     // If the target is already specified in the manifest, return it as is.
        //     return Ok(());
        // }
        // Only operate if we have a candidate file path.
        let candidate = match &self.origin {
            Some(TargetOrigin::SingleFile(path)) | Some(TargetOrigin::DefaultBinary(path)) => {
                Some(path)
            }
            _ => {
                debug!("No candidate file found in target.origin; skipping name determination");
                None
            }
        };

        let candidate = candidate.ok_or_else(|| anyhow::anyhow!("No candidate file found"))?;

        trace!("figure_main: {:?}", &self.origin);

        // Get the candidate file's stem in lowercase.
        let mut candidate_stem = candidate
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        trace!("Candidate stem: {}", candidate_stem);

        // First, check if the manifest path from self matches what we find upward.
        let candidate_dir = candidate.parent().unwrap_or(candidate);
        let found_manifest_dir = crate::e_manifest::find_manifest_dir_from(candidate_dir);
        if let Ok(found_dir) = found_manifest_dir {
            let found_manifest = found_dir.join("Cargo.toml");
            let canon_found = found_manifest.canonicalize()?;
            let canon_target = self.manifest_path.canonicalize()?;
            if canon_found == canon_target {
                trace!(
                    "{} Manifest path matches candidate's upward search result: {:?}",
                    candidate.display(),
                    found_manifest
                );
            } else {
                trace!(
                "{} Manifest path mismatch. Found upward: {:?} but target.manifest_path is: {:?}"
                , candidate.display(), found_manifest, self.manifest_path
            );
                // Compare depths.
                let found_depth = found_manifest.components().count();
                let target_depth = self.manifest_path.components().count();
                if found_depth > target_depth {
                    // Before switching, compare the candidate's relative paths.
                    let orig_parent = self.manifest_path.parent().unwrap_or_else(|| Path::new(""));
                    let found_parent = found_manifest.parent().unwrap_or_else(|| Path::new(""));
                    let orig_rel = candidate.strip_prefix(orig_parent).ok();
                    let found_rel = candidate.strip_prefix(found_parent).ok();
                    if orig_rel == found_rel {
                        trace!(
                            "{} Relative path matches: {:?}",
                            candidate.display(),
                            orig_rel
                        );
                        self.manifest_path = found_manifest;
                    } else {
                        trace!(
                            "{} Relative path mismatch: original: {:?}, found: {:?}",
                            candidate.display(),
                            orig_rel,
                            found_rel
                        );
                    }
                } else {
                    trace!(
                        "{} Keeping target manifest path (deeper or equal): {:?}",
                        candidate.display(),
                        self.manifest_path
                    );
                }
            }
        } else {
            trace!(
                "Could not locate Cargo.toml upward from candidate: {:?}",
                candidate
            );
        }

        // Determine name via manifest processing.
        let mut name = candidate_stem.clone();
        let manifest_contents = fs::read_to_string(&self.manifest_path).unwrap_or_default();
        if let Ok(manifest_toml) = manifest_contents.parse::<Value>() {
            trace!(
                "{} Opened manifest {:?}",
                candidate.display(),
                &self.manifest_path
            );

            // // First, check for any [[bin]] entries.
            // if let Some(bins) = manifest_toml.get("bin").and_then(|v| v.as_array()) {
            //     trace!("Found {} [[bin]] entries", bins.len());
            //     // Iterate over the bin entries and use absolute paths for comparison.
            //     if let Some(bin_name) = bins.iter().find_map(|bin| {
            //         if let (Some(rel_path_str), Some(bn)) = (
            //             bin.get("path").and_then(|p| p.as_str()),
            //             bin.get("name").and_then(|n| n.as_str()),
            //         ) {
            //             // Construct the expected absolute path for the candidate file.
            //             let manifest_parent =
            //                 self.manifest_path.parent().unwrap_or_else(|| Path::new(""));
            //             let expected_path =
            //                 fs::canonicalize(manifest_parent.join(rel_path_str)).ok()?;
            //             let candidate_abs = fs::canonicalize(candidate).ok()?;
            //             trace!(
            //                 "\n{}\n{:?}\nactual candidate absolute path:\n{:?}",
            //                 candidate.display(),
            //                 expected_path,
            //                 candidate_abs
            //             );
            //             if expected_path == candidate_abs {
            //                 trace!(
            //                     "{} Found matching bin with name: {}",
            //                     candidate.display(),
            //                     bn
            //                 );
            //                 return Some(bn.to_string());
            //             }
            //         }
            //         None
            //     }) {
            //         trace!(
            //             "{} Using bin name from manifest: {}",
            //             candidate.display(),
            //             bin_name
            //         );
            //         name = bin_name.clone();
            //         candidate_stem = bin_name.into();
            //     }
            //        }
            if let Some(bin_name) = crate::e_manifest::find_candidate_name(
                &manifest_toml,
                "bin",
                candidate,
                &self.manifest_path,
            ) {
                trace!(
                    "{} Using bin name from manifest: {}",
                    candidate.display(),
                    bin_name
                );
                is_toml_specified = true;
                name = bin_name.clone();
                candidate_stem = bin_name.into();
            } else if let Some(example_name) = crate::e_manifest::find_candidate_name(
                &manifest_toml,
                "example",
                candidate,
                &self.manifest_path,
            ) {
                is_toml_specified = true;
                trace!(
                    "{} Using example name from manifest: {}",
                    candidate.display(),
                    example_name
                );
                name = example_name.clone();
                candidate_stem = example_name.into();
            } else {
                match &self.origin {
                    Some(TargetOrigin::DefaultBinary(_path)) => {
                        // Check for any [package] section.
                        if let Some(pkg) = manifest_toml.get("package") {
                            trace!("Found [package] section in manifest");
                            if let Some(name_value) = pkg.get("name").and_then(|v| v.as_str()) {
                                trace!("Using package name from manifest: {}", name_value);
                                name = name_value.to_string();
                                candidate_stem = name.clone();
                            } else {
                                trace!("No package name found in manifest; keeping name: {}", name);
                            }
                        }
                    }
                    _ => {}
                };
            }

            // if let Some(bins) = manifest_toml.get("bin").and_then(|v| v.as_array()) {
            //     trace!("Found {} [[bin]] entries", bins.len());
            //     // Iterate over the bin entries and use absolute paths for comparison.
            //     if let Some(bin_name) = bins.iter().find_map(|bin| {
            //         if let (Some(rel_path_str), Some(bn)) = (
            //             bin.get("path").and_then(|p| p.as_str()),
            //             bin.get("name").and_then(|n| n.as_str()),
            //         ) {
            //             // Construct the expected absolute path for the candidate file.
            //             let manifest_parent = self.manifest_path.parent().unwrap_or_else(|| Path::new(""));
            //             let expected_path = fs::canonicalize(manifest_parent.join(rel_path_str)).ok()?;
            //             let candidate_abs = fs::canonicalize(candidate).ok()?;
            //             trace!(
            //                 "{} Expected candidate absolute path: {:?}, actual candidate absolute path: {:?}",
            //                 candidate.display(),
            //                 expected_path,
            //                 candidate_abs
            //             );
            //             if expected_path == candidate_abs {
            //                 trace!(
            //                     "{} Found matching bin with name: {}",
            //                     candidate.display(),
            //                     bn
            //                 );
            //                 return Some(bn.to_string());
            //             }
            //         }
            //         None
            //     }) {
            //         trace!("{} Using bin name from manifest: {}", candidate.display(), bin_name);
            //         name = bin_name;
            //     }
            //}
        } else {
            trace!("Failed to open manifest {:?}", &self.manifest_path);
            trace!("Failed to parse manifest TOML; keeping name: {}", name);
        }

        // Only if the candidate stem is "main", apply folder-based logic after manifest processing.
        if candidate_stem == "main" {
            let folder_name = if let Some(parent_dir) = candidate.parent() {
                if let Some(parent_name) = parent_dir.file_name().and_then(|s| s.to_str()) {
                    trace!("Candidate parent folder: {}", parent_name);
                    if parent_name.eq_ignore_ascii_case("src")
                        || parent_name.eq_ignore_ascii_case("src-tauri")
                    {
                        // If candidate is src/main.rs, take the parent of "src".
                        let p = parent_dir
                            .parent()
                            .and_then(|proj_dir| proj_dir.file_name())
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                            .unwrap_or(candidate_stem.clone());
                        if p.eq("src-tauri") {
                            let maybe_name = parent_dir
                                .parent()
                                .and_then(|proj_dir| proj_dir.parent())
                                .and_then(|proj_dir| proj_dir.file_name())
                                .and_then(|s| s.to_str())
                                .map(String::from);
                            match maybe_name {
                                Some(name) => name,
                                None => candidate_stem.clone(),
                            }
                        } else {
                            p
                        }
                    } else if parent_name.eq_ignore_ascii_case("examples") {
                        // If candidate is in an examples folder, use the candidate's parent folder's name.
                        candidate
                            .parent()
                            .and_then(|p| p.file_name())
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                            .unwrap_or(candidate_stem.clone())
                    } else {
                        parent_name.into()
                    }
                } else {
                    candidate_stem.clone()
                }
            } else {
                candidate_stem.clone()
            };
            trace!("Folder-based name: {}-{}", candidate.display(), folder_name);
            // Only override if the folder-based name is different from "main".
            if folder_name != "main" {
                name = folder_name;
            }
        }

        trace!("Final determined name: {}", name);
        if name.eq("main") {
            panic!("Name is main");
        }
        if is_toml_specified {
            self.toml_specified = true;
        }
        self.name = name.clone();
        self.display_name = name;
        Ok(())
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
            trace!("Subproject found: {}", sub_manifest.display());
            trace!("{}", &folder_name);
            return Some(CargoTarget {
                name: folder_name.clone(),
                display_name,
                manifest_path: sub_manifest.clone(),
                // For a subproject, we initially mark it as Manifest;
                // later refinement may resolve it further.
                kind: TargetKind::Manifest,
                toml_specified: false,
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

        let candidate = fs::canonicalize(&candidate).unwrap_or(candidate.to_path_buf());
        // Compute the extended flag based on the candidate file location.
        let extended = crate::e_discovery::is_extended_target(manifest_path, &candidate);

        // Read the candidate file's contents.
        let file_contents = std::fs::read_to_string(&candidate).unwrap_or_default();

        // Use our helper to determine if any special configuration applies.
        let (kind, new_manifest) = crate::e_discovery::determine_target_kind_and_manifest(
            manifest_path,
            &candidate,
            &file_contents,
            example,
            extended,
            false,
            None,
        );
        if kind == TargetKind::Unknown {
            return None;
        }

        // Determine the candidate file's stem in lowercase.
        let name = candidate.file_stem()?.to_str()?.to_lowercase();
        //         let name = if candidate_stem == "main" {
        //     if let Some(parent_dir) = candidate.parent() {
        //         if let Some(parent_name) = parent_dir.file_name().and_then(|s| s.to_str()) {
        //             if parent_name.eq_ignore_ascii_case("src") {
        //                 // If candidate is src/main.rs, take the parent of "src".
        //                 parent_dir.parent()
        //                     .and_then(|proj_dir| proj_dir.file_name())
        //                     .and_then(|s| s.to_str())
        //                     .map(|s| s.to_string())
        //                     .unwrap_or(candidate_stem.clone())
        //             } else if parent_name.eq_ignore_ascii_case("examples") {
        //                 // If candidate is in the examples folder (e.g. examples/main.rs),
        //                 // use the candidate's parent folder's name.
        //                 candidate.parent()
        //                     .and_then(|p| p.file_name())
        //                     .and_then(|s| s.to_str())
        //                     .map(|s| s.to_string())
        //                     .unwrap_or(candidate_stem.clone())
        //             } else {
        //                 // Fall back to the candidate_stem if no special case matches.
        //                 candidate_stem.clone()
        //             }
        //         } else {
        //             candidate_stem.clone()
        //         }
        //     } else {
        //         candidate_stem.clone()
        //     }
        // } else {
        //     candidate_stem.clone()
        // };
        // let name = if candidate_stem.clone() == "main" {
        //     // Read the manifest contents.
        //     let manifest_contents = fs::read_to_string(manifest_path).unwrap_or_default();
        //     if let Ok(manifest_toml) = manifest_contents.parse::<toml::Value>() {
        //         // Look for any [[bin]] entries.
        //         if let Some(bins) = manifest_toml.get("bin").and_then(|v| v.as_array()) {
        //             if let Some(bin_name) = bins.iter().find_map(|bin| {
        //                 if let Some(path_str) = bin.get("path").and_then(|p| p.as_str()) {
        //                     if path_str == "src/bin/main.rs" {
        //                         return bin.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
        //                     }
        //                 }
        //                 None
        //             }) {
        //                 // Found a bin with the matching path; use its name.
        //                 bin_name
        //             } else if let Some(pkg) = manifest_toml.get("package") {
        //                 // No matching bin entry, so use the package name.
        //                 pkg.get("name").and_then(|n| n.as_str()).unwrap_or(&candidate_stem).to_string()
        //             } else {
        //                 candidate_stem.to_string()
        //             }
        //         } else if let Some(pkg) = manifest_toml.get("package") {
        //             // No [[bin]] section; use the package name.
        //             pkg.get("name").and_then(|n| n.as_str()).unwrap_or(&candidate_stem).to_string()
        //         } else {
        //             candidate_stem.to_string()
        //         }
        //     } else {
        //         candidate_stem.to_string()
        //     }
        // } else {
        //     candidate_stem.to_string()
        // };
        let mut target = CargoTarget {
            name: name.clone(),
            display_name: name,
            manifest_path: new_manifest.to_path_buf(),
            kind,
            extended,
            toml_specified: false,
            origin: Some(TargetOrigin::SingleFile(candidate)),
        };
        // Call the method to update name based on the candidate and manifest.
        target.figure_main_name().ok();
        Some(target)
    }
    /// Returns a refined CargoTarget based on its file contents and location.
    /// This function is pure; it takes an immutable CargoTarget and returns a new one.
    /// If the target's origin is either SingleFile or DefaultBinary, it reads the file and uses
    /// `determine_target_kind` to update the kind accordingly.
    pub fn refined_target(target: &CargoTarget) -> CargoTarget {
        let mut refined = target.clone();

        // if target.toml_specified {
        //     // If the target is already specified in the manifest, return it as is.
        //     return refined;
        // }
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
            refined.toml_specified,
            Some(refined.kind),
        );
        if new_kind == TargetKind::ManifestDioxus {}
        refined.kind = new_kind;
        refined.manifest_path = new_manifest;
        refined.figure_main_name().ok();
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

            // // Optionally mark these targets as extended.
            // for t in &mut sub_targets {
            //     if !t.toml_specified {

            //     t.extended = true;
            //     match t.kind {
            //         TargetKind::Example => t.kind = TargetKind::ExtendedExample,
            //         TargetKind::Binary => t.kind = TargetKind::ExtendedBinary,
            //         _ => {} // For other kinds, you may leave them unchanged.
            //     }
            //     }
            // }
            Ok(sub_targets)
        } else {
            // If the target is not a subproject, return an empty vector.
            Ok(vec![])
        }
    }

    pub fn expand_subprojects_in_place(
        targets_map: &mut HashMap<(String, String), CargoTarget>,
    ) -> Result<()> {
        // collect subproject keys…
        let sub_keys: Vec<_> = targets_map
            .iter()
            .filter_map(|(key, t)| {
                matches!(t.origin, Some(TargetOrigin::SubProject(_))).then(|| key.clone())
            })
            .collect();
        log::trace!("Subproject keys: {:?}", sub_keys);
        for key in sub_keys {
            if let Some(sub_target) = targets_map.remove(&key) {
                let expanded = Self::expand_subproject(&sub_target)?;
                for mut new_target in expanded {
                    log::trace!(
                        "Expanding subproject target: {} -> {}",
                        sub_target.display_name,
                        new_target.display_name
                    );
                    // carry forward the toml_specified flag from the original
                    //    new_target.toml_specified |= sub_target.toml_specified;

                    let new_key = Self::target_key(&new_target);

                    match targets_map.entry(new_key) {
                        Entry::Vacant(e) => {
                            new_target.display_name =
                                format!("{} > {}", sub_target.display_name, new_target.name);
                            e.insert(new_target);
                        }
                        Entry::Occupied(mut e) => {
                            // if the existing one is toml-specified, keep it
                            if e.get().toml_specified {
                                new_target.toml_specified = true;
                            }
                            e.insert(new_target);
                        }
                    }
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
            // for t in &mut new_targets {
            //     t.extended = true;
            // }
            // Insert each new target if not already present.
            for new in new_targets {
                let key = CargoTarget::target_key(&new);
                if let Some(existing) = map.get_mut(&key) {
                    // If they already specified this with --manifest-path, leave it untouched:
                    if existing.toml_specified {
                        continue;
                    }
                } else {
                    map.insert(key, new);
                }
                // let key = CargoTarget::target_key(&new);
                // map.entry(key).or_insert(new.clone());
            }
        }
        Ok(())
    }

    /// Returns true if the target is an example.
    pub fn is_example(&self) -> bool {
        matches!(
            self.kind,
            TargetKind::Example
                | TargetKind::UnknownExample
                | TargetKind::UnknownExtendedExample
                | TargetKind::ExtendedExample
                | TargetKind::ManifestDioxusExample
                | TargetKind::ManifestTauriExample
        )
    }
}

/// Deduplicates `CargoTarget` entries by `name`, applying strict priority rules.
///
/// Priority Rules:
/// 1. If the incoming target's `TargetKind` is **greater than `Manifest`**, it overrides any existing lower-priority target,
///    regardless of `TargetOrigin` (including `TomlSpecified`).
/// 2. If both the existing and incoming targets have `TargetKind > Manifest`, prefer the one with the higher `TargetKind`.
/// 3. If neither target is high-priority (`<= Manifest`), compare `(TargetOrigin, TargetKind)` using natural enum ordering.
/// 4. If origin and kind are equal, prefer the target with the deeper `manifest_path`.
/// 5. If any target in the group has `toml_specified = true`, ensure the final target reflects this.
///
/// This guarantees deterministic, priority-driven deduplication while respecting special framework targets.
pub fn dedup_targets(targets: Vec<CargoTarget>) -> Vec<CargoTarget> {
    let mut grouped: HashMap<String, CargoTarget> = HashMap::new();

    for target in &targets {
        let key = target.name.clone();

        grouped
            .entry(key)
            .and_modify(|existing| {
                let target_high = target.kind > TargetKind::Manifest;
                let existing_high = existing.kind > TargetKind::Manifest;

                // Rule 1: If target is high-priority (> Manifest)
                if target_high {
                    if !existing_high || target.kind > existing.kind {
                        let was_toml_specified = existing.toml_specified;
                        *existing = target.clone();
                        existing.toml_specified |= was_toml_specified | target.toml_specified;
                    }
                    return;  // High-priority kinds dominate
                }

                // Rule 2: Both kinds are normal (<= Manifest)
                if target.kind > existing.kind {
                    let was_toml_specified = existing.toml_specified;
                    *existing = target.clone();
                    existing.toml_specified |= was_toml_specified | target.toml_specified;
                    return;
                }

                // Rule 3: If kinds are equal, compare origin
                if target.kind == existing.kind {
                    if target.origin.clone() > existing.origin.clone() {
                        let was_toml_specified = existing.toml_specified;
                        *existing = target.clone();
                        existing.toml_specified |= was_toml_specified | target.toml_specified;
                        return;
                    }

                    // Rule 4: If origin is also equal, compare path depth
                    if target.origin == existing.origin {
                        if path_depth(&target.manifest_path) > path_depth(&existing.manifest_path) {
                            let was_toml_specified = existing.toml_specified;
                            *existing = target.clone();
                            existing.toml_specified |= was_toml_specified | target.toml_specified;
                        }
                    }
                }
                // No replacement needed if none of the conditions matched
            })
            .or_insert(target.clone());
    }

    let toml_specified_names: HashSet<String> = targets.iter()
    .filter(|t| matches!(t.origin, Some(TargetOrigin::TomlSpecified(_))))
    .map(|t| t.name.clone())
    .collect();

// Update toml_specified flag based on origin analysis
for target in grouped.values_mut() {
    if toml_specified_names.contains(&target.name) {
        target.toml_specified = true;
    }
}

// Collect, then sort by (kind, name)
let mut sorted_targets: Vec<_> = grouped.into_values().collect();

sorted_targets.sort_by_key(|t| (t.kind.clone(), t.name.clone()));

sorted_targets
}

/// Calculates the depth of a path (number of components).
fn path_depth(path: &Path) -> usize {
    path.components().count()
}


// /// Returns the "depth" of a path, i.e. the number of components.
// pub fn path_depth(path: &Path) -> usize {
//     path.components().count()
// }

// /// Deduplicates targets that share the same (name, origin key). If duplicates are found,
// /// the target with the manifest path of greater depth is kept.
// pub fn dedup_targets(targets: Vec<CargoTarget>) -> Vec<CargoTarget> {
//     let mut grouped: HashMap<(String, Option<String>), CargoTarget> = HashMap::new();

//     for target in targets {
//         // We'll group targets by (target.name, origin_key)
//         // Create an origin key if available by canonicalizing the origin path.
//         let origin_key = target.origin.as_ref().and_then(|origin| match origin {
//             TargetOrigin::SingleFile(path)
//             | TargetOrigin::DefaultBinary(path)
//             | TargetOrigin::TomlSpecified(path)
//             | TargetOrigin::SubProject(path) => path
//                 .canonicalize()
//                 .ok()
//                 .map(|p| p.to_string_lossy().into_owned()),
//             _ => None,
//         });
//         let key = (target.name.clone(), origin_key);

//         grouped
//             .entry(key)
//             .and_modify(|existing| {
//                 let current_depth = path_depth(&target.manifest_path);
//                 let existing_depth = path_depth(&existing.manifest_path);
//                 // If the current target's manifest path is deeper, replace the existing target.
//                 if current_depth > existing_depth {
//                     println!(
//                         "{} {} Replacing {:?} {:?} with {:?} {:?} manifest path: {} -> {}",
//                         target.name,
//                         existing.name,
//                         target.kind,
//                         existing.kind,
//                         target.origin,
//                         existing.origin,
//                         existing.manifest_path.display(),
//                         target.manifest_path.display()
//                     );
//                     *existing = target.clone();
//                 }
//             })
//             .or_insert(target);
//     }

//     grouped.into_values().collect()
// }
