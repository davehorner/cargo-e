use std::error::Error;
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
/// their corresponding manifest paths. The workspace manifest is expected to have a [workspace]
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
                    let member_path = workspace_root.join(member_str);
                    let member_manifest = member_path.join("Cargo.toml");
                    if member_manifest.exists() {
                        members.push((member_str.to_string(), member_manifest));
                    }
                }
            }
        }
    }
    Ok(members)
}
