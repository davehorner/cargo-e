use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

use crate::TargetKind;

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
/// their corresponding manifest paths. The workspace manifest is expected to have a \[workspace\]
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

/// Checks whether the manifest at `manifest_path` would trigger the workspace error.
/// If so, it patches the file by appending an empty `[workspace]` table, returning the original content.
/// Otherwise, returns None.
#[allow(dead_code)]
pub(crate) fn maybe_patch_manifest_for_run(
    manifest_path: &Path,
) -> Result<Option<String>, Box<dyn Error>> {
    // Run a lightweight command (cargo metadata) to see if the manifest is affected.
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--manifest-path"])
        .arg(manifest_path)
        .output()?;
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let workspace_error_marker = "current package believes it's in a workspace when it's not:";

    if stderr_str.contains(workspace_error_marker) {
        // Read the original manifest content.
        let original = fs::read_to_string(manifest_path)?;
        // If not already opting out, patch it.
        if !original.contains("[workspace]") {
            let patched = format!("{}\n[workspace]\n", original);
            fs::write(manifest_path, &patched)?;
            return Ok(Some(original));
        }
    }
    Ok(None)
}

/// Search upward from the current directory for Cargo.toml.
pub fn find_manifest_dir() -> std::io::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("Cargo.toml").exists() {
            return Ok(dir);
        }
        // Stop if we cannot go any higher.
        if !dir.pop() {
            break;
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not locate Cargo.toml in the current or parent directories.",
    ))
}

/// Returns a comma‑separated list of required features for a given target,
/// based on its manifest, target kind, and name. If the target is not found
/// in the given manifest and the manifest is a workspace, its members are searched.
pub fn get_required_features_from_manifest(
    manifest_path: &Path,
    kind: &TargetKind,
    target_name: &str,
) -> Option<String> {
    // Read and parse the manifest file.
    let content = fs::read_to_string(manifest_path).ok()?;
    let value: Value = content.parse().ok()?;

    // Map the TargetKind to the corresponding section in the manifest.
    let section = match kind {
        TargetKind::Example | TargetKind::ExtendedExample => "example",
        TargetKind::Binary | TargetKind::ExtendedBinary => "bin",
    };

    // Look for the target in the specified section.
    if let Some(targets) = value.get(section).and_then(|v| v.as_array()) {
        for entry in targets {
            if let Some(name) = entry.get("name").and_then(|v| v.as_str()) {
                if name == target_name {
                    if let Some(req_feats) =
                        entry.get("required-features").and_then(|v| v.as_array())
                    {
                        let feats = req_feats
                            .iter()
                            .filter_map(|f| f.as_str())
                            .collect::<Vec<_>>()
                            .join(",");
                        if !feats.is_empty() {
                            return Some(feats);
                        }
                    }
                }
            }
        }
    }

    // If not found and the manifest has a [workspace] table, check each workspace member.
    if value.get("workspace").is_some() {
        // Convert the manifest_path to a &str.
        if let Some(manifest_str) = manifest_path.to_str() {
            if let Ok(members) = collect_workspace_members(manifest_str) {
                for (_, member_manifest_path) in members {
                    if let Some(feats) = get_required_features_from_manifest(
                        &member_manifest_path,
                        kind,
                        target_name,
                    ) {
                        return Some(feats);
                    }
                }
            }
        }
    }
    None
}
