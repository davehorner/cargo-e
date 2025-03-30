use cargo_e::{
    e_command_builder::CargoCommandBuilder,
    e_target::{CargoTarget, TargetKind, TargetOrigin},
};
use std::path::PathBuf;

#[test]
fn integration_test_builder() {
    let target = CargoTarget {
        name: "my_example".to_string(),
        display_name: "My Example".to_string(),
        manifest_path: "Cargo.toml".into(),
        kind: TargetKind::Example,
        extended: true,
        toml_specified: false,
        origin: Some(TargetOrigin::SingleFile(PathBuf::from(
            "examples/my_example.rs",
        ))),
    };

    let args = CargoCommandBuilder::new()
        .with_target(&target)
        .with_extra_args(&["--flag".to_string(), "value".to_string()])
        .build();

    assert!(args.contains(&"run".to_string()));
}
