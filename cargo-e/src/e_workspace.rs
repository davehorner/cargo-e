use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

/// Returns true if the Cargo.toml at `manifest_path` is a workspace manifest,
/// i.e. if it contains a `[workspace]` section.
pub fn is_workspace_manifest(manifest_path: &Path) -> bool {
    if let Ok(content) = fs::read_to_string(manifest_path) {
        // Look for a line starting with "[workspace]"
        for line in content.lines() {
            if line.trim_start().starts_with("[workspace]") {
                return true;
            }
        }
    }
    false
}

/// Parses the workspace manifest at `manifest_path` and returns a vector of tuples.
/// Each tuple contains the member's name (derived from its path) and the absolute path
/// to that member's Cargo.toml file.
/// Returns None if no workspace members are found.
pub fn get_workspace_member_manifest_paths(manifest_path: &Path) -> Option<Vec<(String, PathBuf)>> {
    // Read and parse the manifest file as TOML.
    let content = fs::read_to_string(manifest_path).ok()?;
    let parsed: Value = content.parse().ok()?;
    
    // Get the `[workspace]` table.
    let workspace = parsed.get("workspace")?;
    // Get the members array.
    let members = workspace.get("members")?.as_array()?;
    
    // The workspace root is the directory containing the workspace Cargo.toml.
    let workspace_root = manifest_path.parent()?;
    
    // For each member, construct the path: workspace_root / member / "Cargo.toml"
    // and derive a member name from the member path.
    let member_paths: Vec<(String, PathBuf)> = members
        .iter()
        .filter_map(|member| {
            member.as_str().map(|s| {
                // Construct the member's Cargo.toml path.
                let member_manifest = workspace_root.join(s).join("Cargo.toml");
                // Derive the member name from the last path component of s.
                let member_name = Path::new(s)
                    .file_name()
                    .map(|os_str| os_str.to_string_lossy().into_owned())
                    .unwrap_or_else(|| s.to_string());
                (member_name, member_manifest)
            })
        })
        .collect();
    
    if member_paths.is_empty() {
        None
    } else {
        Some(member_paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_get_workspace_member_manifest_paths() {
        let mut file = NamedTempFile::new().unwrap();
        // Write a simple workspace manifest.
        writeln!(file, "[workspace]").unwrap();
        writeln!(file, "members = [\"cargo-e\", \"addendum/e_crate_version_checker\"]").unwrap();
        let paths = get_workspace_member_manifest_paths(file.path());
        assert!(paths.is_some());
        let paths = paths.unwrap();
        // Check that we got two entries.
        assert_eq!(paths.len(), 2);
        // For each entry, the path should end with "Cargo.toml".
        for (_, path) in paths {
            assert!(path.ends_with("Cargo.toml"));
        }
    }
}