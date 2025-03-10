use cargo_e::e_command_builder::{CargoCommandBuilder, CargoTarget, TargetKind, TargetOrigin};
use std::path::PathBuf;

fn main() {
    let target = CargoTarget {
        name: "my_example".to_string(),
        display_name: "My Example".to_string(),
        manifest_path: "Cargo.toml".to_string(),
        kind: TargetKind::Example,
        extended: true,
        origin: Some(TargetOrigin::SingleFile(PathBuf::from(
            "examples/my_example.rs",
        ))),
    };

    let args = CargoCommandBuilder::new()
        .with_target(&target)
        .with_extra_args(&vec!["--flag".to_string(), "value".to_string()])
        .build();

    println!("Built Cargo command: {:?}", args);
}
