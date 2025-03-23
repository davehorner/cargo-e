use crate::e_types::TargetKind;
use crate::Example;
use anyhow::{Context, Result};
use std::process::Command;

/// Prebuilds all given targets by invoking `cargo build` with the appropriate flags.
///
/// This function supports all target kinds:
/// - **Example:** Runs `cargo build --example <name>`
/// - **ExtendedExample:** Runs `cargo build --example <name> --manifest-path <manifest_path>`
/// - **Binary:** Runs `cargo build --bin <name>`
/// - **ExtendedBinary:** Runs `cargo build --bin <name> --manifest-path <manifest_path>`
///
/// # Parameters
///
/// - `targets`: A slice of `Example` instances representing the targets to prebuild.
///
/// # Returns
///
/// Returns `Ok(())` if all targets build successfully; otherwise, returns an error.
pub fn prebuild_examples(targets: &[Example]) -> Result<()> {
    for target in targets {
        // Determine the build flag and whether to include the manifest path
        let (build_flag, use_manifest) = match target.kind {
            TargetKind::Example => ("--example", false),
            TargetKind::ExtendedExample => ("--example", true),
            TargetKind::Binary => ("--bin", false),
            TargetKind::ExtendedBinary => ("--bin", true),
        };

        println!("Prebuilding target [{}]: {}", build_flag, target.name);

        let mut command = Command::new("cargo");
        command.arg("build").arg(build_flag).arg(&target.name);

        if use_manifest {
            command.args(&["--manifest-path", &target.manifest_path]);
        }

        let status = command
            .status()
            .with_context(|| format!("Cargo build failed for target {}", target.name))?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Prebuild failed for target {}",
                target.name
            ));
        }
    }
    Ok(())
}
