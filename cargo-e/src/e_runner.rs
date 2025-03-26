use crate::prelude::*;
// #[cfg(not(feature = "equivalent"))]
// use ctrlc;
use once_cell::sync::Lazy;

// Global shared container for the currently running child process.
pub static GLOBAL_CHILD: Lazy<Arc<Mutex<Option<Child>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

/// Registers a global Ctrl+C handler once.
/// The handler checks GLOBAL_CHILD and kills the child process if present.
pub fn register_ctrlc_handler() -> Result<(), Box<dyn Error>> {
    ctrlc::set_handler(move || {
        let mut child_lock = GLOBAL_CHILD.lock().unwrap();
        if let Some(child) = child_lock.as_mut() {
            eprintln!("Ctrl+C pressed, terminating running child process...");
            let _ = child.kill();
        } else {
            eprintln!("Ctrl+C pressed, no child process running. Exiting nicely.");
            exit(0);
        }
    })?;
    Ok(())
}

/// In "equivalent" mode, behave exactly like "cargo run --example <name>"
#[cfg(feature = "equivalent")]
pub fn run_equivalent_example(
    cli: &crate::Cli,
) -> Result<std::process::ExitStatus, Box<dyn Error>> {
    // In "equivalent" mode, behave exactly like "cargo run --example <name>"
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--example",
        cli.explicit_example.as_deref().unwrap_or(""),
    ]);
    if !cli.extra.is_empty() {
        cmd.arg("--").args(cli.extra.clone());
    }
    // Inherit the standard input (as well as stdout/stderr) so that input is passed through.
    use std::process::Stdio;
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}

/// Runs the given example (or binary) target.
pub fn run_example(
    cli: &crate::Cli,
    target: &crate::e_target::CargoTarget,
) -> anyhow::Result<std::process::ExitStatus> {
    // Retrieve the current package name at compile time.
    let current_bin = env!("CARGO_PKG_NAME");

    // Avoid running our own binary.
    if target.kind == crate::e_target::TargetKind::Binary && target.name == current_bin {
        return Err(anyhow::anyhow!(
            "Skipping automatic run: {} is the same as the running binary",
            target.name
        ));
    }

    // Build the command using the CargoCommandBuilder.
    let mut builder = crate::e_command_builder::CargoCommandBuilder::new()
        .with_target(target)
        .with_required_features(&target.manifest_path, target)
        .with_cli(cli);

    if !cli.extra.is_empty() {
        builder = builder.with_extra_args(&cli.extra);
    }

    // Build the command.
    let mut cmd = builder.clone().build_command();

    // Before spawning, determine the directory to run from.
    // If a custom execution directory was set (e.g. for Tauri targets), that is used.
    // Otherwise, if the target is extended, run from its parent directory.
    if let Some(exec_dir) = builder.execution_dir {
        cmd.current_dir(exec_dir);
    } else if target.extended {
        if let Some(dir) = target.manifest_path.parent() {
            cmd.current_dir(dir);
        }
    }

    // Print the full command for debugging.
    let full_command = format!(
        "{} {}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("Running: {}", full_command);

    // Check if the manifest triggers the workspace error.
    let maybe_backup = crate::e_manifest::maybe_patch_manifest_for_run(&target.manifest_path)?;

    // Spawn the process.
    let child = cmd.spawn()?;
    {
        let mut global = GLOBAL_CHILD.lock().unwrap();
        *global = Some(child);
    }
    let status = {
        let mut global = GLOBAL_CHILD.lock().unwrap();
        if let Some(mut child) = global.take() {
            child.wait()?
        } else {
            return Err(anyhow::anyhow!("Child process missing"));
        }
    };

    // Restore the manifest if we patched it.
    if let Some(original) = maybe_backup {
        fs::write(&target.manifest_path, original)?;
    }

    Ok(status)
}
// /// Runs an example or binary target, applying a temporary manifest patch if a workspace error is detected.
// /// This function uses the same idea as in the collection helpers: if the workspace error is found,
// /// we patch the manifest, run the command, and then restore the manifest.
// pub fn run_example(
//     target: &crate::e_target::CargoTarget,
//     extra_args: &[String],
// ) -> Result<std::process::ExitStatus, Box<dyn Error>> {
//     // Retrieve the current package name (or binary name) at compile time.

//     use crate::e_target::TargetKind;

//     let current_bin = env!("CARGO_PKG_NAME");

//     // Avoid running our own binary if the target's name is the same.
//     if target.kind == TargetKind::Binary && target.name == current_bin {
//         return Err(format!(
//             "Skipping automatic run: {} is the same as the running binary",
//             target.name
//         )
//         .into());
//     }

//     let mut cmd = Command::new("cargo");
//     // Determine which manifest file is used.
//     let manifest_path: PathBuf;

//     match target.kind {
//         TargetKind::Bench => {
//             manifest_path = PathBuf::from(target.manifest_path.clone());
//             cmd.args([
//                 "bench",
//                 "--bench",
//                 &target.name,
//                 "--manifest-path",
//                 &target.manifest_path.to_str().unwrap_or_default().to_owned(),
//             ]);
//         }
//         TargetKind::Test => {
//             manifest_path = PathBuf::from(target.manifest_path.clone());
//             cmd.args([
//                 "test",
//                 "--test",
//                 &target.name,
//                 "--manifest-path",
//                 &target.manifest_path.to_str().unwrap_or_default().to_owned(),
//             ]);
//         }
//         TargetKind::Manifest => {
//             manifest_path = PathBuf::from(target.manifest_path.clone());
//             cmd.args([
//                 "run",
//                 "--release",
//                 "--manifest-path",
//                 &target.manifest_path.to_str().unwrap_or_default().to_owned(),
//                 "-p",
//                 &target.name,
//             ]);
//         }
//         TargetKind::Example => {
//             if target.extended {
//                 println!(
//                     "Running extended example in folder: examples/{}",
//                     target.name
//                 );
//                 // For extended examples, assume the manifest is inside the example folder.
//                 manifest_path = PathBuf::from(format!("examples/{}/Cargo.toml", target.name));
//                 cmd.arg("run")
//                     .current_dir(format!("examples/{}", target.name));
//             } else {
//                 manifest_path = PathBuf::from(crate::locate_manifest(false)?);
//                 cmd.args([
//                     "run",
//                     "--release",
//                     "--example",
//                     &target.name,
//                     "--manifest-path",
//                     &target.manifest_path.to_str().unwrap_or_default().to_owned(),
//                 ]);
//             }
//         }
//         TargetKind::Binary => {
//             println!("Running binary: {}", target.name);
//             manifest_path = PathBuf::from(crate::locate_manifest(false)?);
//             cmd.args([
//                 "run",
//                 "--release",
//                 "--bin",
//                 &target.name,
//                 "--manifest-path",
//                 &target.manifest_path.to_str().unwrap_or_default().to_owned(),
//             ]);
//         }
//         TargetKind::ExtendedBinary => {
//             println!("Running extended binary: {}", target.name);
//             manifest_path = PathBuf::from(crate::locate_manifest(false)?);
//             cmd.args([
//                 "run",
//                 "--release",
//                 "--manifest-path",
//                 &target.manifest_path.to_str().unwrap_or_default().to_owned(),
//                 "--bin",
//                 &target.name,
//             ]);
//         }
//         TargetKind::ExtendedExample => {
//             println!("Running extended example: {}", target.name);
//             manifest_path = PathBuf::from(crate::locate_manifest(false)?);
//             cmd.args([
//                 "run",
//                 "--release",
//                 "--manifest-path",
//                 &target.manifest_path.to_str().unwrap_or_default().to_owned(),
//                 "--example",
//                 &target.name,
//             ]);
//         }
//         TargetKind::ManifestTauri => {
//             println!("Running tauri: {}", target.name);
//             // For a Tauri example, run `cargo tauri dev`
//             manifest_path = PathBuf::from(target.manifest_path.clone());
//             let manifest_dir = PathBuf::from(manifest_path.parent().expect("expected a parent"));
//             // Start a new command for tauri dev
//             cmd.arg("tauri").arg("dev").current_dir(manifest_dir); // run from the folder where Cargo.toml is located
//         }
//         TargetKind::ManifestDioxus => {
//             println!("Running dioxus: {}", target.name);
//             cmd = Command::new("dx");
//             // For a Tauri example, run `cargo tauri dev`
//             manifest_path = PathBuf::from(target.manifest_path.clone());
//             let manifest_dir = PathBuf::from(manifest_path.parent().expect("expected a parent"));
//             // Start a new command for tauri dev
//             cmd.arg("serve").current_dir(manifest_dir); // run from the folder where Cargo.toml is located
//         }
//         TargetKind::ManifestDioxusExample => {
//             println!("Running dioxus: {}", target.name);
//             cmd = Command::new("dx");
//             // For a Tauri example, run `cargo tauri dev`
//             manifest_path = PathBuf::from(target.manifest_path.clone());
//             let manifest_dir = PathBuf::from(manifest_path.parent().expect("expected a parent"));
//             // Start a new command for tauri dev
//             cmd.arg("serve")
//                 .arg("--example")
//                 .arg(&target.name)
//                 .current_dir(manifest_dir); // run from the folder where Cargo.toml is located
//         }
//     }

//     // --- Add required-features support ---
//     // This call will search the provided manifest, and if it's a workspace,
//     // it will search workspace members for the target.
//     if let Some(features) = crate::e_manifest::get_required_features_from_manifest(
//         manifest_path.as_path(),
//         &target.kind,
//         &target.name,
//     ) {
//         cmd.args(&["--features", &features]);
//     }
//     // --- End required-features support ---

//     if !extra_args.is_empty() {
//         cmd.arg("--").args(extra_args);
//     }

//     let full_command = format!(
//         "{} {}",
//         cmd.get_program().to_string_lossy(),
//         cmd.get_args()
//             .map(|arg| arg.to_string_lossy())
//             .collect::<Vec<_>>()
//             .join(" ")
//     );
//     println!("Running: {}", full_command);

//     // Before spawning, check if the manifest triggers the workspace error.
//     // If so, patch it temporarily.
//     let maybe_backup = crate::e_manifest::maybe_patch_manifest_for_run(&manifest_path)?;

//     // Spawn the process.
//     let child = cmd.spawn()?;
//     {
//         let mut global = GLOBAL_CHILD.lock().unwrap();
//         *global = Some(child);
//     }
//     let status = {
//         let mut global = GLOBAL_CHILD.lock().unwrap();
//         if let Some(mut child) = global.take() {
//             child.wait()?
//         } else {
//             return Err("Child process missing".into());
//         }
//     };

//     // Restore the manifest if we patched it.
//     if let Some(original) = maybe_backup {
//         fs::write(&manifest_path, original)?;
//     }

//     //    println!("Process exited with status: {:?}", status.code());
//     Ok(status)
// }
/// Helper function to spawn a cargo process.
/// On Windows, this sets the CREATE_NEW_PROCESS_GROUP flag.
pub fn spawn_cargo_process(args: &[&str]) -> Result<Child, Box<dyn Error>> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        let child = Command::new("cargo")
            .args(args)
            .creation_flags(CREATE_NEW_PROCESS_GROUP)
            .spawn()?;
        Ok(child)
    }
    #[cfg(not(windows))]
    {
        let child = Command::new("cargo").args(args).spawn()?;
        Ok(child)
    }
}
