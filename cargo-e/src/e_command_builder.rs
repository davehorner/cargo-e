use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

use crate::e_target::{CargoTarget, TargetKind, TargetOrigin};

/// A builder that constructs a Cargo command for a given target.
#[derive(Clone)]
pub struct CargoCommandBuilder {
    pub args: Vec<String>,
    pub alternate_cmd: Option<String>,
    pub execution_dir: Option<PathBuf>,
    pub suppressed_flags: HashSet<String>,
}
impl Default for CargoCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}
impl CargoCommandBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> Self {
        CargoCommandBuilder {
            args: Vec::new(),
            alternate_cmd: None,
            execution_dir: None,
            suppressed_flags: HashSet::new(),
        }
    }

    // /// Configures the command based on the provided CargoTarget.
    // pub fn with_target(mut self, target: &CargoTarget) -> Self {
    //     match target.kind {
    //         CargoTargetKind::Example => {
    //             self.args.push("run".into());
    //             self.args.push("--example".into());
    //             self.args.push(target.name.clone());
    //         }
    //         CargoTargetKind::Binary => {
    //             self.args.push("run".into());
    //             self.args.push("--bin".into());
    //             self.args.push(target.name.clone());
    //         }
    //         CargoTargetKind::Test => {
    //             self.args.push("test".into());
    //             self.args.push(target.name.clone());
    //         }
    //         CargoTargetKind::Manifest => {
    //             // For a manifest target, you might simply want to open or browse it.
    //             // Adjust the behavior as needed.
    //             self.args.push("manifest".into());
    //         }
    //     }

    //     // If the target is "extended", add a --manifest-path flag
    //     if target.extended {
    //         self.args.push("--manifest-path".into());
    //         self.args.push(target.manifest_path.clone());
    //     }

    //     // Optionally use the origin information if available.

    //     if let Some(TargetOrigin::SubProject(ref path)) = target.origin {
    //         self.args.push("--manifest-path".into());
    //         self.args.push(path.display().to_string());
    //     }

    //     self
    // }

    /// Configure the command based on the target kind.
    pub fn with_target(mut self, target: &CargoTarget) -> Self {
        println!("Target origin: {:?}", target.origin.clone().unwrap());
        match target.kind {
            TargetKind::Unknown => {
                return self;
            }
            TargetKind::Bench => {
                // To run benchmarks, use the "bench" command.
                self.alternate_cmd = Some("bench".to_string());
                self.args.push(target.name.clone());
            }
            TargetKind::Test => {
                self.args.push("test".into());
                // Pass the target's name as a filter to run specific tests.
                self.args.push(target.name.clone());
            }
            TargetKind::Example | TargetKind::ExtendedExample => {
                self.args.push("run".into());
                self.args.push("--message-format=json".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::Binary | TargetKind::ExtendedBinary => {
                self.args.push("run".into());
                self.args.push("--bin".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::Manifest => {
                self.suppressed_flags.insert("quiet".to_string());
                self.args.push("run".into());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::ManifestTauriExample => {
                self.suppressed_flags.insert("quiet".to_string());
                self.args.push("run".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self.args.push("--manifest-path".into());
                self.args.push(
                    target
                        .manifest_path
                        .clone()
                        .to_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
            TargetKind::ManifestTauri => {
                self.suppressed_flags.insert("quiet".to_string());
                // Helper closure to check for tauri.conf.json in a directory.
                let has_tauri_conf = |dir: &Path| -> bool { dir.join("tauri.conf.json").exists() };

                // Try candidate's parent (if origin is SingleFile or DefaultBinary).
                let candidate_dir_opt = match &target.origin {
                    Some(TargetOrigin::SingleFile(path))
                    | Some(TargetOrigin::DefaultBinary(path)) => path.parent(),
                    _ => None,
                };

                if let Some(candidate_dir) = candidate_dir_opt {
                    if has_tauri_conf(candidate_dir) {
                        println!("Using candidate directory: {}", candidate_dir.display());
                        self.execution_dir = Some(candidate_dir.to_path_buf());
                    } else if let Some(manifest_parent) = target.manifest_path.parent() {
                        if has_tauri_conf(manifest_parent) {
                            println!("Using manifest parent: {}", manifest_parent.display());
                            self.execution_dir = Some(manifest_parent.to_path_buf());
                        } else if let Some(grandparent) = manifest_parent.parent() {
                            if has_tauri_conf(grandparent) {
                                println!("Using manifest grandparent: {}", grandparent.display());
                                self.execution_dir = Some(grandparent.to_path_buf());
                            } else {
                                println!("No tauri.conf.json found in candidate, manifest parent, or grandparent; defaulting to manifest parent: {}", manifest_parent.display());
                                self.execution_dir = Some(manifest_parent.to_path_buf());
                            }
                        } else {
                            println!("No grandparent for manifest; defaulting to candidate directory: {}", candidate_dir.display());
                            self.execution_dir = Some(candidate_dir.to_path_buf());
                        }
                    } else {
                        println!(
                            "No manifest parent found for: {}",
                            target.manifest_path.display()
                        );
                    }
                } else if let Some(manifest_parent) = target.manifest_path.parent() {
                    if has_tauri_conf(manifest_parent) {
                        println!("Using manifest parent: {}", manifest_parent.display());
                        self.execution_dir = Some(manifest_parent.to_path_buf());
                    } else if let Some(grandparent) = manifest_parent.parent() {
                        if has_tauri_conf(grandparent) {
                            println!("Using manifest grandparent: {}", grandparent.display());
                            self.execution_dir = Some(grandparent.to_path_buf());
                        } else {
                            println!(
                                "No tauri.conf.json found; defaulting to manifest parent: {}",
                                manifest_parent.display()
                            );
                            self.execution_dir = Some(manifest_parent.to_path_buf());
                        }
                    }
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.args.push("tauri".into());
                self.args.push("dev".into());
            }
            TargetKind::ManifestDioxus => {
                let exe_path = match which("dx") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'dx' not found in PATH: {}", err);
                        return self;
                    }
                };
                // For Dioxus targets, print the manifest path and set the execution directory
                // to be the same directory as the manifest.
                if let Some(manifest_parent) = target.manifest_path.parent() {
                    println!("Manifest path: {}", target.manifest_path.display());
                    println!(
                        "Execution directory (same as manifest folder): {}",
                        manifest_parent.display()
                    );
                    self.execution_dir = Some(manifest_parent.to_path_buf());
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                self.args.push("serve".into());
                self = self.with_required_features(&target.manifest_path, target);
            }
            TargetKind::ManifestDioxusExample => {
                let exe_path = match which("dx") {
                    Ok(path) => path,
                    Err(err) => {
                        eprintln!("Error: 'dx' not found in PATH: {}", err);
                        return self;
                    }
                };
                // For Dioxus targets, print the manifest path and set the execution directory
                // to be the same directory as the manifest.
                if let Some(manifest_parent) = target.manifest_path.parent() {
                    println!("Manifest path: {}", target.manifest_path.display());
                    println!(
                        "Execution directory (same as manifest folder): {}",
                        manifest_parent.display()
                    );
                    self.execution_dir = Some(manifest_parent.to_path_buf());
                } else {
                    println!(
                        "No manifest parent found for: {}",
                        target.manifest_path.display()
                    );
                }
                self.alternate_cmd = Some(exe_path.as_os_str().to_string_lossy().to_string());
                self.args.push("serve".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
                self = self.with_required_features(&target.manifest_path, target);
            }
        }
        self
    }

    /// Configure the command using CLI options.
    pub fn with_cli(mut self, cli: &crate::Cli) -> Self {
        if cli.quiet && !self.suppressed_flags.contains("quiet") {
            // Insert --quiet right after "run" if present.
            if let Some(pos) = self.args.iter().position(|arg| arg == "run") {
                self.args.insert(pos + 1, "--quiet".into());
            } else {
                self.args.push("--quiet".into());
            }
        }
        if cli.release {
            // Insert --release right after the initial "run" command if applicable.
            // For example, if the command already contains "run", insert "--release" after it.
            if let Some(pos) = self.args.iter().position(|arg| arg == "run") {
                self.args.insert(pos + 1, "--release".into());
            } else {
                // If not running a "run" command (like in the Tauri case), simply push it.
                self.args.push("--release".into());
            }
        }
        // Append extra arguments (if any) after a "--" separator.
        if !cli.extra.is_empty() {
            self.args.push("--".into());
            self.args.extend(cli.extra.iter().cloned());
        }
        self
    }
    /// Append required features based on the manifest, target kind, and name.
    /// This method queries your manifest helper function and, if features are found,
    /// appends "--features" and the feature list.
    pub fn with_required_features(mut self, manifest: &PathBuf, target: &CargoTarget) -> Self {
        if let Some(features) = crate::e_manifest::get_required_features_from_manifest(
            manifest,
            &target.kind,
            &target.name,
        ) {
            self.args.push("--features".to_string());
            self.args.push(features);
        }
        self
    }

    /// Appends extra arguments to the command.
    pub fn with_extra_args(mut self, extra: &[String]) -> Self {
        if !extra.is_empty() {
            // Use "--" to separate Cargo arguments from target-specific arguments.
            self.args.push("--".into());
            self.args.extend(extra.iter().cloned());
        }
        self
    }

    /// Builds the final vector of command-line arguments.
    pub fn build(self) -> Vec<String> {
        self.args
    }

    /// Optionally, builds a std::process::Command.
    pub fn build_command(self) -> Command {
        let mut cmd = if let Some(alternate) = self.alternate_cmd {
            Command::new(alternate)
        } else {
            Command::new("cargo")
        };
        cmd.args(self.args);
        if let Some(dir) = self.execution_dir {
            cmd.current_dir(dir);
        }
        cmd
    }
}

// --- Example usage ---
#[cfg(test)]
mod tests {
    use crate::e_target::TargetOrigin;

    use super::*;

    #[test]
    fn test_command_builder_example() {
        let target = CargoTarget {
            name: "my_example".to_string(),
            display_name: "My Example".to_string(),
            manifest_path: "Cargo.toml".into(),
            kind: TargetKind::Example,
            extended: true,
            toml_specified: false,
            origin: Some(TargetOrigin::SingleFile(PathBuf::from(
                "examples/my_example.rs",
            ))),
        };

        let extra_args = vec!["--flag".to_string(), "value".to_string()];

        let args = CargoCommandBuilder::new()
            .with_target(&target)
            .with_extra_args(&extra_args)
            .build();

        // For an example target, we expect something like:
        // cargo run --example my_example --manifest-path Cargo.toml -- --flag value
        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--example".to_string()));
        assert!(args.contains(&"my_example".to_string()));
        assert!(args.contains(&"--manifest-path".to_string()));
        assert!(args.contains(&"Cargo.toml".to_string()));
        assert!(args.contains(&"--".to_string()));
        assert!(args.contains(&"--flag".to_string()));
        assert!(args.contains(&"value".to_string()));
    }
}
