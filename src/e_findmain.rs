// src/e_findmain.rs

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;
use tracing::error;

use crate::e_types::{Example, TargetKind};

/// Given an Example, attempts to locate the main file.
///
/// For **extended samples** (i.e. sample.extended is true), it first checks for a file at:
/// 1. `<manifest_dir>/src/main.rs`  
/// 2. `<manifest_dir>/main.rs`  
/// and if found returns that path.
///
/// Otherwise (or if the above do not exist), it falls back to parsing the Cargo.toml:
///   - For binaries: it looks in the `[[bin]]` section.
///   - For examples: it first checks the `[[example]]` section, and if not found, falls back to `[[bin]]`.
/// If a target matching the sample name is found, it uses the provided `"path"` (if any)
/// or defaults to `"src/main.rs"`.
/// Returns Some(candidate) if the file exists.
pub fn find_main_file(sample: &Example) -> Option<PathBuf> {
    let manifest_path = Path::new(&sample.manifest_path);
    let base = manifest_path.parent()?;

    if sample.extended {
        // Check conventional locations in extended samples.
        let candidate_src = base.join("src/main.rs");
        if candidate_src.exists() {
            return Some(candidate_src);
        }
        let candidate_main = base.join("main.rs");
        if candidate_main.exists() {
            return Some(candidate_main);
        }
        // If neither conventional file exists, fall through to Cargo.toml parsing.
    }

    let contents = fs::read_to_string(manifest_path).ok()?;
    let value: Value = contents.parse().ok()?;
    let targets = if sample.kind == TargetKind::Binary {
        value.get("bin")
    } else {
        value.get("example").or_else(|| value.get("bin"))
    }?;
    if let Some(arr) = targets.as_array() {
        for target in arr {
            if let Some(name) = target.get("name").and_then(|v| v.as_str()) {
                if name == sample.name {
                    let relative = target
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("src/main.rs");
                    let base = manifest_path.parent()?;
                    let candidate = base.join(relative);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    None
}

/// Searches the given file for "fn main" and returns (line, column) where it is first found.
/// Both line and column are 1-indexed.
pub fn find_main_line(file: &Path) -> Option<(usize, usize)> {
    let content = fs::read_to_string(file).ok()?;
    for (i, line) in content.lines().enumerate() {
        if let Some(col) = line.find("fn main") {
            return Some((i + 1, col + 1));
        }
    }
    None
}

/// Computes the arguments for VSCode given a sample target.
/// Returns a tuple (folder_str, goto_arg).
/// - `folder_str` is the folder that will be opened in VSCode.
/// - `goto_arg` is an optional string of the form "<file>:<line>:<column>"
///   determined by searching for "fn main" in the candidate file.
///
/// For extended samples, it checks first for "src/main.rs", then "main.rs".
/// For non-extended examples, it assumes the file is at "examples/<name>.rs" relative to cwd.
pub fn compute_vscode_args(sample: &Example) -> (String, Option<String>) {
    let manifest_path = Path::new(&sample.manifest_path);
    // Debug print
    println!("DEBUG: manifest_path: {:?}", manifest_path);

    let candidate_file: Option<PathBuf> = if sample.kind == TargetKind::Binary
        || (sample.kind == TargetKind::Example && sample.extended)
    {
        // Try to find the main file via Cargo.toml.
        find_main_file(sample).or_else(|| {
            // Fallback to "src/main.rs" in the manifest's folder.
            let base = manifest_path.parent()?;
            let fallback = base.join("src/main.rs");
            if fallback.exists() {
                Some(fallback)
            } else {
                None
            }
        })
    } else if sample.kind == TargetKind::Example && !sample.extended {
        // For built-in examples, assume the file is "examples/<name>.rs" relative to current directory.
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let fallback = cwd.join("examples").join(format!("{}.rs", sample.name));
        if fallback.exists() {
            Some(fallback)
        } else {
            None
        }
    } else {
        None
    };

    println!("DEBUG: candidate_file: {:?}", candidate_file);

    let (folder, goto_arg) = if let Some(file) = candidate_file {
        let folder = file.parent().unwrap_or(&file).to_path_buf();
        let goto_arg = if let Some((line, col)) = find_main_line(&file) {
            Some(format!(
                "{}:{}:{}",
                file.to_str().unwrap_or_default(),
                line,
                col
            ))
        } else {
            Some(file.to_str().unwrap_or_default().to_string())
        };
        (folder, goto_arg)
    } else {
        (
            manifest_path
                .parent()
                .unwrap_or(manifest_path)
                .to_path_buf(),
            None,
        )
    };

    let folder_str = folder.to_str().unwrap_or_default().to_string();
    println!("DEBUG: folder_str: {}", folder_str);
    println!("DEBUG: goto_arg: {:?}", goto_arg);

    (folder_str, goto_arg)
}

/// Asynchronously opens VSCode for the given sample target.
/// It computes the VSCode arguments using `compute_vscode_args` and then launches VSCode.
pub async fn open_vscode_for_sample(sample: &Example) {
    let (folder_str, goto_arg) = compute_vscode_args(sample);

    let output = if cfg!(target_os = "windows") {
        if let Some(ref goto) = goto_arg {
            Command::new("cmd")
                .args(&["/C", "code", folder_str.as_str(), "--goto", goto.as_str()])
                .output()
        } else {
            Command::new("cmd")
                .args(&["/C", "code", folder_str.as_str()])
                .output()
        }
    } else {
        let mut cmd = Command::new("code");
        cmd.arg(folder_str.as_str());
        if let Some(goto) = goto_arg {
            cmd.args(&["--goto", goto.as_str()]);
        }
        cmd.output()
    };

    match output {
        Ok(output) if output.status.success() => {
            // VSCode opened successfully.
            println!("DEBUG: VSCode command output: {:?}", output);
        }
        Ok(output) => {
            let msg = format!(
                "Error opening VSCode:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            error!("{}", msg);
        }
        Err(e) => {
            let msg = format!("Failed to execute VSCode command: {}", e);
            error!("{}", msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    // use std::io::Write;

    // #[test]
    // fn test_find_main_file_default() -> Result<(), Box<dyn std::error::Error>> {
    //     // Create a temporary directory.
    //     let dir = tempdir()?;
    //     let manifest_path = dir.path().join("Cargo.toml");
    //     let main_rs = dir.path().join("src/main.rs");
    //     fs::create_dir_all(main_rs.parent().unwrap())?;
    //     fs::write(&main_rs, "fn main() {}")?;

    //     // Write a Cargo.toml with a [[bin]] table without an explicit "path".
    //     let toml_contents = r#"
    //         [package]
    //         name = "dummy"
    //         version = "0.1.0"
    //         edition = "2021"

    //         [[bin]]
    //         name = "sample1"
    //     "#;
    //     fs::write(&manifest_path, toml_contents)?;

    //     let sample = Example {
    //         name: "sample1".to_string(),
    //         display_name: "dummy".to_string(),
    //         manifest_path: manifest_path.to_string_lossy().to_string(),
    //         kind: TargetKind::Binary,
    //         extended: false,
    //     };

    //     let found = find_main_file(&sample).expect("Should find main file");
    //     assert_eq!(found, main_rs);
    //     dir.close()?;
    //     Ok(())
    // }

    // #[test]
    // fn test_find_main_file_with_explicit_path() -> Result<(), Box<dyn std::error::Error>> {
    //     let dir = tempdir()?;
    //     let manifest_path = dir.path().join("Cargo.toml");
    //     let custom_main = dir.path().join("custom/main.rs");
    //     fs::create_dir_all(custom_main.parent().unwrap())?;
    //     fs::write(&custom_main, "fn main() {}")?;

    //     let toml_contents = format!(
    //         r#"
    //         [package]
    //         name = "dummy"
    //         version = "0.1.0"
    //         edition = "2021"

    //         [[bin]]
    //         name = "sample2"
    //         path = "{}"
    //         "#,
    //         custom_main.strip_prefix(dir.path()).unwrap().to_str().unwrap()
    //     );
    //     fs::write(&manifest_path, toml_contents)?;

    //     let sample = Example {
    //         name: "sample2".to_string(),
    //         display_name: "dummy".to_string(),
    //         manifest_path: manifest_path.to_string_lossy().to_string(),
    //         kind: TargetKind::Binary,
    //         extended: false,
    //     };

    //     let found = find_main_file(&sample).expect("Should find custom main file");
    //     assert_eq!(found, custom_main);
    //     dir.close()?;
    //     Ok(())
    // }

    // #[test]
    // fn test_find_main_line() -> Result<(), Box<dyn std::error::Error>> {
    //     let dir = tempdir()?;
    //     let file_path = dir.path().join("src/main.rs");
    //     fs::create_dir_all(file_path.parent().unwrap())?;
    //     let content = "\n\nfn helper() {}\nfn main() { println!(\"Hello\"); }\n";
    //     fs::write(&file_path, content)?;
    //     let pos = find_main_line(&file_path).expect("Should find fn main");
    //     assert_eq!(pos.0, 4); // Line 4 should contain fn main.
    //     dir.close()?;
    //     Ok(())
    // }
    // Test for a non-extended sample with no explicit path in Cargo.toml (should fallback to "src/main.rs").
    #[test]
    fn test_find_main_file_default() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let manifest_path = dir.path().join("Cargo.toml");
        let main_rs = dir.path().join("src/main.rs");
        fs::create_dir_all(main_rs.parent().unwrap())?;
        fs::write(&main_rs, "fn main() {}")?;
        let toml_contents = r#"
            [package]
            name = "dummy"
            version = "0.1.0"
            edition = "2021"
            
            [[bin]]
            name = "sample1"
        "#;
        fs::write(&manifest_path, toml_contents)?;
        let sample = Example {
            name: "sample1".to_string(),
            display_name: "dummy".to_string(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            kind: TargetKind::Binary,
            extended: false,
        };
        let found = find_main_file(&sample).expect("Should find main file");
        assert_eq!(found, main_rs);
        dir.close()?;
        Ok(())
    }

    // Test for a non-extended sample with an explicit "path" in Cargo.toml.
    #[test]
    fn test_find_main_file_with_explicit_path() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let manifest_path = dir.path().join("Cargo.toml");
        let custom_main = dir.path().join("custom/main.rs");
        fs::create_dir_all(custom_main.parent().unwrap())?;
        fs::write(&custom_main, "fn main() {}")?;
        let toml_contents = format!(
            r#"
            [package]
            name = "dummy"
            version = "0.1.0"
            edition = "2021"
            
            [[bin]]
            name = "sample2"
            path = "{}"
            "#,
            custom_main
                .strip_prefix(dir.path())
                .unwrap()
                .to_str()
                .unwrap()
        );
        fs::write(&manifest_path, toml_contents)?;
        let sample = Example {
            name: "sample2".to_string(),
            display_name: "dummy".to_string(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            kind: TargetKind::Binary,
            extended: false,
        };
        let found = find_main_file(&sample).expect("Should find custom main file");
        assert_eq!(found, custom_main);
        dir.close()?;
        Ok(())
    }

    // Test for an extended sample where "src/main.rs" exists.
    #[test]
    fn test_extended_sample_src_main() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        // Simulate an extended sample folder (e.g. "examples/sample_ext")
        let sample_dir = dir.path().join("examples").join("sample_ext");
        fs::create_dir_all(sample_dir.join("src"))?;
        let main_rs = sample_dir.join("src/main.rs");
        fs::write(&main_rs, "fn main() {}")?;
        // Write a Cargo.toml in the sample folder.
        let manifest_path = sample_dir.join("Cargo.toml");
        let toml_contents = r#"
            [package]
            name = "sample_ext"
            version = "0.1.0"
            edition = "2021"
        "#;
        fs::write(&manifest_path, toml_contents)?;

        let sample = Example {
            name: "sample_ext".to_string(),
            display_name: "extended sample".to_string(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            kind: TargetKind::Example,
            extended: true,
        };

        // For extended samples, our function should find "src/main.rs" first.
        let found = find_main_file(&sample).expect("Should find src/main.rs in extended sample");
        assert_eq!(found, main_rs);
        dir.close()?;
        Ok(())
    }

    // Test for an extended sample where "src/main.rs" does not exist but "main.rs" exists.
    #[test]
    fn test_extended_sample_main_rs() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let sample_dir = dir.path().join("examples").join("sample_ext2");
        fs::create_dir_all(&sample_dir)?;
        let main_rs = sample_dir.join("main.rs");
        fs::write(&main_rs, "fn main() {}")?;
        let manifest_path = sample_dir.join("Cargo.toml");
        let toml_contents = r#"
            [package]
            name = "sample_ext2"
            version = "0.1.0"
            edition = "2021"
        "#;
        fs::write(&manifest_path, toml_contents)?;
        let sample = Example {
            name: "sample_ext2".to_string(),
            display_name: "extended sample 2".to_string(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            kind: TargetKind::Example,
            extended: true,
        };
        let found = find_main_file(&sample).expect("Should find main.rs in extended sample");
        assert_eq!(found, main_rs);
        dir.close()?;
        Ok(())
    }

    // Test for find_main_line: it should locate "fn main" and return the correct (line, column).
    #[test]
    fn test_find_main_line() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let file_path = dir.path().join("src/main.rs");
        fs::create_dir_all(file_path.parent().unwrap())?;
        // Create a file with some lines and a line with "fn main"
        let content = "\n\nfn helper() {}\nfn main() { println!(\"Hello\"); }\n";
        fs::write(&file_path, content)?;
        let pos = find_main_line(&file_path).expect("Should find fn main");
        // "fn main" should appear on line 4 (1-indexed)
        assert_eq!(pos.0, 4);
        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_compute_vscode_args_non_extended() -> Result<(), Box<dyn std::error::Error>> {
        // Simulate a non-extended example where the file is "examples/<name>.rs".
        let cwd = std::env::current_dir()?;
        let examples_dir = cwd.join("examples");
        fs::create_dir_all(&examples_dir)?;
        let sample_file = examples_dir.join("sample_non_ext.rs");
        fs::write(&sample_file, "fn main() { println!(\"non-ext\"); }")?;

        // Create a dummy Cargo.toml in the current directory.
        let manifest_path = cwd.join("Cargo.toml");
        fs::write(
            &manifest_path,
            "[package]\nname = \"dummy\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )?;

        let sample = Example {
            name: "sample_non_ext".to_string(),
            display_name: "non-extended".to_string(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            kind: TargetKind::Example,
            extended: false,
        };

        let (folder_str, goto_arg) = compute_vscode_args(&sample);
        // In this case, we expect folder_str to end with "examples" and goto_arg to point to sample_non_ext.rs.
        assert!(folder_str.contains("examples"));
        assert!(goto_arg.unwrap().contains("sample_non_ext.rs"));
        fs::remove_file(manifest_path)?;
        fs::remove_file(sample_file)?;
        Ok(())
    }

    #[test]
    fn test_compute_vscode_args_extended_src_main() -> Result<(), Box<dyn std::error::Error>> {
        // Simulate an extended sample where Cargo.toml is in the sample folder and "src/main.rs" exists.
        let dir = tempdir()?;
        let sample_dir = dir.path().join("extended_sample");
        fs::create_dir_all(sample_dir.join("src"))?;
        let main_rs = sample_dir.join("src/main.rs");
        fs::write(&main_rs, "fn main() { println!(\"extended src main\"); }")?;
        let manifest_path = sample_dir.join("Cargo.toml");
        fs::write(
            &manifest_path,
            "[package]\nname = \"extended_sample\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )?;

        let sample = Example {
            name: "extended_sample".to_string(),
            display_name: "extended".to_string(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            kind: TargetKind::Example,
            extended: true,
        };

        let (folder_str, goto_arg) = compute_vscode_args(&sample);
        // The folder should be sample_dir/src since that's where main.rs is.
        assert!(folder_str.ends_with("src"));
        let goto = goto_arg.unwrap();
        // The goto argument should contain main.rs.
        assert!(goto.contains("main.rs"));
        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_compute_vscode_args_extended_main_rs() -> Result<(), Box<dyn std::error::Error>> {
        // Simulate an extended sample where "src/main.rs" does not exist, but "main.rs" exists.
        let dir = tempdir()?;
        let sample_dir = dir.path().join("extended_sample2");
        fs::create_dir_all(&sample_dir)?;
        let main_rs = sample_dir.join("main.rs");
        fs::write(&main_rs, "fn main() { println!(\"extended main\"); }")?;
        let manifest_path = sample_dir.join("Cargo.toml");
        fs::write(
            &manifest_path,
            "[package]\nname = \"extended_sample2\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )?;

        let sample = Example {
            name: "extended_sample2".to_string(),
            display_name: "extended2".to_string(),
            manifest_path: manifest_path.to_string_lossy().to_string(),
            kind: TargetKind::Example,
            extended: true,
        };

        let (folder_str, goto_arg) = compute_vscode_args(&sample);
        // The folder should be the sample_dir (since main.rs is directly in it).
        assert!(folder_str.ends_with("extended_sample2"));
        let goto = goto_arg.unwrap();
        assert!(goto.contains("main.rs"));
        dir.close()?;
        Ok(())
    }
}
