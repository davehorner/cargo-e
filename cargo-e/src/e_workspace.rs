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
    // Read and parse the workspace manifest.
    let content = fs::read_to_string(manifest_path).ok()?;
    let parsed: Value = content.parse().ok()?;

    // Get the `[workspace]` table and its "members" array.
    let workspace = parsed.get("workspace")?;
    let members = workspace.get("members")?.as_array()?;

    // The workspace root is the directory containing the workspace Cargo.toml.
    let workspace_root = manifest_path.parent()?;

    let mut member_paths = Vec::new();

    for member in members {
        if let Some(s) = member.as_str() {
            if s.ends_with("/*") {
                // Strip the trailing "/*" and use that as a base directory.
                let base = workspace_root.join(s.trim_end_matches("/*"));
                // Scan the base directory for subdirectories that contain a Cargo.toml.
                if let Ok(entries) = fs::read_dir(&base) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let cargo_toml = path.join("Cargo.toml");
                            if cargo_toml.exists() {
                                // Use the directory's name as the member name.
                                if let Some(member_name) =
                                    path.file_name().and_then(|os| os.to_str())
                                {
                                    member_paths.push((
                                        format!(
                                            "{}/{}",
                                            s.trim_end_matches("/*"),
                                            member_name.to_string()
                                        ),
                                        cargo_toml,
                                    ));
                                }
                            }
                        }
                    }
                }
            } else {
                // Use the declared member path directly.
                let member_path = workspace_root.join(s);
                let member_manifest = member_path.join("Cargo.toml");
                if member_manifest.exists() {
                    let mut member_name = Path::new(s)
                        .file_name()
                        .and_then(|os| os.to_str())
                        .unwrap_or(s)
                        .to_string();
                    if member_name.eq("src-tauri") {
                        // Special case for src-tauri, use the parent directory name.
                        member_name = member_path
                            .parent()
                            .and_then(|p| p.file_name())
                            .and_then(|os| os.to_str())
                            .unwrap_or(s)
                            .to_string();
                    }
                    member_paths.push((member_name, member_manifest));
                }
            }
        }
    }

    if member_paths.is_empty() {
        None
    } else {
        Some(member_paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_member_manifest_paths_found() {
        // Create a temporary directory to serve as the workspace root.
        let temp_dir = TempDir::new().unwrap();

        // Create a workspace manifest (Cargo.toml) in the workspace root.
        let workspace_manifest_path = temp_dir.path().join("Cargo.toml");
        let workspace_manifest_content = r#"
[workspace]
members = ["cargo-e", "addendum/e_crate_version_checker"]
        "#;
        fs::write(&workspace_manifest_path, workspace_manifest_content).unwrap();

        // Create a dummy member directory "cargo-e" with its own Cargo.toml.
        let cargo_e_dir = temp_dir.path().join("cargo-e");
        fs::create_dir_all(&cargo_e_dir).unwrap();
        fs::write(cargo_e_dir.join("Cargo.toml"), "dummy content").unwrap();

        // Create a dummy member directory "addendum/e_crate_version_checker" with its own Cargo.toml.
        let e_crate_dir = temp_dir
            .path()
            .join("addendum")
            .join("e_crate_version_checker");
        fs::create_dir_all(&e_crate_dir).unwrap();
        fs::write(e_crate_dir.join("Cargo.toml"), "dummy content").unwrap();

        // Call the function under test.
        let result = get_workspace_member_manifest_paths(&workspace_manifest_path);
        assert!(result.is_some());
        let members = result.unwrap();
        assert_eq!(members.len(), 2);

        // Verify that each returned path ends with "Cargo.toml".
        for (_, path) in members {
            assert!(path.ends_with("Cargo.toml"));
        }
    }

    #[test]
    fn test_workspace_member_manifest_paths_not_found() {
        // Create a temporary directory to serve as the workspace root.
        let temp_dir = TempDir::new().unwrap();

        // Create a workspace manifest (Cargo.toml) in the workspace root.
        let workspace_manifest_path = temp_dir.path().join("Cargo.toml");
        let workspace_manifest_content = r#"
[workspace]
members = ["cargo-e", "addendum/e_crate_version_checker"]
        "#;
        fs::write(&workspace_manifest_path, workspace_manifest_content).unwrap();

        // Do NOT create the dummy member directories or Cargo.toml files.

        // Call the function and assert that it returns None.
        let result = get_workspace_member_manifest_paths(&workspace_manifest_path);
        assert!(result.is_none());
    }
}
