use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
mod common {
    pub mod test_utils;
}
use common::test_utils::{create_testgen_ex_project, count_samples};


#[test]
fn test_sample_count_in_testgen_ex_project() -> Result<(), Box<dyn std::error::Error>> {
    // Create a testgen_ex project with a specific example name.
    let ex_name = "testgen_ex_builtin";
    let project = create_testgen_ex_project(ex_name)?;

    // Change the current working directory to the project root.
    env::set_current_dir(project.path())?;

    // Count samples in the project.
    let sample_count = count_samples(project.path());
    // In a testgen_ex project, we expect exactly 1 example target.
    assert_eq!(sample_count, 1, "Expected 1 sample target, got {}", sample_count);

    // Optionally, run your binary here to verify it picks up the one sample.
    let mut cmd = Command::cargo_bin("cargo-e")?;
    // Passing the example name explicitly to force it to run.
    cmd.arg(ex_name)
       .assert()
       .success()
       .stdout(predicate::str::contains(format!("{} HAS RUN SUCCESSFULLY", ex_name)));

    Ok(())
}

