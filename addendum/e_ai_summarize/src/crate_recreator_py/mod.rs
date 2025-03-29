use anyhow::Result;
use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
mod script_generator;
use self::script_generator::generate_py_script;
// Import shared functions from the top‑level modules.
use crate::cargo_utils::{find_cargo_toml, get_crate_name_from_cargo_toml};
use crate::file_gatherer::gather_files;

/// Recreates the crate by generating a Python script that, when executed,
/// rebuilds the directory structure and files of the crate.
/// The `source_folder` is used as the starting point, and if `src_only` is true,
/// only the `src` subfolder is processed.
pub fn recreate_crate_py(source_folder: &Path, src_only: bool) -> Result<()> {
    let cargo_toml = find_cargo_toml(source_folder);
    let (crate_root, crate_name) = if let Some(ref toml_path) = cargo_toml {
        // Explicitly annotate type to help inference.
        let root: std::path::PathBuf = toml_path.parent().unwrap().to_path_buf();
        let name = get_crate_name_from_cargo_toml(toml_path).unwrap_or_else(|| {
            root.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("crate")
                .to_string()
        });
        (root, name)
    } else {
        let fallback = source_folder
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("crate")
            .to_string();
        (source_folder.to_path_buf(), fallback)
    };

    let gather_folder = if src_only {
        crate_root.join("src")
    } else {
        crate_root.clone()
    };

    if !gather_folder.exists() {
        anyhow::bail!("Source folder {:?} does not exist.", gather_folder);
    }

    let files_dict = gather_files(&gather_folder)?;
    let py_script = generate_py_script(&files_dict, &crate_name);
    let timestamp = Local::now().format("%y%m%d_%H%M").to_string();
    let output_filename = format!("{}_recreate_{}.py", crate_name, timestamp);
    fs::write(&output_filename, &py_script)?;
    set_executable_permission(&output_filename)?;
    copy_to_clipboard(&py_script)?;
    println!(
        "[TRACE] Generated Python script saved to {}",
        output_filename
    );
    Ok(())
}

#[cfg(unix)]
fn set_executable_permission(path: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let mut perms = metadata.permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable_permission(_path: &str) -> Result<()> {
    Ok(())
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    if cfg!(target_os = "windows") {
        let mut child = Command::new("clip")
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
    } else if cfg!(target_os = "macos") {
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
    } else {
        println!("[TRACE] Clipboard copy not supported on this platform.");
    }
    Ok(())
}
