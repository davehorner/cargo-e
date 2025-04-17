use anyhow::{Context, Result};
use std::process::Command;

use crate::e_target::{CargoTarget, TargetKind};

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
pub fn prebuild_examples(targets: &[CargoTarget]) -> Result<()> {
    for target in targets {
        // Determine the build flag and whether to include the manifest path
        let (build_flag, use_manifest) = match target.kind {
            TargetKind::Example => ("--example", false),
            TargetKind::ExtendedExample => ("--example", true),
            TargetKind::Binary => ("--bin", false),
            TargetKind::ExtendedBinary => ("--bin", true),
            TargetKind::ManifestTauriExample => ("", true),
            TargetKind::ManifestTauri => ("", true),
            TargetKind::Test => ("--test", true),
            TargetKind::Manifest => ("", true),
            TargetKind::Bench => ("", true),
            TargetKind::ManifestDioxus => ("", true),
            TargetKind::ManifestDioxusExample => ("", true),
            TargetKind::ManifestLeptos => ("", true),
            TargetKind::Unknown => ("", true),
            _ => ("", true),
        };

        if build_flag.is_empty() {
            return Ok(());
        }
        println!("Prebuilding target [{}]: {}", build_flag, target.name);

        let mut command = Command::new("cargo");
        command.arg("build").arg(build_flag).arg(&target.name);

        if use_manifest {
            command.args(&[
                "--manifest-path",
                &target.manifest_path.to_str().unwrap_or_default().to_owned(),
            ]);
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
