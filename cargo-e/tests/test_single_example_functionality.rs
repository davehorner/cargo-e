// Below is an integration test that sets up a temporary project with a dummy Cargo.toml and a single example file
// (that prints "non-ext"). The test then runs your binary with no additional arguments. Depending on whether the
// equivalent feature is enabled, it expects different output:
//
// In equivalent mode, since no explicit example name is given, the binary will forward to Cargo and Cargo will complain
// that --example takes one argument (and list the available examples).
// In non‑equivalent mode, the binary will automatically run the single example and output "non-ext".

use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_single_example_functionality() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory to isolate the test.
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Change the current working directory to the temporary directory.
    env::set_current_dir(temp_path)?;

    // Create a dummy Cargo.toml in the temporary directory.
    let manifest_path = temp_path.join("Cargo.toml");
    fs::write(
        &manifest_path,
        "[package]\nname = \"dummy\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;

    // Create an examples directory with a single example file.
    let examples_dir = temp_path.join("examples");
    fs::create_dir_all(&examples_dir)?;
    let example_file = examples_dir.join("sample_non_ext.rs");
    fs::write(&example_file, "fn main() { println!(\"non-ext\"); }")?;

    // Run the binary with no arguments.
    let mut cmd = Command::cargo_bin("cargo-e")?;
    let assert = cmd.assert();

    if cfg!(feature = "equivalent") {
        // In equivalent mode, without an explicit example name,
        // Cargo should error about "--example" missing its argument.
        assert
            .failure()
            .stderr(predicate::str::contains(
                "error: \"--example\" takes one argument",
            ))
            .stderr(predicate::str::contains("Available examples:"))
            .stderr(predicate::str::contains("sample_non_ext"));
    } else {
        // In non-equivalent mode, a single example should run automatically.
        // Thus, the output should contain "non-ext".
        assert
            .success()
            .stdout(predicate::str::contains("[ex.] sample_non_ext"));
    }

    Ok(())
}
