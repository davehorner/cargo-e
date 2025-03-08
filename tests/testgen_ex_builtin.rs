// Below is an integration test that sets up a temporary project with a generated Cargo.toml 
// and a single example file (named according to the ex_name variable) that prints its own name 
// followed by " HAS RUN SUCCESSFULLY". The test then runs your binary with the explicit example name.
// It verifies that:
// 1. The example file exists on the filesystem.
// 2. The output (via stdout) contains the expected success message.
//
// This test uses a single variable for the example name so that we don't repeat static strings.

use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use tempfile::tempdir;

#[test]
fn testgen_ex_builtin() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory to isolate the test.
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();
    env::set_current_dir(&temp_path)?;

    // Define the example name once.
    let ex_name = "testgen_ex_builtin";
    let expected_output = format!("{} HAS RUN SUCCESSFULLY", ex_name);

    // Create a Cargo.toml file with a package name set to ex_name.
    let manifest_path = temp_path.join("Cargo.toml");
    fs::write(
        &manifest_path,
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            ex_name
        ),
    )?;

    // Create an examples directory and an example file named "<ex_name>.rs".
    let examples_dir = temp_path.join("examples");
    fs::create_dir_all(&examples_dir)?;
    let example_filename = format!("{}.rs", ex_name);
    let example_file = examples_dir.join(&example_filename);
    // Write the example file which prints the expected output.
    fs::write(
        &example_file,
        &format!("fn main() {{ println!(\"{}\"); }}", expected_output),
    )?;

    // Verify that the example file exists.
    assert!(
        example_file.exists(),
        "Example file {:?} does not exist on the filesystem",
        example_file
    );

    // Run the binary with the explicit example name.
    let mut cmd = Command::cargo_bin("cargo-e")?;
    cmd.arg(ex_name);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(&expected_output));

    Ok(())
}

