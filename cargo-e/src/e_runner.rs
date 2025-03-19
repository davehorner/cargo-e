use crate::prelude::*;
// #[cfg(not(feature = "equivalent"))]
// use ctrlc;
use crate::Example;
use once_cell::sync::Lazy;

// Global shared container for the currently running child process.
static GLOBAL_CHILD: Lazy<Arc<Mutex<Option<Child>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

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
pub fn run_example(
    example: &Example,
    extra_args: &[String],
) -> Result<std::process::ExitStatus, Box<dyn Error>> {
    // In "equivalent" mode, behave exactly like "cargo run --example <name>"
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--example", &example.name]);
    if !extra_args.is_empty() {
        cmd.arg("--").args(extra_args);
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
#[cfg(not(feature = "equivalent"))]
/// Runs an example or binary target, applying a temporary manifest patch if a workspace error is detected.
/// This function uses the same idea as in the collection helpers: if the workspace error is found,
/// we patch the manifest, run the command, and then restore the manifest.
pub fn run_example(
    target: &Example,
    extra_args: &[String],
) -> Result<std::process::ExitStatus, Box<dyn Error>> {
    // Retrieve the current package name (or binary name) at compile time.
    let current_bin = env!("CARGO_PKG_NAME");

    // Avoid running our own binary if the target's name is the same.
    if target.kind == crate::TargetKind::Binary && target.name == current_bin {
        return Err(format!(
            "Skipping automatic run: {} is the same as the running binary",
            target.name
        )
        .into());
    }

    let mut cmd = Command::new("cargo");
    // Determine which manifest file is used.
    let manifest_path: PathBuf;

    match target.kind {
        crate::TargetKind::Example => {
            if target.extended {
                println!(
                    "Running extended example in folder: examples/{}",
                    target.name
                );
                // For extended examples, assume the manifest is inside the example folder.
                manifest_path = PathBuf::from(format!("examples/{}/Cargo.toml", target.name));
                cmd.arg("run")
                    .current_dir(format!("examples/{}", target.name));
            } else {
                manifest_path = PathBuf::from(crate::locate_manifest(false)?);
                cmd.args(["run", "--release", "--example", &target.name]);
            }
        }
        crate::TargetKind::Binary => {
            println!("Running binary: {}", target.name);
            manifest_path = PathBuf::from(crate::locate_manifest(false)?);
            cmd.args(["run", "--release", "--bin", &target.name]);
        }
    }

    if !extra_args.is_empty() {
        cmd.arg("--").args(extra_args);
    }

    let full_command = format!(
        "cargo {}",
        cmd.get_args()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("Running: {}", full_command);

    // Before spawning, check if the manifest triggers the workspace error.
    // If so, patch it temporarily.
    let maybe_backup = crate::e_manifest::maybe_patch_manifest_for_run(&manifest_path)?;

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
            return Err("Child process missing".into());
        }
    };

    // Restore the manifest if we patched it.
    if let Some(original) = maybe_backup {
        fs::write(&manifest_path, original)?;
    }

    //    println!("Process exited with status: {:?}", status.code());
    Ok(status)
}
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
