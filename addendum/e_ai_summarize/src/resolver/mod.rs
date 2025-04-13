use std::{
    fs,
    path::{Path, PathBuf},
};
use regex::Regex;

/// Recursively finds all `crate_name::module` references in the file at `path`,
/// locates the corresponding `.rs` (or `mod.rs`) file under `src/`
/// (relative to the Cargo.toml) or alongside the current `path`, and
/// recurses into it.
///
/// For each module found, prints:
///   Resolved module `<name>` to `<path>`
///
/// If a module cannot be found, prints a warning.
///
/// Returns a Vec<PathBuf> of every file it successfully resolved.
pub fn resolve_local_modules(
    crate_name: &str,
    crate_toml_path: &PathBuf,
    path: &Path,
) -> Vec<PathBuf> {
    let crate_ident = crate_name.replace('-', "_");
    // Read this file’s source
    let source = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));

    // Determine crate root and src directory
    let crate_dir = crate_toml_path
        .parent()
        .expect("Cargo.toml should have a parent directory");
    let src_dir = crate_dir.join("src");

    // Regex to capture `<crate_name>::<module>`
    let re = Regex::new(&format!(r"\b{}::([A-Za-z_][A-Za-z0-9_]*)", crate_ident))
        .expect("Failed to compile regex");

    let mut resolved = Vec::new();
    println!("Resolving modules in {:?}", path);
    println!("Regex: {:?}", re);
    for cap in re.captures_iter(&source) {
        let module = &cap[1];

        // Candidate paths, in order:
        let mut candidates = vec![
            src_dir.join(format!("{}.rs", module)),
            src_dir.join(module).join("mod.rs"),
            path.parent().unwrap_or(Path::new("")).join(format!("{}.rs", module)),
            path.parent().unwrap_or(Path::new("")).join(module).join("mod.rs"),
        ];

        if let Some(found) = candidates.drain(..).find(|p| p.exists()) {
            println!("Resolved module `{}` to {:?}", module, found);
            resolved.push(found.clone());

            // Recurse into the newly found file
            let mut child = resolve_local_modules(crate_name, crate_toml_path, &found);
            resolved.append(&mut child);
        }
        else {
            eprintln!(
                "Warning: could not resolve module `{}` for crate `{}`",
                module, crate_name
            );
        };
    }

    resolved
}