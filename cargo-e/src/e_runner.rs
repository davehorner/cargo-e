use crate::e_processmanager::ProcessManager;
use crate::{e_target::TargetOrigin, prelude::*};
// #[cfg(not(feature = "equivalent"))]
// use ctrlc;
use crate::e_cargocommand_ext::CargoProcessHandle;
use crate::e_target::CargoTarget;
#[cfg(feature = "uses_plugins")]
use crate::plugins::plugin_api::Target as PluginTarget;
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
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

fn library_hint(lib: &str) -> &str {
    match lib {
        "javascriptcoregtk-4.1" => "libjavascriptcoregtk-4.1-dev",
        "libsoup-3.0" => "libsoup-3.0-dev",
        "webkit2gtk-4.1" => "libwebkit2gtk-4.1-dev",
        "openssl" => "libssl-dev",
        _ => lib, // Fallback, assume same name
    }
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

    // If this is a plugin-provided target, execute it via the plugin's in-process run
    #[cfg(feature = "uses_plugins")]
    if target.kind == crate::e_target::TargetKind::Plugin {
        if let Some(crate::e_target::TargetOrigin::Plugin { plugin_path, .. }) = &target.origin {
            // Current working directory
            let cwd = std::env::current_dir()?;
            // Load the plugin directly based on its file extension
            let ext = plugin_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let plugin: Box<dyn crate::plugins::plugin_api::Plugin> = match ext {
                "lua" => {
                    #[cfg(feature = "uses_lua")]
                    {
                        Box::new(crate::plugins::lua_plugin::LuaPlugin::load(
                            plugin_path,
                            cli,
                            manager.clone(),
                        )?)
                    }
                    #[cfg(not(feature = "uses_lua"))]
                    {
                        return Err(anyhow::anyhow!("Lua plugin support is not enabled"));
                    }
                }
                "rhai" => {
                    #[cfg(feature = "uses_rhai")]
                    {
                        Box::new(crate::plugins::rhai_plugin::RhaiPlugin::load(
                            plugin_path,
                            cli,
                            manager.clone(),
                        )?)
                    }
                    #[cfg(not(feature = "uses_rhai"))]
                    {
                        return Err(anyhow::anyhow!("Rhai plugin support is not enabled"));
                    }
                }
                "wasm" => {
                    #[cfg(feature = "uses_wasm")]
                    {
                        if let Some(wp) =
                            crate::plugins::wasm_plugin::WasmPlugin::load(plugin_path)?
                        {
                            Box::new(wp)
                        } else {
                            // Fallback to generic export plugin
                            Box::new(
                                crate::plugins::wasm_export_plugin::WasmExportPlugin::load(
                                    plugin_path,
                                )?
                                .expect("Failed to load export plugin"),
                            )
                        }
                    }
                    #[cfg(not(feature = "uses_wasm"))]
                    {
                        return Err(anyhow::anyhow!("WASM plugin support is not enabled"));
                    }
                }
                "dll" => {
                    #[cfg(feature = "uses_wasm")]
                    {
                        Box::new(
                            crate::plugins::wasm_export_plugin::WasmExportPlugin::load(
                                plugin_path,
                            )?
                            .expect("Failed to load export plugin"),
                        )
                    }
                    #[cfg(not(feature = "uses_wasm"))]
                    {
                        return Err(anyhow::anyhow!("WASM export plugin support is not enabled"));
                    }
                }
                other => {
                    return Err(anyhow::anyhow!("Unknown plugin extension: {}", other));
                }
            };
            // Run the plugin and capture output
            let plugin_target = PluginTarget::from(target.clone());
            let output = plugin.run(&cwd, &plugin_target)?;
            // Print exit code and subsequent output lines
            if !output.is_empty() {
                if let Ok(code) = output[0].parse::<i32>() {
                    eprintln!("Plugin exited with code: {}", code);
                }
                for line in &output[1..] {
                    println!("{}", line);
                }
            }
            return Ok(None);
        }
    }
    // Not a plugin, continue with standard cargo invocation
    let manifest_path = PathBuf::from(target.manifest_path.clone());
    // Build the command using the CargoCommandBuilder.
    let mut builder = crate::e_command_builder::CargoCommandBuilder::new(
        &target.name,
        &manifest_path,
        &cli.subcommand,
        cli.filter,
        cli.cached,
        cli.default_binary_is_runner,
    )
    .with_target(target)
    .with_required_features(&target.manifest_path, target)
    .with_cli(cli);

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
    let a_blder = Arc::new(builder.clone());
    let pid = a_blder.run(|_pid, handle| {
        manager.register(handle);
    })?;
    let result = manager.wait(pid, None)?;
    // println!("HERE IS THE RESULT!{} {:?}",pid,manager.get(pid));
    // println!("\n\nHERE IS THE RESULT!{} {:?}",pid,result);
    if result
        .exit_status
        .map_or(false, |status| status.code() == Some(101))
    {
        println!(
            "ProcessManager senses pid {} cargo error, running again to capture and analyze",
            pid
        );
        match builder.clone().capture_output() {
            Ok(output) => {
                let system_lib_regex = Regex::new(
                    r"\s*The system library `([^`]+)` required by crate `([^`]+)` was not found\.",
                )
                .unwrap();

                if let Some(captures) = system_lib_regex.captures(&output) {
                    let library = &captures[1];
                    let crate_name = &captures[2];
                    println!(
                        "cargo-e detected missing system library '{}' required by crate '{}'.",
                        library, crate_name
                    );

                    // Suggest installation based on common package managers
                    println!(
                        "You might need to install '{}' via your system package manager.",
                        library
                    );
                    println!("For example:");

                    println!(
                        "  • Debian/Ubuntu: sudo apt install {}",
                        library_hint(library)
                    );
                    println!("  • Fedora: sudo dnf install {}", library_hint(library));
                    println!("  • Arch: sudo pacman -S {}", library_hint(library));
                    println!(
                        "  • macOS (Homebrew): brew install {}",
                        library_hint(library)
                    );
                    std::process::exit(0);
                } else if output.contains("error: failed to load manifest for workspace member") {
                    println!("cargo-e error: failed to load manifest for workspace member, please check your workspace configuration.");
                    println!("cargo-e autorecovery: removing manfifest path from argument and changing to parent of Cargo.toml.");
                    let cwd = target
                        .manifest_path
                        .parent()
                        .unwrap_or_else(|| Path::new("."));
                    // Rebuild the command with the new cwd
                    builder.execution_dir = Some(cwd.to_path_buf());
                    // Remove --manifest-path and its associated value from the args array
                    if let Some(pos) = builder.args.iter().position(|arg| arg == "--manifest-path")
                    {
                        // Remove --manifest-path and the next argument (the manifest path value)
                        builder.args.remove(pos); // Remove --manifest-path
                        if pos < builder.args.len() {
                            builder.args.remove(pos); // Remove the manifest path value
                        }
                    }
                    let mut cmd = builder.clone().build_command();
                    cmd.current_dir(cwd);

                    // Retry the command execution
                    let mut child = cmd.spawn()?;
                    let status = child.wait()?;
                    return Ok(Some(status)); // Return the exit status after retrying
                                             // return run_example(manager, cli, target);  // Recursively call run_example
                }
                if output.contains("no such command: `tauri`") {
                    println!("cargo tauri is not installed, please install it with cargo install tauri-cli");
                    // Use the yesno function to prompt the user
                    match crate::e_prompts::yesno(
                        "Do you want to install tauri-cli?",
                        Some(true), // Default to yes
                    ) {
                        Ok(Some(true)) => {
                            println!("Installing tauri-cli...");
                            match spawn_cargo_process(&["install", "tauri-cli"]) {
                                Ok(mut child) => {
                                    child.wait().ok(); // Wait for the installation to finish
                                } // Installation successful
                                Err(e) => {
                                    eprintln!("Error installing tauri-cli: {}", e);
                                }
                            }
                        }
                        Ok(Some(false)) => {}
                        Ok(None) => {
                            println!("Installation cancelled (timeout or invalid input).");
                        }
                        Err(e) => {
                            eprintln!("Error during prompt: {}", e);
                        }
                    }
                } else if output.contains("error: no such command: `leptos`") {
                    println!("cargo-leptos is not installed, please install it with cargo install cargo-leptos");
                    // Use the yesno function to prompt the user
                    match crate::e_prompts::yesno(
                        "Do you want to install cargo-leptos?",
                        Some(true), // Default to yes
                    ) {
                        Ok(Some(true)) => {
                            println!("Installing cargo-leptos...");
                            match spawn_cargo_process(&["install", "cargo-leptos"]) {
                                Ok(mut child) => {
                                    child.wait().ok(); // Wait for the installation to finish
                                } // Installation successful
                                Err(e) => {
                                    eprintln!("Error installing cargo-leptos: {}", e);
                                }
                            }
                        }
                        Ok(Some(false)) => {}
                        Ok(None) => {
                            println!("Installation cancelled (timeout or invalid input).");
                        }
                        Err(e) => {
                            eprintln!("Error during prompt: {}", e);
                        }
                    }
                    // needed for cargo-leptos but as part of tool installer
                    //   } else if output.contains("Command 'perl' not found. Is perl installed?") {
                    //     println!("cargo e sees a perl issue; maybe a prompt in the future or auto-resolution.");
                    //     crate::e_autosense::auto_sense_perl();
                  }  else if output.contains("Unable to find libclang")
      || output.contains("couldn't find any valid shared libraries matching: ['clang.dll', 'libclang.dll']") 
{
    crate::e_autosense::auto_sense_llvm();

                } else if output.contains("no such command: `dx`") {
                    println!("cargo dx is not installed, please install it with cargo install dioxus-cli");
                } else if output.contains("no such command: `scriptisto`") {
                    println!("cargo scriptisto is not installed, please install it with cargo install scriptisto");
                } else if output.contains("no such command: `rust-script`") {
                    println!("cargo rust-script is not installed, please install it with cargo install rust-script");
                } else if output.contains(
                    "No platform feature enabled. Please enable one of the following features:",
                ) {
                    println!("cargo e sees a dioxus issue; maybe a prompt in the future or auto-resolution.");
                } else {
                    //println!("cargo error: {}", output);
                }
            }
            Err(e) => {
                eprintln!("Error running cargo: {}", e);
            }
        }
    }
    // let is_run_command = matches!(cli.subcommand.as_str(), "run" | "r");
    // if !is_run_command && result.is_filter || ( result.is_filter && !result.is_could_not_compile ) {
    result.print_exact();
    result.print_compact();
    result.print_short();
    manager.print_prefixed_summary();
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.level.eq("error"))
        .collect();
    let error_width = errors.len().to_string().len().max(1);
    let line: Vec<String> = errors
        .iter()
        .enumerate()
        .map(|(i, diag)| {
            let index = format!("{:0width$}", i + 1, width = error_width);
            let lineref = if diag.lineref.is_empty() {
                ""
            } else {
                &diag.lineref
            };
            // Resolve filename to full path
            let (filename, goto) = if let Some((file, line, col)) = diag
                .lineref
                .split_once(':')
                .and_then(|(f, rest)| rest.split_once(':').and_then(|(l, c)| Some((f, l, c))))
            {
                let full_path = std::fs::canonicalize(file).unwrap_or_else(|_| {
                    let manifest_dir = std::path::Path::new(&manifest_path)
                        .parent()
                        .unwrap_or_else(|| {
                            eprintln!(
                                "Failed to determine parent directory for manifest: {:?}",
                                manifest_path
                            );
                            std::path::Path::new(".")
                        });
                    let fallback_path = manifest_dir.join(file);
                    std::fs::canonicalize(&fallback_path).unwrap_or_else(|_| {
                        let parent_fallback_path = manifest_dir.join("../").join(file);
                        std::fs::canonicalize(&parent_fallback_path).unwrap_or_else(|_| {
                            eprintln!("Failed to resolve full path for: {} using ../", file);
                            file.into()
                        })
                    })
                });
                let stripped_file = full_path.to_string_lossy().replace("\\\\?\\", "");

                (stripped_file.to_string(), format!("{}:{}", line, col))
            } else {
                ("".to_string(), "".to_string())
            };
            let code_path = which("code").unwrap_or_else(|_| "code".to_string().into());
            format!(
                "{}: {}\nanchor:{}: {}\\n {}|\"{}\" --goto \"{}:{}\"\n",
                index,
                diag.message.trim(),
                index,
                diag.message.trim(),
                lineref,
                code_path.display(),
                filename,
                goto,
            )
        })
        .collect();
    if !errors.is_empty() {
        if let Ok(e_window_path) = which("e_window") {
            // Compose a nice message for e_window's stdin
            let stats = result.stats;
            // Compose a table with cargo-e and its version, plus panic info
            let cargo_e_version = env!("CARGO_PKG_VERSION");
            let card = format!(
                "--title \"failed build: {target}\" --width 400 --height 300 --decode-debug\n\
                target | {target} | string\n\
                cargo-e | {version} | string\n\
                \n\
                failed build: {target}\n{errors} errors.\n\n{additional_errors}",
                target = stats.target_name,
                version = cargo_e_version,
                errors = errors.len(),
                additional_errors = line
                    .iter()
                    .map(|l| l.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            // Set the working directory to the manifest's parent directory
            let manifest_dir = std::path::Path::new(&manifest_path)
                .parent()
                .unwrap_or_else(|| {
                    eprintln!(
                        "Failed to determine parent directory for manifest: {:?}",
                        target.manifest_path
                    );
                    std::path::Path::new(".")
                });

            let child = std::process::Command::new(e_window_path)
                .current_dir(manifest_dir) // Set working directory
                .stdin(std::process::Stdio::piped())
                .spawn();
            if let Ok(mut child) = child {
                if let Some(stdin) = child.stdin.as_mut() {
                    use std::io::Write;
                    let _ = stdin.write_all(card.as_bytes());
                }
            }
        }
    }

    // }

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
    wait_for_tts_to_finish(15000);

    Ok(result.exit_status)
}

#[cfg(feature = "uses_tts")]
pub fn wait_for_tts_to_finish(max_wait_ms: u64) {
    let tts_mutex = crate::GLOBAL_TTS.get();
    if tts_mutex.is_none() {
        eprintln!("TTS is not initialized, skipping wait.");
        return;
    }
    let start = std::time::Instant::now();
    let mut tts_guard = None;
    for _ in 0..3 {
        if let Ok(guard) = tts_mutex.unwrap().lock() {
            tts_guard = Some(guard);
            break;
        } else {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        if start.elapsed().as_millis() as u64 >= max_wait_ms {
            eprintln!("Timeout while trying to lock TTS mutex.");
            return;
        }
    }
    if let Some(tts) = tts_guard {
        while tts.is_speaking().unwrap_or(false) {
            if start.elapsed().as_millis() as u64 >= max_wait_ms {
                eprintln!("Timeout while waiting for TTS to finish speaking.");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    } else {
        eprintln!("Failed to lock TTS mutex after 3 attempts, skipping wait.");
    }
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
    let rust_script = check_rust_script_installed();
    if rust_script.is_err() {
        return None;
    }
    let rust_script = rust_script.unwrap();
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
        if let Ok(true) = is_active_rust_script(explicit_path) {
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
        let extra_str_slice: Vec<String> = extra_args.to_vec();

        if let Ok(true) = is_active_scriptisto(explicit_path) {
            // Run the child process in a separate thread to allow Ctrl+C handling
            let handle = thread::spawn(move || {
                let extra_str_slice_cloned: Vec<String> = extra_str_slice.clone();
                let child = run_scriptisto(
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
                //         child.wait()
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
