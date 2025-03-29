use std::collections::HashMap;
use std::path::PathBuf;

/// Generates a self-contained Python script that recreates the crate.
/// The `files` parameter maps each relative file path to its contents.
/// The `crate_name` is used to name the base folder in the generated script.
pub fn generate_py_script(files: &HashMap<PathBuf, String>, crate_name: &str) -> String {
    let mut lines = Vec::new();

    lines.push("#!/usr/bin/env python3".to_string());
    lines.push("\"\"\"".to_string());
    lines.push("crate_recreator.py".to_string());
    lines.push("".to_string());
    lines.push(
        "This script recreates the directory structure and files of a Rust crate.".to_string(),
    );
    lines.push("\"\"\"".to_string());
    lines.push("import os".to_string());
    lines.push("import sys".to_string());
    lines.push("import subprocess".to_string());
    lines.push("import stat".to_string());
    lines.push("from datetime import datetime".to_string());
    lines.push("".to_string());
    lines.push("def create_crate():".to_string());
    lines.push(format!(
        "    base_folder = os.path.join(os.getcwd(), '{}')",
        crate_name
    ));
    lines.push("    print('[TRACE] Creating base folder:', base_folder)".to_string());
    lines.push("    os.makedirs(base_folder, exist_ok=True)".to_string());
    lines.push("    files = {".to_string());

    // Embed each file: the key is the relative path and the value is the file content.
    for (rel_path, content) in files {
        let path_str = rel_path.to_string_lossy();
        // Escape the file content so that it can be embedded safely.
        let escaped_content = content.escape_default().to_string();
        lines.push(format!(
            "        {}: {} ,  # File: {}",
            format!("{:?}", path_str),
            format!("{:?}", escaped_content),
            path_str
        ));
    }
    lines.push("    }".to_string());
    lines.push("".to_string());
    lines.push("    for rel_path, content in files.items():".to_string());
    lines.push("        full_path = os.path.join(base_folder, rel_path)".to_string());
    lines.push("        directory = os.path.dirname(full_path)".to_string());
    lines.push("        if not os.path.exists(directory):".to_string());
    lines.push("            os.makedirs(directory, exist_ok=True)".to_string());
    lines.push("            print('[TRACE] Created directory:', directory)".to_string());
    lines.push("        with open(full_path, 'w', encoding='utf-8') as f:".to_string());
    lines.push("            f.write(content)".to_string());
    lines.push("        print('[TRACE] Created file:', full_path)".to_string());
    lines.push("".to_string());
    lines.push("if __name__ == '__main__':".to_string());
    lines.push("    create_crate()".to_string());
    lines.push("    print('[TRACE] Crate creation complete.')".to_string());

    lines.join("\n")
}
