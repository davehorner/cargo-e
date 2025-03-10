use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum TargetOrigin {
    SingleFile(PathBuf),
    MultiFile(PathBuf),
    SubProject(PathBuf),
    Named(OsString),
}

#[derive(Debug, Clone)]
pub enum TargetKind {
    Example,
    Binary,
    Test,
    Manifest, // For browsing the entire Cargo.toml or package-level targets.
}

#[derive(Debug, Clone)]
pub struct CargoTarget {
    pub name: String,
    pub display_name: String,
    pub manifest_path: String,
    pub kind: TargetKind,
    pub extended: bool,
    pub origin: Option<TargetOrigin>, // Captures where the target was discovered.
}

/// A builder that constructs a Cargo command for a given target.
pub struct CargoCommandBuilder {
    args: Vec<String>,
}

impl CargoCommandBuilder {
    /// Creates a new, empty builder.
    pub fn new() -> Self {
        CargoCommandBuilder { args: Vec::new() }
    }

    /// Configures the command based on the provided CargoTarget.
    pub fn with_target(mut self, target: &CargoTarget) -> Self {
        match target.kind {
            TargetKind::Example => {
                self.args.push("run".into());
                self.args.push("--example".into());
                self.args.push(target.name.clone());
            }
            TargetKind::Binary => {
                self.args.push("run".into());
                self.args.push("--bin".into());
                self.args.push(target.name.clone());
            }
            TargetKind::Test => {
                self.args.push("test".into());
                self.args.push(target.name.clone());
            }
            TargetKind::Manifest => {
                // For a manifest target, you might simply want to open or browse it.
                // Adjust the behavior as needed.
                self.args.push("manifest".into());
            }
        }

        // If the target is "extended", add a --manifest-path flag
        if target.extended {
            self.args.push("--manifest-path".into());
            self.args.push(target.manifest_path.clone());
        }

        // Optionally use the origin information if available.
        if let Some(ref origin) = target.origin {
            match origin {
                // If it's a subproject, override the manifest path to point directly to that subproject.
                TargetOrigin::SubProject(path) => {
                    self.args.push("--manifest-path".into());
                    self.args.push(path.to_string_lossy().to_string());
                }
                // For other variants, you might want to log or adjust behavior.
                _ => { /* No additional flags needed for SingleFile, MultiFile, or Named */ }
            }
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
        let mut cmd = Command::new("cargo");
        cmd.args(self.args);
        cmd
    }
}

// --- Example usage ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_builder_example() {
        let target = CargoTarget {
            name: "my_example".to_string(),
            display_name: "My Example".to_string(),
            manifest_path: "Cargo.toml".to_string(),
            kind: TargetKind::Example,
            extended: true,
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
