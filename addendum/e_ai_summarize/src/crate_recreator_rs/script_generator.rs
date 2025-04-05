use crate::sanitize;

/// Helper function to create a raw string literal with a delimiter that is one '#' longer
/// than the maximum number of consecutive '#' characters in the input.
fn make_raw_string_literal(input: &str) -> String {
    let cleaned = sanitize(input);
    // If the input already starts with "r#", assume it's already a raw string literal.
    if input.starts_with("r#") {
        return input.to_string();
    }
    // Count the maximum consecutive '#' in the cleaned input
    let mut max_hashes = 0;
    let mut current = 0;
    for ch in cleaned.chars() {
        if ch == '#' {
            current += 1;
            max_hashes = max_hashes.max(current);
        } else {
            current = 0;
        }
    }

    let num_hashes = max_hashes + 1;
    let hashes = "#".repeat(num_hashes);
    format!("r{hashes}\"{}\"{hashes}", cleaned)
}

/// Generates a self-contained Rust script that, when executed,
/// recreates the crate’s directory structure and file contents.
///
/// This function embeds each file's content as a raw string literal using a dynamic delimiter,
/// and converts file paths to use forward slashes.
///
pub fn generate_rust_script(
    files: &std::collections::HashMap<std::path::PathBuf, String>,
    crate_name: &str,
    crate_version: &str,
) -> String {
    let mut lines = Vec::new();

    // Header and imports
    lines.push("#!/usr/bin/env rust-script".to_string());
    lines.push("//! ```cargo".to_string());
    lines.push("//! [dependencies]".to_string());
    lines.push("//! clap = { version = \"4.5.34\", features = [\"derive\"] }".to_string());
    lines.push("//! arboard = \"3.4.1\"".to_string());
    lines.push("//! path-slash = \"0.2.1\"".to_string());
    lines.push("//! ```".to_string());
    lines.push("extern crate clap; extern crate arboard;".to_string());
    lines.push("use clap::Parser;".to_string());
    lines.push("use std::env;".to_string());
    lines.push("use std::fs;".to_string());
    lines.push("use std::path::Path;".to_string());
    lines.push("use path_slash::PathBufExt;".to_string());
    lines.push("use path_slash::PathExt;".to_string());
    lines.push("use arboard::Clipboard;".to_string());
    lines.push(String::new());

    lines.push(format!("const CRATE_NAME: &str = \"{}\";", crate_name));
    lines.push(format!(
        "const CRATE_VERSION: &str = \"{}\";",
        crate_version
    ));
    lines.push(format!("const TOTAL_FILES: i32 = {};", files.len()));
    lines.push(String::new());

    // CLI struct using Clap
    lines.push("#[derive(Parser, Debug)]".to_string());
    lines.push("#[clap(author, version, about)]".to_string());
    lines.push("struct Args {".to_string());
    lines.push(
        "    /// Tunnel mode: copy this script’s own source to the clipboard on exit.".to_string(),
    );
    lines.push("    #[clap(short = 't', long = \"tunnel\")]".to_string());
    lines.push("    tunnel: bool,".to_string());
    lines.push(String::new());
    lines.push("    /// Heredoc mode: (default) copy the combined source to the clipboard and print summary info.".to_string());
    lines.push("    #[clap(short = 'h', long = \"heredoc\")]".to_string());
    lines.push("    heredoc: bool,".to_string());
    lines.push("    /// Generate crate and save to disk.".to_string());
    lines.push("    #[clap(short = 'g', long = \"gen\")]".to_string());
    lines.push("    gen: bool,".to_string());
    lines.push("    #[clap(short = 'p', long = \"path\")]".to_string());
    lines.push("    path: Option<String>,".to_string());

    lines.push("}".to_string());
    lines.push(String::new());

    // Recursive file count helper function
    lines.push("fn count_files_in_dir<P: AsRef<Path>>(path: P) -> usize {".to_string());
    lines.push("    let mut count = 0;".to_string());
    lines.push("    if let Ok(entries) = fs::read_dir(path) {".to_string());
    lines.push("        for entry in entries.flatten() {".to_string());
    lines.push("            let p = entry.path();".to_string());
    lines.push("            if p.is_dir() {".to_string());
    lines.push("                count += count_files_in_dir(&p);".to_string());
    lines.push("            } else if p.is_file() {".to_string());
    lines.push("                count += 1;".to_string());
    lines.push("            }".to_string());
    lines.push("        }".to_string());
    lines.push("    }".to_string());
    lines.push("    count".to_string());
    lines.push("}".to_string());
    lines.push(String::new());

    // Function: handle_heredoc (now prints summary and copies combined source)
    lines.push("fn handle_heredoc() {".to_string());
    lines.push("    let mut combined_source = String::new();".to_string());
    lines.push("    let crate_name = CRATE_NAME.to_string();".to_string());
    lines.push("    let crate_version = CRATE_VERSION.to_string();".to_string());
    lines.push("    if let Ok(entries) = fs::read_dir(\"src\") {".to_string());
    lines.push("        for entry in entries.flatten() {".to_string());
    lines.push("            let path = entry.path();".to_string());
    lines.push(
        "            if path.extension().and_then(|e| e.to_str()) == Some(\"rs\") {".to_string(),
    );
    lines.push("                let rel_path = path.to_slash_lossy().into_owned();".to_string());
    lines.push("                let header = format!(\"//- ----- [{}]::{} -----\\n\", crate_name, rel_path);".to_string());
    lines.push("                combined_source.push_str(&header);".to_string());
    lines.push("                if let Ok(file_content) = fs::read_to_string(&path) {".to_string());
    lines.push("                    combined_source.push_str(&file_content);".to_string());
    lines.push("                }".to_string());
    lines.push("                let footer = format!(\"\\n//- ----- [{}]::{} -----\\n\\n\", crate_name, rel_path);".to_string());
    lines.push("                combined_source.push_str(&footer);".to_string());
    lines.push("            }".to_string());
    lines.push("        }".to_string());
    lines.push("    }".to_string());
    lines.push("    let src_count = count_files_in_dir(\"src\");".to_string());
    lines.push("    let total_count = count_files_in_dir(\".\");".to_string());
    lines.push("    println!(\"{} v{}, src: {}, Total: {}\", crate_name, crate_version, src_count, total_count);".to_string());
    lines.push("    copy_text_to_clipboard(&combined_source);".to_string());
    lines.push("}".to_string());
    lines.push(String::new());

    // Generic function to copy text to the clipboard.
    lines.push("fn copy_text_to_clipboard(text: &str) {".to_string());
    lines.push("    if let Ok(mut clipboard) = Clipboard::new() {".to_string());
    lines.push("        if clipboard.set_text(text).is_ok() {".to_string());
    lines.push("            println!(\"{} {} copied {} to clipboard.\", &CRATE_NAME.to_string(),CRATE_VERSION.to_string(), TOTAL_FILES);".to_string());
    lines.push("        } else {".to_string());
    lines.push("            println!(\"Failed to copy to clipboard.\");".to_string());
    lines.push("        }".to_string());
    lines.push("    } else {".to_string());
    lines.push("        println!(\"Clipboard not available.\");".to_string());
    lines.push("    }".to_string());

    lines.push("}".to_string());
    lines.push(String::new());

    // Function to copy the script's own source to the clipboard.
    lines.push("fn copy_self_to_clipboard() {".to_string());
    lines.push(
        "    let exe_path = env::current_exe().expect(\"Failed to get current exe path\");"
            .to_string(),
    );
    lines.push("    let self_source_path = Path::new(file!());".to_string());
    lines.push("    let content = fs::read_to_string(self_source_path).expect(\"Failed to read self source file\");".to_string());
    lines.push("    copy_text_to_clipboard(&content);".to_string());
    lines.push(
        "    println!(\"Script {} contents copied to clipboard.\",exe_path.display());".to_string(),
    );
    lines.push("}".to_string());
    lines.push(String::new());

    // Helper function to create a dynamic raw string literal.
    lines.push("fn make_raw_string_literal(input: &str) -> String {".to_string());
    lines.push("    let mut max_hashes = 0;".to_string());
    lines.push("    let mut current = 0;".to_string());
    lines.push("    for ch in input.chars() {".to_string());
    lines.push("        if ch == '#' {".to_string());
    lines.push("            current += 1;".to_string());
    lines.push("            if current > max_hashes {".to_string());
    lines.push("                max_hashes = current;".to_string());
    lines.push("            }".to_string());
    lines.push("        } else {".to_string());
    lines.push("            current = 0;".to_string());
    lines.push("        }".to_string());
    lines.push("    }".to_string());
    lines.push("    let num_hashes = max_hashes + 1;".to_string());
    lines.push("    let hashes = \"#\".repeat(num_hashes);".to_string());
    lines.push("    format!(\"r{hashes}\\\"{}\\\"{hashes}\", input)".to_string());
    lines.push("}".to_string());
    lines.push(String::new());

    // Existing function: create_crate (file embedding)
    lines.push("fn create_crate() {".to_string());
    lines.push(format!(
        "    let base_folder = Path::new(\".\").join(\"{}\");",
        crate_name
    ));
    lines.push("    println!(\"base folder: {:?}\", base_folder);".to_string());
    lines.push(
        "    fs::create_dir_all(&base_folder).expect(\"Failed to create base folder\");"
            .to_string(),
    );
    lines.push("    let files = [".to_string());

    for (rel_path, content) in files {
        let slash_path = sanitize(path_slash::PathBufExt::to_slash_lossy(rel_path));
        let literal_content = make_raw_string_literal(content);
        lines.push(format!(
            "        (\"{}\", {}),",
            slash_path, literal_content
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
    lines.push("            println!(\"created directory: {:?}\", parent);".to_string());
    lines.push("        }".to_string());
    lines.push(
        "        fs::write(&full_path, content).expect(\"Failed to write file\");".to_string(),
    );
    lines.push("        println!(\"created file: {:?}\", full_path);".to_string());
    lines.push("    }".to_string());
    lines.push("}".to_string());
    // Main function: if tunnel is specified, copy self; otherwise, handle heredoc
    lines.push("fn main() {".to_string());
    lines.push("    let args = Args::parse();".to_string());
    lines.push("    if args.tunnel {".to_string());
    lines.push("        copy_self_to_clipboard();".to_string());
    lines.push("    } else {".to_string());
    lines.push("        handle_heredoc();".to_string());
    lines.push("    }".to_string());
    lines.push("    create_crate();".to_string());
    lines.push("    println!(\"{} v{}, total_files: {}\", &CRATE_NAME.to_string(),CRATE_VERSION.to_string(), TOTAL_FILES);".to_string());
    lines.push("}".to_string());

    lines.join("\n")
}
