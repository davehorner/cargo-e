// This integration test exercises the support functions in our test support module,
// ensuring that the functions to generate testgen_ex, testgen_bin, and testgen_ext projects work,
// and that the sample counting utility returns the expected values.

mod common {
    pub mod test_utils;
}
use common::test_utils::{
    count_samples, create_testgen_bin_project, create_testgen_ex_project,
    create_testgen_ext_project,
};

#[test]
fn exercise_test_support_functions() {
    // Test the creation of a testgen_ex project.
    let ex_name = "ex_project";
    let ex_project =
        create_testgen_ex_project(ex_name).expect("Failed to create testgen_ex project");

    // Verify that the Cargo.toml exists in the project root.
    let manifest_path = ex_project.path().join("Cargo.toml");
    assert!(
        manifest_path.exists(),
        "Cargo.toml should exist in testgen_ex project"
    );

    // Verify that the example file exists.
    let example_path = ex_project
        .path()
        .join("examples")
        .join(format!("{}.rs", ex_name));
    assert!(
        example_path.exists(),
        "Example file {:?} should exist in testgen_ex project",
        example_path
    );

    // Count samples: testgen_ex project should have exactly 1 example.
    let ex_sample_count = count_samples(ex_project.path());
    assert_eq!(
        ex_sample_count, 1,
        "Expected 1 sample target in testgen_ex project, found {}",
        ex_sample_count
    );

    // Test the creation of a testgen_bin project.
    let bin_name = "bin_project";
    let bin_project =
        create_testgen_bin_project(bin_name).expect("Failed to create testgen_bin project");

    // Verify that the binary file exists.
    let bin_path = bin_project
        .path()
        .join("src")
        .join("bin")
        .join(format!("{}.rs", bin_name));
    assert!(
        bin_path.exists(),
        "Binary file {:?} should exist in testgen_bin project",
        bin_path
    );

    // Count samples: testgen_bin project should have exactly 1 target (the binary).
    let bin_sample_count = count_samples(bin_project.path());
    assert_eq!(
        bin_sample_count, 1,
        "Expected 1 sample target in testgen_bin project, found {}",
        bin_sample_count
    );

    // Test the creation of a testgen_ext project.
    let ext_name = "ext_project";
    let ext_project =
        create_testgen_ext_project(ext_name).expect("Failed to create testgen_ext project");

    // Verify that the extended example file exists.
    let ext_path = ext_project
        .path()
        .join("examples")
        .join("extended")
        .join(format!("{}.rs", ext_name));
    assert!(
        ext_path.exists(),
        "Extended example file {:?} should exist in testgen_ext project",
        ext_path
    );

    // Count samples: testgen_ext project should have exactly 1 sample target (the extended example).
    let ext_sample_count = count_samples(ext_project.path());
    assert_eq!(
        ext_sample_count, 1,
        "Expected 1 sample target in testgen_ext project, found {}",
        ext_sample_count
    );
}

#[test]
fn exercise_all_test_support_functions() -> Result<(), Box<dyn std::error::Error>> {
    let ex_project = create_testgen_ex_project("ex_proj")?;
    let bin_project = create_testgen_bin_project("bin_proj")?;
    let ext_project = create_testgen_ext_project("ext_proj")?;

    // Ensure each project created a Cargo.toml.
    for project in [&ex_project, &bin_project, &ext_project] {
        assert!(project.path().join("Cargo.toml").exists());
    }

    // Use count_samples to at least exercise its functionality.
    let ex_samples = count_samples(ex_project.path());
    let bin_samples = count_samples(bin_project.path());
    let ext_samples = count_samples(ext_project.path());

    // We expect each project to have exactly 1 sample target.
    assert_eq!(ex_samples, 1);
    assert_eq!(bin_samples, 1);
    assert_eq!(ext_samples, 1);

    Ok(())
}
