#![allow(dead_code)]
use std::fs;
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// A wrapper around a temporary project directory.
pub struct TestProject {
    /// The temporary directory. When this is dropped, the directory and its contents are removed.
    pub temp_dir: TempDir,
    /// The root directory for the generated project.
    pub root: PathBuf,
}

impl TestProject {
    /// Create a new project with the given name.
    pub fn new(project_name: &str) -> IoResult<Self> {
        let temp_dir = tempdir()?;
        let root = temp_dir.path().join(project_name);
        fs::create_dir_all(&root)?;
        // Create a Cargo.toml file in the project root.
        let cargo_toml = root.join("Cargo.toml");
        fs::write(
            &cargo_toml,
            format!(
                "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
                project_name
            ),
        )?;
        Ok(TestProject { temp_dir, root })
    }

    /// Returns a reference to the project root.
    pub fn path(&self) -> &Path {
        &self.root
    }
}

/// Create a testgen_ex project: a project with a single example target located in the `examples` folder.
pub fn create_testgen_ex_project(ex_name: &str) -> IoResult<TestProject> {
    let project = TestProject::new(ex_name)?;
    let examples_dir = project.root.join("examples");
    fs::create_dir_all(&examples_dir)?;
    let ex_file = examples_dir.join(format!("{}.rs", ex_name));
    // The example prints its own name followed by " HAS RUN SUCCESSFULLY"
    fs::write(
        &ex_file,
        format!(
            "fn main() {{ println!(\"{} HAS RUN SUCCESSFULLY\"); }}",
            ex_name
        ),
    )?;
    Ok(project)
}

/// Create a testgen_bin project: a project with a single binary target located in the `src/bin` folder.
pub fn create_testgen_bin_project(bin_name: &str) -> IoResult<TestProject> {
    let project = TestProject::new(bin_name)?;
    let bin_dir = project.root.join("src").join("bin");
    fs::create_dir_all(&bin_dir)?;
    let bin_file = bin_dir.join(format!("{}.rs", bin_name));
    fs::write(
        &bin_file,
        format!(
            "fn main() {{ println!(\"{} HAS RUN SUCCESSFULLY\"); }}",
            bin_name
        ),
    )?;
    Ok(project)
}

/// Create a testgen_ext project: a project with an extended example target.
/// Here, the extended example is placed in the subfolder `examples/extended`.
#[cfg(test)]
pub fn create_testgen_ext_project(ex_name: &str) -> IoResult<TestProject> {
    let project = TestProject::new(ex_name)?;
    let ext_dir = project.root.join("examples").join("extended");
    fs::create_dir_all(&ext_dir)?;
    let ext_file = ext_dir.join(format!("{}.rs", ex_name));
    fs::write(
        &ext_file,
        format!(
            "fn main() {{ println!(\"{} HAS RUN SUCCESSFULLY\"); }}",
            ex_name
        ),
    )?;
    Ok(project)
}

/// Count the number of sample targets in a project directory.
///
/// This utility counts:
/// 1. All `.rs` files in the `examples` folder (excluding those in an `extended` subfolder),
/// 2. All `.rs` files in the `examples/extended` folder, and
/// 3. All `.rs` files in the `src/bin` folder.
pub fn count_samples(project_root: &Path) -> usize {
    let mut count = 0;

    // Count examples in "examples", but exclude subfolder "extended"
    let examples_dir = project_root.join("examples");
    if examples_dir.exists() {
        if let Ok(entries) = fs::read_dir(&examples_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    count += 1;
                }
            }
        }
        // Count extended examples in "examples/extended"
        let ext_dir = examples_dir.join("extended");
        if ext_dir.exists() {
            if let Ok(entries) = fs::read_dir(&ext_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                        count += 1;
                    }
                }
            }
        }
    }

    // Count binaries in "src/bin"
    let bin_dir = project_root.join("src").join("bin");
    if bin_dir.exists() {
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    count += 1;
                }
            }
        }
    }

    count
}
