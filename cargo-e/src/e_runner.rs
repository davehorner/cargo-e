use crate::e_processmanager::ProcessManager;
use crate::{e_target::TargetOrigin, prelude::*};
// #[cfg(not(feature = "equivalent"))]
// use ctrlc;
use crate::e_cargocommand_ext::CargoProcessHandle;
use crate::e_target::CargoTarget;
use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use which::which; // Adjust the import based on your project structure

// lazy_static! {
//     pub static ref GLOBAL_CHILDREN: Arc<Mutex<Vec<Arc<CargoProcessHandle>>>> = Arc::new(Mutex::new(Vec::new()));
//     static CTRL_C_COUNT: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
// }

// pub static GLOBAL_CHILDREN:     Lazy<Arc<Mutex<Vec<Arc<Mutex<CargoProcessHandle>>>>>> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));
pub static GLOBAL_CHILDREN: Lazy<Arc<Mutex<HashMap<u32, Arc<Mutex<CargoProcessHandle>>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static CTRL_C_COUNT: AtomicUsize = AtomicUsize::new(0);

// Global shared container for the currently running child process.
// pub static GLOBAL_CHILD: Lazy<Arc<Mutex<Option<Child>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
// static CTRL_C_COUNT: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));

// pub static GLOBAL_CHILDREN: Lazy<Arc<Mutex<VecDeque<CargoProcessHandle>>>> = Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));
/// Resets the Ctrl+C counter.
/// This can be called to reset the count when starting a new program or at any other point.
pub fn reset_ctrl_c_count() {
    CTRL_C_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
}

// pub fn kill_last_process() -> Result<()> {
//     let mut global = GLOBAL_CHILDREN.lock().unwrap();

//     if let Some(mut child_handle) = global.pop_back() {
//         // Kill the most recent process
//         eprintln!("Killing the most recent child process...");
//         let _ = child_handle.kill();
//         Ok(())
//     } else {
//         eprintln!("No child processes to kill.");
//         Err(anyhow::anyhow!("No child processes to kill").into())
//     }
// }

pub fn take_process_results(pid: u32) -> Option<CargoProcessHandle> {
    let mut global = GLOBAL_CHILDREN.lock().ok()?;
    // Take ownership
    // let handle = global.remove(&pid)?;
    // let mut handle = handle.lock().ok()?;
    let handle = global.remove(&pid)?;
    // global.remove(&pid)
    // This will succeed only if no other Arc exists
    Arc::try_unwrap(handle)
        .ok()? // fails if other Arc exists
        .into_inner()
        .ok() // fails if poisoned
}

pub fn get_process_results_in_place(
    pid: u32,
) -> Option<crate::e_cargocommand_ext::CargoProcessResult> {
    let global = GLOBAL_CHILDREN.lock().ok()?; // MutexGuard<HashMap>
    let handle = global.get(&pid)?.clone(); // Arc<Mutex<CargoProcessHandle>>
    let handle = handle.lock().ok()?; // MutexGuard<CargoProcessHandle>
    Some(handle.result.clone()) // ✅ return the result field
}

// /// Registers a global Ctrl+C handler that interacts with the `GLOBAL_CHILDREN` process container.
// pub fn register_ctrlc_handler() -> Result<(), Box<dyn Error>> {
//     println!("Registering Ctrl+C handler...");
//     ctrlc::set_handler(move || {
//          let count = CTRL_C_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
//         {
//             eprintln!("Ctrl+C pressed");

//     // lock only ONE mutex safely
//     if let Ok(mut global) = GLOBAL_CHILDREN.try_lock() {
//             // let mut global = GLOBAL_CHILDREN.lock().unwrap();
//             eprintln!("Ctrl+C got lock on global container");

//             // If there are processes in the global container, terminate the most recent one
//             if let Some((pid, child_handle)) = global.iter_mut().next() {
//                 eprintln!("Ctrl+C pressed, terminating the child process with PID: {}", pid);

//                 // Lock the child process and kill it
//                 let mut child_handle = child_handle.lock().unwrap();
//                 if child_handle.requested_exit {
//                     eprintln!("Child process is already requested kill...");
//                 } else {
//                     eprintln!("Child process is not running, no need to kill.");
//                     child_handle.requested_exit=true;
//                     println!("Killing child process with PID: {}", pid);
//                     let _ = child_handle.kill();  // Attempt to kill the process
//                     println!("Killed child process with PID: {}", pid);

//                     reset_ctrl_c_count();
//                     return;  // Exit after successfully terminating the process
//                 }

//                 // Now remove the process from the global container
//                 // let pid_to_remove = *pid;

//                 // // Reacquire the lock after killing and remove the process from global
//                 // drop(global);  // Drop the first borrow

//                 // // Re-lock global and safely remove the entry using the pid
//                 // let mut global = GLOBAL_CHILDREN.lock().unwrap();
//                 // global.remove(&pid_to_remove); // Remove the process entry by PID
//                 // println!("Removed process with PID: {}", pid_to_remove);
//             }

//     } else {
//         eprintln!("Couldn't acquire GLOBAL_CHILDREN lock safely");
//     }

/// Registers a global Ctrl+C handler that uses the process manager.
pub fn register_ctrlc_handler(process_manager: Arc<ProcessManager>) -> Result<(), Box<dyn Error>> {
    println!("Registering Ctrl+C handler...");
    ctrlc::set_handler(move || {
        let count = CTRL_C_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        eprintln!("Ctrl+C pressed");

        // Use the process manager's API to handle killing
        match process_manager.kill_one() {
            Ok(true) => {
                eprintln!("Process was successfully terminated.");
                reset_ctrl_c_count();
                return; // Exit handler early after a successful kill.
            }
            Ok(false) => {
                eprintln!("No process was killed this time.");
            }
            Err(e) => {
                eprintln!("Error killing process: {:?}", e);
            }
        }

        // Handle Ctrl+C count logic for exiting the program.
        if count == 3 {
            eprintln!("Ctrl+C pressed 3 times with no child process running. Exiting.");
            std::process::exit(0);
        } else if count == 2 {
            eprintln!("Ctrl+C pressed 2 times, press one more to exit.");
        } else {
            eprintln!("Ctrl+C pressed {} times, no child process running.", count);
        }
    })?;
    Ok(())
}

//         }

//         // Now handle the Ctrl+C count and display messages
//         // If Ctrl+C is pressed 3 times without any child process, exit the program.
//         if count == 3 {
//             eprintln!("Ctrl+C pressed 3 times with no child process running. Exiting.");
//             std::process::exit(0);
//         } else if count == 2 {
//             // Notify that one more Ctrl+C will exit the program.
//             eprintln!("Ctrl+C pressed 2 times, press one more to exit.");
//         } else {
//             eprintln!("Ctrl+C pressed {} times, no child process running.", count);
//         }
//     })?;
//     Ok(())
// }

// /// Registers a global Ctrl+C handler once.
// /// The handler checks GLOBAL_CHILD and kills the child process if present.
// pub fn register_ctrlc_handler() -> Result<(), Box<dyn Error>> {
//     ctrlc::set_handler(move || {
//         let mut count_lock = CTRL_C_COUNT.lock().unwrap();
//         *count_lock += 1;

//         let count = *count_lock;

//         // If there is no child process and Ctrl+C is pressed 3 times, exit the program
//         if count == 3 {
//             eprintln!("Ctrl+C pressed 3 times with no child process running. Exiting.");
//             exit(0);
//         } else {
//             let mut child_lock = GLOBAL_CHILD.lock().unwrap();
//             if let Some(child) = child_lock.as_mut() {
//                 eprintln!(
//                     "Ctrl+C pressed {} times, terminating running child process...",
//                     count
//                 );
//                 let _ = child.kill();
//             } else {
//                 eprintln!("Ctrl+C pressed {} times, no child process running.", count);
//             }
//         }
//     })?;
//     Ok(())
// }

/// Asynchronously launches the GenAI summarization example for the given target.
/// It builds the command using the target's manifest path as the "origin" argument.
pub async fn open_ai_summarize_for_target(target: &CargoTarget) {
    // Extract the origin path from the target (e.g. the manifest path).
    let origin_path = match &target.origin {
        Some(TargetOrigin::SingleFile(path)) | Some(TargetOrigin::DefaultBinary(path)) => path,
        _ => return,
    };

    let exe_path = match which("cargoe_ai_summarize") {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Error: 'cargoe_ai_summarize' not found in PATH: {}", err);
            return;
        }
    };
    // Build the command based on the platform.
    // let mut cmd = if cfg!(target_os = "windows") {
    //     let command_str = format!(
    //         "e_ai_summarize --streaming --stdin {}",
    //         origin_path.as_os_str().to_string_lossy()
    //     );
    //     println!("Running command: {}", command_str);
    //     let mut command = Command::new("cmd");
    //     command.args(["/C", &command_str]);
    //     command
    // } else {
    let mut cmd = Command::new(exe_path);
    cmd.arg("--streaming");
    cmd.arg("--stdin");
    // cmd.arg(".");
    cmd.arg(origin_path);
    // command
    // };

    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // Spawn the command and wait for it to finish.
    let child = cmd.spawn();
    let status = child
        .expect("Failed to spawn command")
        .wait()
        .expect("Failed to wait for command");

    if !status.success() {
        eprintln!("Command exited with status: {}", status);
    }

    // // Build the command to run the example.
    // let output = if cfg!(target_os = "windows") {
    //     let command_str = format!("e_ai_summarize --stdin {}", origin_path.as_os_str().to_string_lossy());
    //     println!("Running command: {}", command_str);
    //     Command::new("cmd")
    //         .args([
    //             "/C",
    //             command_str.as_str(),
    //         ])
    //         .output()
    // } else {
    //     Command::new("e_ai_summarize")
    //         .args([origin_path])
    //         .output()
    // };

    // // Handle the output from the command.
    // match output {
    //     Ok(output) if output.status.success() => {
    //         // The summarization example ran successfully.
    //         println!("----
    //         {}", String::from_utf8_lossy(&output.stdout));
    //     }
    //     Ok(output) => {
    //         let msg = format!(
    //             "Error running summarization example:\nstdout: {}\nstderr: {}",
    //             String::from_utf8_lossy(&output.stdout),
    //             String::from_utf8_lossy(&output.stderr)
    //         );
    //         error!("{}", msg);
    //     }
    //     Err(e) => {
    //         let msg = format!("Failed to execute summarization command: {}", e);
    //         error!("{}", msg);
    //     }
    // }
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
    manager: Arc<ProcessManager>,
    cli: &crate::Cli,
    target: &crate::e_target::CargoTarget,
) -> anyhow::Result<Option<std::process::ExitStatus>> {
    crate::e_runall::set_rustflags_if_quiet(cli.quiet);
    // Retrieve the current package name at compile time.
    let current_bin = env!("CARGO_PKG_NAME");

    // Avoid running our own binary.
    if target.kind == crate::e_target::TargetKind::Binary && target.name == current_bin {
        println!(
            "Skipping automatic run: {} is the same as the running binary",
            target.name
        );
        return Ok(None);
    }
    // Handle plugin-provided targets by invoking their run_command
    #[cfg(feature = "uses_plugins")]
    {
        use std::process::Stdio;
        use crate::plugins::plugin_api::{load_plugins, Target as PluginTarget};
        use crate::e_target::TargetOrigin;

        if target.kind == crate::e_target::TargetKind::Plugin {
            let cwd = std::env::current_dir()?;
            if let Some(TargetOrigin::Plugin { plugin_path, reported }) = &target.origin {
                let pt = PluginTarget {
                    name: target.name.clone(),
                    metadata: Some(reported.to_string_lossy().to_string()),
                };
                for plugin in load_plugins()? {
                    if plugin.source().map(|s| std::path::PathBuf::from(s)) == Some(plugin_path.clone()) {
                        let mut cmd = plugin.run_command(&cwd, &pt)?;
                        cmd.stdin(Stdio::inherit())
                           .stdout(Stdio::inherit())
                           .stderr(Stdio::inherit());
                        let status = cmd.status()?;
                        return Ok(Some(status));
                    }
                }
            }
        }
    }

    let manifest_path = PathBuf::from(target.manifest_path.clone());
    // Build the command using the CargoCommandBuilder.
    let mut builder = crate::e_command_builder::CargoCommandBuilder::new(
        &manifest_path,
        &cli.subcommand,
        cli.filter,
    )
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
    if let Some(ref exec_dir) = builder.execution_dir {
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

    let pid = Arc::new(builder).run(|_pid, handle| {
        manager.register(handle);
    })?;
    let result = manager.wait(pid, None)?;
    // println!("HERE IS THE RESULT!{} {:?}",pid,manager.get(pid));
    // println!("\n\nHERE IS THE RESULT!{} {:?}",pid,result);
    if result.is_filter {
        result.print_exact();
        result.print_short();
        result.print_compact();

        // manager.print_shortened_output();
        manager.print_prefixed_summary();
        // manager.print_compact();
    }

    // let handle=    Arc::new(builder).run_wait()?;
    // Spawn the process.
    // let child = cmd.spawn()?;
    // {
    //     let mut global = GLOBAL_CHILD.lock().unwrap();
    //     *global = Some(child);
    // }
    // let status = {
    //     let mut global = GLOBAL_CHILD.lock().unwrap();
    //     if let Some(mut child) = global.take() {
    //         child.wait()?
    //     } else {
    //         return Err(anyhow::anyhow!("Child process missing"));
    //     }
    // };

    // Restore the manifest if we patched it.
    if let Some(original) = maybe_backup {
        fs::write(&target.manifest_path, original)?;
    }

    Ok(result.exit_status)
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
    // #[cfg(windows)]
    // {
    //     use std::os::windows::process::CommandExt;
    //     const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    //     let child = Command::new("cargo")
    //         .args(args)
    //         .creation_flags(CREATE_NEW_PROCESS_GROUP)
    //         .spawn()?;
    //     Ok(child)
    // }
    // #[cfg(not(windows))]
    // {
    let child = Command::new("cargo").args(args).spawn()?;
    Ok(child)
    // }
}

/// Returns true if the file's a "scriptisto"
pub fn is_active_scriptisto<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let file = File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    if !first_line.contains("scriptisto") || !first_line.starts_with("#") {
        return Ok(false);
    }
    Ok(true)
}

/// Returns true if the file's a "rust-script"
pub fn is_active_rust_script<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let file = File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    if !first_line.contains("rust-script") || !first_line.starts_with("#") {
        return Ok(false);
    }
    Ok(true)
}

/// Checks if `scriptisto` is installed and suggests installation if it's not.
pub fn check_scriptisto_installed() -> Result<std::path::PathBuf, Box<dyn Error>> {
    let r = which("scriptisto");
    match r {
        Ok(_) => {
            // installed
        }
        Err(e) => {
            // scriptisto is not found in the PATH
            eprintln!("scriptisto is not installed.");
            println!("Suggestion: To install scriptisto, run the following command:");
            println!("cargo install scriptisto");
            return Err(e.into());
        }
    }
    Ok(r?)
}

pub fn run_scriptisto<P: AsRef<Path>>(script_path: P, args: &[&str]) -> Option<Child> {
    let scriptisto = check_scriptisto_installed().ok()?;

    let script: &std::path::Path = script_path.as_ref();
    let child = Command::new(scriptisto)
        .arg(script)
        .args(args)
        .spawn()
        .ok()?;
    Some(child)
}

/// Checks if `rust-script` is installed and suggests installation if it's not.
pub fn check_rust_script_installed() -> Result<std::path::PathBuf, Box<dyn Error>> {
    let r = which("rust-script");
    match r {
        Ok(_) => {
            // rust-script is installed
        }
        Err(e) => {
            // rust-script is not found in the PATH
            eprintln!("rust-script is not installed.");
            println!("Suggestion: To install rust-script, run the following command:");
            println!("cargo install rust-script");
            return Err(e.into());
        }
    }
    Ok(r?)
}

pub fn run_rust_script<P: AsRef<Path>>(script_path: P, args: &[&str]) -> Option<Child> {
    let rust_script = check_rust_script_installed().ok()?;

    let script: &std::path::Path = script_path.as_ref();
    let child = Command::new(rust_script)
        .arg(script)
        .args(args)
        .spawn()
        .ok()?;
    Some(child)
}

pub fn run_rust_script_with_ctrlc_handling(explicit: String, extra_args: Vec<String>) {
    let explicit_path = Path::new(&explicit); // Construct Path outside the lock

    if explicit_path.exists() {
        let extra_str_slice: Vec<String> = extra_args.iter().cloned().collect();
        if let Ok(true) = is_active_rust_script(&explicit_path) {
            // Run the child process in a separate thread to allow Ctrl+C handling
            let handle = thread::spawn(move || {
                let extra_str_slice_cloned = extra_str_slice.clone();
                let mut child = run_rust_script(
                    &explicit,
                    &extra_str_slice_cloned
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                )
                .unwrap_or_else(|| {
                    eprintln!("Failed to run rust-script: {:?}", &explicit);
                    std::process::exit(1); // Exit with an error code
                });

                child.wait()
            });

            match handle.join() {
                Ok(_) => {
                    println!("Child process finished successfully.");
                }
                Err(_) => {
                    eprintln!("Child process took too long to finish. Exiting...");
                    std::process::exit(1); // Exit if the process takes too long
                }
            }
        }
    }
}

pub fn run_scriptisto_with_ctrlc_handling(explicit: String, extra_args: Vec<String>) {
    let relative: String = make_relative(Path::new(&explicit)).unwrap_or_else(|e| {
        eprintln!("Error computing relative path: {}", e);
        std::process::exit(1);
    });

    let explicit_path = Path::new(&relative);
    if explicit_path.exists() {
        // let extra_args = EXTRA_ARGS.lock().unwrap(); // Locking the Mutex to access the data
        let extra_str_slice: Vec<String> = extra_args.iter().cloned().collect();

        if let Ok(true) = is_active_scriptisto(&explicit_path) {
            // Run the child process in a separate thread to allow Ctrl+C handling
            let handle = thread::spawn(move || {
                let extra_str_slice_cloned: Vec<String> = extra_str_slice.clone();
                let mut child = run_scriptisto(
                    &relative,
                    &extra_str_slice_cloned
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                )
                .unwrap_or_else(|| {
                    eprintln!("Failed to run rust-script: {:?}", &explicit);
                    std::process::exit(1); // Exit with an error code
                });

                // // Lock global to store the child process
                // {
                //     let mut global = GLOBAL_CHILD.lock().unwrap();
                //     *global = Some(child);
                // }

                // // Wait for the child process to complete
                // let status = {
                //     let mut global = GLOBAL_CHILD.lock().unwrap();
                //     if let Some(mut child) = global.take() {
                child.wait()
                //     } else {
                //         // Handle missing child process
                //         eprintln!("Child process missing");
                //         std::process::exit(1); // Exit with an error code
                //     }
                // };

                // // Handle the child process exit status
                // match status {
                //     Ok(status) => {
                //         eprintln!("Child process exited with status code: {:?}", status.code());
                //         std::process::exit(status.code().unwrap_or(1)); // Exit with the child's status code
                //     }
                //     Err(err) => {
                //         eprintln!("Error waiting for child process: {}", err);
                //         std::process::exit(1); // Exit with an error code
                //     }
                // }
            });

            // Wait for the thread to complete, but with a timeout
            // let timeout = Duration::from_secs(10);
            match handle.join() {
                Ok(_) => {
                    println!("Child process finished successfully.");
                }
                Err(_) => {
                    eprintln!("Child process took too long to finish. Exiting...");
                    std::process::exit(1); // Exit if the process takes too long
                }
            }
        }
    }
}
/// Given any path, produce a relative path string starting with `./` (or `.\` on Windows).
fn make_relative(path: &Path) -> std::io::Result<String> {
    let cwd = env::current_dir()?;
    // Try to strip the cwd prefix; if it isn’t under cwd, just use the original path.
    let rel: PathBuf = match path.strip_prefix(&cwd) {
        Ok(stripped) => stripped.to_path_buf(),
        Err(_) => path.to_path_buf(),
    };

    let mut rel = if rel.components().count() == 0 {
        // special case: the same directory
        PathBuf::from(".")
    } else {
        rel
    };

    // Prepend "./" (or ".\") if it doesn’t already start with "." or ".."
    let first = rel.components().next().unwrap();
    match first {
        std::path::Component::CurDir | std::path::Component::ParentDir => {}
        _ => {
            rel = PathBuf::from(".").join(rel);
        }
    }

    // Convert back to a string with the correct separator
    let s = rel
        .to_str()
        .expect("Relative path should be valid UTF-8")
        .to_string();

    Ok(s)
}

// trait JoinTimeout {
//     fn join_timeout(self, timeout: Duration) -> Result<(), ()>;
// }

// impl<T> JoinTimeout for thread::JoinHandle<T> {
//     fn join_timeout(self, timeout: Duration) -> Result<(), ()> {
//         println!("Waiting for thread to finish...{}", timeout.as_secs());
//         let _ = thread::sleep(timeout);
//         match self.join() {
//             Ok(_) => Ok(()),
//             Err(_) => Err(()),
//         }
//     }
// }
