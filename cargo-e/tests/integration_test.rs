// Example integration test file in tests/integration_test.rs
mod common;

// use common::test_prelude::*;
use common::test_testgen::project_setup::{create_testgen_bin_project, create_testgen_ex_project};

#[test]
fn test_example_project() {
    let project = create_testgen_ex_project("example_project").unwrap();
    // Use assert_cmd or other prelude items from test_prelude
    assert!(project.path().join("Cargo.toml").exists());
}

#[test]
fn test_create_testgen_bin_project() -> std::io::Result<()> {
    // Use a known binary name for testing.
    let bin_name = "test_bin";
    // Create the test binary project.
    let project = create_testgen_bin_project(bin_name)?;

    // Verify that the src/bin directory exists.
    let bin_dir = project.path().join("src").join("bin");
    assert!(bin_dir.exists(), "The src/bin directory should exist");

    // Verify that the binary file exists.
    let bin_file = bin_dir.join(format!("{}.rs", bin_name));
    assert!(bin_file.exists(), "The binary file should exist");

    // Read the file and verify its contents.
    let contents = std::fs::read_to_string(&bin_file)?;
    let expected = format!(
        "fn main() {{ println!(\"{} HAS RUN SUCCESSFULLY\"); }}",
        bin_name
    );
    assert_eq!(
        contents, expected,
        "The file content should match the expected output"
    );

    Ok(())
}
