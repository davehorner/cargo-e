use anyhow::Result;
use arboard::Clipboard;
use chrono::Local;
use std::fs;
use std::path::Path;

// Import shared functions from the top‑level modules.
use crate::cargo_utils::{find_cargo_toml, get_crate_name_from_cargo_toml};
use crate::file_gatherer::gather_files;
mod script_generator;
use self::script_generator::generate_rust_script;

/// Generates a self-contained Rust script that, when executed, recreates the crate's structure and files.
/// The `source_folder` is used as the starting point, and if `src_only` is true, only the `src` subfolder is processed.
pub fn recreate_crate_rs(source_folder: &Path, src_only: bool) -> Result<()> {
    let cargo_toml = find_cargo_toml(source_folder);
    let (crate_root, crate_name) = if let Some(ref toml_path) = cargo_toml {
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
    let rust_script = generate_rust_script(&files_dict, &crate_name);
    let timestamp = Local::now().format("%y%m%d_%H%M").to_string();
    let output_filename = format!("{}_recreate_{}.rs", crate_name, timestamp);
    fs::write(&output_filename, &rust_script)?;
    set_executable_permission(&output_filename)?;
    copy_to_clipboard(&rust_script)?;
    println!("[TRACE] Generated Rust script saved to {}", output_filename);
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
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    Ok(())
}
