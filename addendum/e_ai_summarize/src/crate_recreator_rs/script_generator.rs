use std::collections::HashMap;
use std::path::PathBuf;

/// Generates a self-contained Rust script that, when executed,
/// recreates the crate’s directory structure and file contents.
///
/// The function escapes all backslashes and double quotes in both the file paths
/// and file contents so that they are safely embedded in a double-quoted string literal.
pub fn generate_rust_script(files: &HashMap<PathBuf, String>, crate_name: &str) -> String {
    let mut lines = Vec::new();

    // Header lines
    lines.push("#!/usr/bin/env rust-script".to_string());
    lines.push("use std::fs;".to_string());
    lines.push("use std::path::Path;".to_string());
    lines.push(String::new());

    lines.push("/// Recreates the directory structure and files of the crate.".to_string());
    lines.push("fn create_crate() {".to_string());
    lines.push(format!(
        "    let base_folder = Path::new(\".\").join(\"{}\");",
        crate_name
    ));
    lines.push("    println!(\"[TRACE] Creating base folder: {:?}\", base_folder);".to_string());
    lines.push(
        "    fs::create_dir_all(&base_folder).expect(\"Failed to create base folder\");"
            .to_string(),
    );
    lines.push("    let files = [".to_string());

    // For each file, escape backslashes and double quotes in both path and content.
    for (rel_path, content) in files {
        let path_str = rel_path.to_string_lossy();
        let escaped_path = path_str.replace('\\', "\\\\").replace('\"', "\\\"");
        let escaped_content = content.replace('\\', "\\\\").replace('\"', "\\\"");
        lines.push(format!(
            "        (\"{}\", \"{}\"),  // File: {}",
            escaped_path, escaped_content, path_str
        ));
    }

    lines.push("    ];".to_string());
    lines.push(String::new());

    lines.push("    for (rel_path, content) in files.iter() {".to_string());
    lines.push("        let full_path = base_folder.join(rel_path);".to_string());
    lines.push("        if let Some(parent) = full_path.parent() {".to_string());
    lines.push(
        "            fs::create_dir_all(parent).expect(\"Failed to create directory\");"
            .to_string(),
    );
    lines.push("            println!(\"[TRACE] Created directory: {:?}\", parent);".to_string());
    lines.push("        }".to_string());
    lines.push(
        "        fs::write(&full_path, content).expect(\"Failed to write file\");".to_string(),
    );
    lines.push("        println!(\"[TRACE] Created file: {:?}\", full_path);".to_string());
    lines.push("    }".to_string());
    lines.push("}".to_string());
    lines.push(String::new());

    lines.push("fn main() {".to_string());
    lines.push("    create_crate();".to_string());
    lines.push("    println!(\"[TRACE] Crate creation complete.\");".to_string());
    lines.push("}".to_string());

    lines.join("\n")
}
