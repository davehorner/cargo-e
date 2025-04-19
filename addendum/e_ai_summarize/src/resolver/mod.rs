use regex::Regex;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

/// Recursively finds all `crate_name::...` references in `path`,
/// up to `remaining_depth` levels (or infinite if `None`), using `visited`
/// to avoid cycles. Returns every file it successfully resolved.
pub fn resolve_local_modules(
    crate_name: &str,
    crate_toml_path: &PathBuf,
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    remaining_depth: Option<usize>,
) -> Vec<PathBuf> {
    // Stop if we've hit the depth limit
    if let Some(0) = remaining_depth {
        return Vec::new();
    }

    // Canonicalize & cycle check
    let canonical = fs::canonicalize(path)
        .unwrap_or_else(|e| panic!("Failed to canonicalize {:?}: {}", path, e));
    if !visited.insert(canonical.clone()) {
        return Vec::new();
    }

    // Read source
    let source =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));

    let crate_ident = crate_name.replace('-', "_");

    // Regex for `use crate::{...}` and `use crate::foo::bar;`
    let use_pattern = format!(
        r"use\s+{crate}::(?:\{{\s*([A-Za-z_][A-Za-z0-9_]*(?:\s*,\s*[A-Za-z_][A-Za-z0-9_]*)*)\s*\}}|([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*))\s*;",
        crate = crate_ident
    );
    let use_re = Regex::new(&use_pattern).expect("Failed to compile use-pattern regex");

    // Regex for function calls: `crate::foo::bar()` with optional `;`
    let call_pattern = format!(
        r"\b{crate}::([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*\(\s*\)\s*;?",
        crate = crate_ident
    );
    let call_re = Regex::new(&call_pattern).expect("Failed to compile call-pattern regex");

    println!("Resolving modules in {:?}", path);
    let mut resolved_modules = Vec::new();

    // 1) Handle `use crate::{...}` and `use crate::foo::bar;`
    for cap in use_re.captures_iter(&source) {
        println!("{:?}", cap);
        // Braced list: group 1
        if let Some(list) = cap.get(1) {
            for module in list.as_str().split(',').map(str::trim) {
                resolved_modules.extend(resolve_one(
                    crate_name,
                    crate_toml_path,
                    module,
                    &canonical,
                    visited,
                    remaining_depth.map(|d| d.saturating_sub(1)),
                ));
            }
        }
        // Single or nested path: group 2
        else if let Some(path_match) = cap.get(2) {
            resolved_modules.extend(resolve_one(
                crate_name,
                crate_toml_path,
                path_match.as_str(),
                &canonical,
                visited,
                remaining_depth.map(|d| d.saturating_sub(1)),
            ));
        }
    }

    // 2) Handle function calls: `crate::foo::bar()`
    for cap in call_re.captures_iter(&source) {
        println!("{:?}", cap);
        let full = cap.get(1).unwrap().as_str(); // e.g. "foo::bar"
                                                 // only resolve if there's at least one `::` to split
        if let Some(idx) = full.rfind("::") {
            let module_path = &full[..idx]; // e.g. "foo"
            resolved_modules.extend(resolve_one(
                crate_name,
                crate_toml_path,
                module_path,
                &canonical,
                visited,
                remaining_depth.map(|d| d.saturating_sub(1)),
            ));
        }
    }

    resolved_modules
}
/// Convert `CamelCase` or `PascalCase` to `snake_case`
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i != 0 {
                result.push('_');
            }
            for lc in c.to_lowercase() {
                result.push(lc);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Resolve a single module path like `"foo::CachedEventStream"` to its file,
/// trying progressively shorter snake_case prefixes (and finally the original),
/// then recurse into any found file.
fn resolve_one(
    crate_name: &str,
    crate_toml_path: &PathBuf,
    module_path: &str,           // e.g. "foo::CachedEventStream"
    current_canonical: &PathBuf, // the file we’re scanning
    visited: &mut HashSet<PathBuf>,
    remaining_depth: Option<usize>,
) -> Vec<PathBuf> {
    let crate_dir = crate_toml_path
        .parent()
        .expect("Cargo.toml should have a parent directory");
    let src_dir = crate_dir.join("src");

    // 1) Split into segments, pull off the last
    let mut segments: Vec<&str> = module_path.split("::").collect();
    let last = segments.pop().unwrap();

    // 2) Build the list of name‐variants to try:
    //    [ "cached_event_stream", "cached_event", "cached", "CachedEventStream" ]
    let snake = to_snake_case(last);
    let parts: Vec<&str> = snake.split('_').collect();
    let mut variants: Vec<String> = Vec::new();
    for i in (1..=parts.len()).rev() {
        variants.push(parts[..i].join("_"));
    }
    // fallback to the original CamelCase
    variants.push(last.to_string());

    // 3) Build candidate paths in priority order
    let mut candidates = Vec::new();

    // 3a) under src/…
    let mut base = src_dir.clone();
    for seg in &segments {
        base = base.join(seg);
    }
    for name in &variants {
        candidates.push(base.join(name).with_extension("rs"));
        candidates.push(base.join(name).join("mod.rs"));
    }

    // 3b) relative to current file’s directory
    let mut rel_base = current_canonical
        .parent()
        .unwrap_or(Path::new(""))
        .to_path_buf();
    for seg in &segments {
        rel_base = rel_base.join(seg);
    }
    for name in &variants {
        candidates.push(rel_base.join(name).with_extension("rs"));
        candidates.push(rel_base.join(name).join("mod.rs"));
    }

    // 4) Pick the first that exists
    if let Some(found) = candidates.into_iter().find(|p| p.exists()) {
        let found_canon = fs::canonicalize(&found)
            .unwrap_or_else(|e| panic!("Failed to canonicalize {:?}: {}", found, e));

        // Skip self‑reference
        if found_canon == *current_canonical {
            println!("Skipping self‑reference to {:?}", found);
            return Vec::new();
        }

        println!("Resolved `{}` → {:?}", module_path, found);
        let mut result = vec![found.clone()];

        // Recurse deeper
        let mut child = resolve_local_modules(
            crate_name,
            crate_toml_path,
            &found,
            visited,
            remaining_depth,
        );
        result.append(&mut child);
        result
    } else {
        eprintln!(
            "Warning: could not resolve `{}` in crate `{}`",
            module_path, crate_name
        );
        Vec::new()
    }
}
// /// Resolve a single module path like "foo::bar" to its file, recurse, and
// /// return everything found downstream.
// fn resolve_one(
//     crate_name: &str,
//     crate_toml_path: &PathBuf,
//     module_path: &str,
//     current_canonical: &PathBuf,
//     visited: &mut HashSet<PathBuf>,
//     remaining_depth: Option<usize>,
// ) -> Vec<PathBuf> {
//     println!("{:?}",module_path);
//     let crate_dir = crate_toml_path
//         .parent()
//         .expect("Cargo.toml should have a parent directory");
//     let src_dir = crate_dir.join("src");

//     // Split "foo::bar" -> ["foo","bar"]
//     let segments: Vec<&str> = module_path.split("::").collect();

//     // Build candidate paths
//     let mut candidates = Vec::new();
//     // src/foo/bar.rs
//     let file_rs = segments.iter().fold(src_dir.clone(), |p, seg| p.join(seg))
//                           .with_extension("rs");
//     candidates.push(file_rs);
//     // src/foo/bar/mod.rs
//     let mod_rs = segments.iter().fold(src_dir.clone(), |p, seg| p.join(seg))
//                          .join("mod.rs");
//     candidates.push(mod_rs);
//     // relative to current file's parent
//     let parent = current_canonical.parent().unwrap_or(Path::new("")).to_path_buf();
//     let rel_rs = segments.iter().fold(parent.clone(), |p, seg| p.join(seg))
//                          .with_extension("rs");
//     candidates.push(rel_rs);
//     // relative mod.rs
//     let rel_mod_rs = segments.iter().fold(parent.clone(), |p, seg| p.join(seg))
//                               .join("mod.rs");
//     candidates.push(rel_mod_rs);

//     // Find the first existing path
//     if let Some(found) = candidates.into_iter().find(|p| p.exists()) {
//         let found_canon = fs::canonicalize(&found)
//             .unwrap_or_else(|e| panic!("Failed to canonicalize {:?}: {}", found, e));
//         // Skip self-reference
//         if found_canon == *current_canonical {
//             println!("Skipping self-reference to {:?}", found);
//             return Vec::new();
//         }

//         println!("Resolved `{}` -> {:?}", module_path, found);
//         let mut resolved = vec![found.clone()];

//         // Recurse
//         let mut child = resolve_local_modules(
//             crate_name,
//             crate_toml_path,
//             &found,
//             visited,
//             remaining_depth,
//         );
//         resolved.append(&mut child);
//         resolved
//     } else {
//         eprintln!(
//             "Warning: could not resolve `{}` in crate `{}`",
//             module_path, crate_name
//         );
//         Vec::new()
//     }
// }

// /// Recursively finds all `crate_name::module` references in `path`,
// /// up to `remaining_depth` levels (or infinite if `None`), using `visited`
// /// to avoid cycles. Returns every file it successfully resolved.
// pub fn resolve_local_modules(
//     crate_name: &str,
//     crate_toml_path: &PathBuf,
//     path: &Path,
//     visited: &mut HashSet<PathBuf>,
//     remaining_depth: Option<usize>,
// ) -> Vec<PathBuf> {
//     // If we've hit the depth limit, stop here.
//     if let Some(0) = remaining_depth {
//         return Vec::new();
//     }

//     // Canonicalize so we catch symlinked duplicates too
//     let canonical = fs::canonicalize(path)
//         .unwrap_or_else(|e| panic!("Failed to canonicalize {:?}: {}", path, e));

//     // If we've already been here, bail out
//     if !visited.insert(canonical.clone()) {
//         return Vec::new();
//     }

//     let source = fs::read_to_string(path)
//         .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));

//     let crate_dir = crate_toml_path
//         .parent()
//         .expect("Cargo.toml should have a parent directory");
//     let src_dir = crate_dir.join("src");
//     let crate_ident = crate_name.replace('-', "_");
//  // build a regex that captures either:
// //  - blinds::{ A, B, C }
// //  - blinds::X
// let pattern = format!(
//     r"\b{crate}::(?:
//         \{{\s*([A-Za-z_][A-Za-z0-9_]*(?:\s*,\s*[A-Za-z_][A-Za-z0-9_]*)*)\s*\}}  # group import
//       | ([A-Za-z_][A-Za-z0-9_]*)                                                # single import
//     )",
//     crate = crate_ident,
// );
// let re = Regex::new(&pattern).expect("Failed to compile regex");
//     // let re = Regex::new(&format!(r"\b{}::([A-Za-z_][A-Za-z0-9_]*)", crate_ident))
//     //     .expect("Failed to compile regex");

//     println!("Resolving modules in {:?}", path);
//     let mut resolved = Vec::new();

//     for cap in re.captures_iter(&source) {
//         let module = &cap[1];
//         let mut candidates = vec![
//             src_dir.join(format!("{}.rs", module)),
//             src_dir.join(module).join("mod.rs"),
//             path.parent().unwrap_or(Path::new("")).join(format!("{}.rs", module)),
//             path.parent().unwrap_or(Path::new("")).join(module).join("mod.rs"),
//         ];

//         if let Some(found) = candidates.drain(..).find(|p| p.exists()) {
//             // Avoid self‑reference
//             let found_canon = fs::canonicalize(&found)
//                 .unwrap_or_else(|e| panic!("Failed to canonicalize {:?}: {}", found, e));
//             if found_canon == canonical {
//                 println!("Skipping self‑reference to {:?}", found);
//                 continue;
//             }

//             println!("Resolved module `{}` to {:?}", module, found);
//             resolved.push(found.clone());

//             // Recurse with decremented depth
//             let next_depth = remaining_depth.map(|d| d.saturating_sub(1));
//             let mut child = resolve_local_modules(
//                 crate_name,
//                 crate_toml_path,
//                 &found,
//                 visited,
//                 next_depth,
//             );
//             resolved.append(&mut child);
//         } else {
//             eprintln!(
//                 "Warning: could not resolve module `{}` for crate `{}`",
//                 module, crate_name
//             );
//         };
//     }

//     resolved
// }
// /// Recursively finds all `crate_name::module` references in the file at `path`,
// /// locates the corresponding `.rs` (or `mod.rs`) file under `src/`
// /// (relative to the Cargo.toml) or alongside the current `path`, and
// /// recurses into it.
// ///
// /// For each module found, prints:
// ///   Resolved module `<name>` to `<path>`
// ///
// /// If a module cannot be found, prints a warning.
// ///
// /// Returns a Vec<PathBuf> of every file it successfully resolved.
// pub fn resolve_local_modules(
//     crate_name: &str,
//     crate_toml_path: &PathBuf,
//     path: &Path,
// ) -> Vec<PathBuf> {
//     let crate_ident = crate_name.replace('-', "_");
//     // Read this file’s source
//     let source = fs::read_to_string(path)
//         .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));

//     // Determine crate root and src directory
//     let crate_dir = crate_toml_path
//         .parent()
//         .expect("Cargo.toml should have a parent directory");
//     let src_dir = crate_dir.join("src");

//     // Regex to capture `<crate_name>::<module>`
//     let re = Regex::new(&format!(r"\b{}::([A-Za-z_][A-Za-z0-9_]*)", crate_ident))
//         .expect("Failed to compile regex");

//     let mut resolved = Vec::new();
//     println!("Resolving modules in {:?}", path);
//     println!("Regex: {:?}", re);
//     for cap in re.captures_iter(&source) {
//         let module = &cap[1];

//         // Candidate paths, in order:
//         let mut candidates = vec![
//             src_dir.join(format!("{}.rs", module)),
//             src_dir.join(module).join("mod.rs"),
//             path.parent().unwrap_or(Path::new("")).join(format!("{}.rs", module)),
//             path.parent().unwrap_or(Path::new("")).join(module).join("mod.rs"),
//         ];

//         if let Some(found) = candidates.drain(..).find(|p| p.exists()) {
//             println!("Resolved module `{}` to {:?}", module, found);
//             resolved.push(found.clone());

//             // Recurse into the newly found file
//             let mut child = resolve_local_modules(crate_name, crate_toml_path, &found);
//             resolved.append(&mut child);
//         }
//         else {
//             eprintln!(
//                 "Warning: could not resolve module `{}` for crate `{}`",
//                 module, crate_name
//             );
//         };
//     }

//     resolved
// }
