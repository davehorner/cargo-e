use crate::e_cli::RunAll;
use crate::e_command_builder::CargoCommandBuilder;
use crate::e_processmanager::ProcessManager;
use crate::e_target::{CargoTarget, TargetKind};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use std::process::Child;

#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::Pid;

// #[cfg(target_os = "windows")]
// use std::os::windows::process::CommandExt;

// #[cfg(target_os = "windows")]
// const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

// #[cfg(target_os = "windows")]
// fn send_ctrl_c(child: &mut Child) -> Result<()> {
//     println!("Sending CTRL-C to child process...");
//     use windows::Win32::System::Console::{GenerateConsoleCtrlEvent, CTRL_C_EVENT};

//     // Send CTRL+C to the child process group.
//     // The child must have been spawned with CREATE_NEW_PROCESS_GROUP.
//     let result = unsafe { GenerateConsoleCtrlEvent(CTRL_C_EVENT, child.id()) };
//     if result.is_err() {
//         return Err(anyhow::anyhow!("Failed to send CTRL_C_EVENT on Windows"));
//     }

//     // Allow some time for the child to handle the signal gracefully.
//     std::thread::sleep(std::time::Duration::from_millis(1000));

//     Ok(())
// }

#[cfg(not(target_os = "windows"))]
pub fn send_ctrl_c(child: &mut  std::process::Child) -> Result<()> {
    // On Unix, send SIGINT to the child.
    kill(Pid::from_raw(child.id() as i32), Signal::SIGINT).context("Failed to send SIGINT")?;
    // Wait briefly to allow graceful shutdown.
    std::thread::sleep(Duration::from_millis(2000));
    Ok(())
}

/// Runs all filtered targets with prebuild, child process management, and timeout‐based termination.
///
/// If the CLI flag `pre_build` is enabled, this function first prebuilds all targets by invoking
/// `cargo build` with the appropriate flags (using `--example` or `--bin` and, for extended targets,
/// the `--manifest-path` flag). Then it spawns a child process for each target using `cargo run`,
/// waits for the duration specified by `cli.wait`, kills the child process, and then checks its output.
///
/// # Parameters
///
/// - `cli`: A reference to the CLI configuration (containing flags like `pre_build`, `wait`, and extra arguments).
/// - `filtered_targets`: A slice of `Example` instances representing the targets to run.
///
/// # Errors
///
/// Returns an error if the prebuild step fails or if any child process fails to spawn or complete.
pub fn run_all_examples(
    manager: Arc<ProcessManager>,
    cli: &crate::Cli,
    filtered_targets: &[CargoTarget],
) -> Result<bool> {
    // let _ = crate::e_runner::register_ctrlc_handler();
    // Adjust RUSTFLAGS if --quiet was provided.
    set_rustflags_if_quiet(cli.quiet);

    // Prebuild targets if requested.
    if cli.pre_build {
        crate::e_prebuild::prebuild_examples(filtered_targets)
            .context("Prebuild of targets failed")?;
    }

    let mut targets = filtered_targets.to_vec();
    targets.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    // let manager = ProcessManager::new(&cli);

    let user_requested_quit = false;
    //for target in targets {
    for (idx, target) in targets.iter().enumerate() {
        println!("\nRunning target: {}", target.name);

        let current_bin = env!("CARGO_PKG_NAME");
        // Skip running our own binary.
        if target.kind == TargetKind::Binary && target.name == current_bin {
            continue;
        }

        // Build the command using CargoCommandBuilder.
        let manifest_path = PathBuf::from(target.manifest_path.clone());
        let builder = CargoCommandBuilder::new(&manifest_path, &cli.subcommand, cli.filter)
            .with_target(&target)
            .with_required_features(&target.manifest_path, &target)
            .with_cli(cli)
            .with_extra_args(&cli.extra);

        // For debugging, print out the full command.
        let cmd_debug = format!(
            "{} {}",
            builder.alternate_cmd.as_deref().unwrap_or("cargo"),
            builder.args.join(" ")
        );
        // PROMPT let key = crate::e_prompts::prompt(&format!("Full command: {}", cmd_debug), 2)?;
        // if let Some('q') = key {
        //     user_requested_quit = true;
        //     println!("User requested quit.");
        //     break;
        // }

        // Build the std::process::Command.
        // let mut command = builder.build_command();
        // #[cfg(target_os = "windows")]
        // {
        //     command.creation_flags(CREATE_NEW_PROCESS_GROUP);
        // }

        // Before spawning, check for workspace manifest errors and patch if necessary.
        let maybe_backup = crate::e_manifest::maybe_patch_manifest_for_run(&target.manifest_path)
            .context("Failed to patch manifest for run")?;

        //    let pid=    Arc::new(builder).run()?;

        //        let pid = Arc::new(builder).run(|pid, handle| {
        //     manager.register(handle);
        let mut system = System::new_all();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        // Refresh CPU usage to get actual value.
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu(),
        );
        // })?;
        let pid = Arc::new(builder).run({
            let manager_ref = Arc::clone(&manager);
            move |_pid, handle| {
                manager_ref.register(handle);
            }
        })?;

        // Spawn the child process.
        // let mut child = command
        //     .spawn()
        //     .with_context(|| format!("Failed to spawn cargo run for target {}", target.name))?;
        // {
        //     let mut global = GLOBAL_CHILD.lock().unwrap();
        //     *global = Some(child);
        // }

        // Let the target run for the specified duration.
        let run_duration = Duration::from_secs(cli.wait);
        // thread::sleep(run_duration);
        // PROMPT let key = crate::e_prompts::prompt("waiting", run_duration.as_secs())?;
        // if let Some('q') = key {
        //     user_requested_quit = true;
        //     println!("User requested quit.");
        //     break;
        // }

        let _output = {
            // let mut global = crate::e_runner::GLOBAL_CHILD.lock().unwrap();
            // // Take ownership of the child.
            // let mut child = global
            //     .take()
            //     .ok_or_else(|| anyhow::anyhow!("No child process found"))?;

            // Set timeout based on the run_all mode.
            let timeout = match cli.run_all {
                RunAll::Timeout(secs) => Duration::from_secs(secs),
                RunAll::Forever => Duration::from_secs(u64::MAX), // effectively no timeout
                RunAll::NotSpecified => Duration::from_secs(cli.wait),
            };

            let mut start = None; //Instant::now();
            let runtime_start = manager
                .get(pid)
                .unwrap()
                .lock()
                .unwrap()
                .stats
                .lock()
                .unwrap()
                .build_finished_time;
            //println!("Runtime start time: {:?}", runtime_start);
            loop {
                //println!("Checking process status for PID: {}", pid);
                match manager.try_wait(pid) {
                    Ok(Some(status)) => {
                        // Process finished naturally.
                        println!("Process finished naturally.{:?}", status);
                        break;
                    }
                    _ => {
                        // Process is still running.
                        //println!("Process is still running.");
                    }
                }
                if manager.has_signalled() > 0 {
                    println!(
                        "Detected Ctrl+C. Exiting run_all loop.{}",
                        manager.has_signalled()
                    );
                    return Ok(false);
                }
                // Here, use your non-blocking prompt function if available.
                // For illustration, assume prompt_nonblocking returns Ok(Some(key)) if a key was pressed.
                // PROMPT if let Ok(Some(key)) = prompt("waiting press q to quit", 0) {
                //     // Wait on the child process.
                //     if key == 'q' {
                //         println!("User requested stop {}. pid {}", target.name, pid);
                //         manager.kill_by_pid(pid).ok();
                //         // let mut global = GLOBAL_CHILDREN.lock().unwrap();
                //         // if let Some(cargo_process_handle) = global.remove(&pid) {
                //         //     let mut cargo_process_handle = cargo_process_handle.lock().unwrap();
                //         //     send_ctrl_c(&mut cargo_process_handle.child)?;
                //         //     let _ = cargo_process_handle.kill(); // Attempt to kill the process
                //         //     // Ignore errors if the process has already terminated.
                //         //     // cargo_process_handle.wait_with_output().ok();
                //         // }
                //         break;
                //     }
                // }

                // Check if the child process has already finished.
                // if let Some(_status) = child.try_wait()? {
                //     // Process finished naturally.
                //     break child.wait_with_output().context(format!(
                //         "Failed to get process output for target {}",
                //         target.name
                //     ))?;
                // }
                // let process_handle = manager.get(pid).unwrap();
                // let handle = process_handle.lock().unwrap();
                // let stats = handle.stats.lock().unwrap().clone();
                // // let runtime_start = manager.get(pid).unwrap().lock().unwrap().stats.lock().unwrap().build_finished_time;
                // let runtime_start = stats.build_finished_time;
                let (_stats, runtime_start, end_time, status_display) = {
                    // Acquire the process handle from the manager.
                    let process_handle = manager.get(pid).unwrap();
                    // Lock the process handle to gain mutable or safe read access.
                    let handle = process_handle.lock().unwrap();

                    // Lock the stats and clone them.
                    let stats = handle.stats.lock().unwrap().clone();
                    // Extract the build_finished_time from the cloned stats.
                    let runtime_start = stats.build_finished_time;
                    let end_time = handle.result.end_time;
                    drop(handle);
                    let status_display = ProcessManager::format_process_status(
                        pid,
                        &process_handle,
                        &system,
                        &target,
                        (idx + 1, targets.len()),
                    );
                    // Return both the stats and runtime_start.
                    (stats, runtime_start, end_time, status_display)
                };
                // Refresh CPU usage to get actual value.
                system.refresh_processes_specifics(
                    ProcessesToUpdate::All,
                    true,
                    ProcessRefreshKind::nothing().with_cpu(),
                );

                ProcessManager::update_status_line(&status_display, true).ok();
                // println!("start time: {:?} endtime {:?}", runtime_start, end_time);
                if runtime_start.is_some() {
                    if start.is_none() {
                        start = Some(Instant::now());
                    }
                    // Check if the timeout has elapsed.
                    if start.expect("start should have set").elapsed() >= timeout {
                        println!(
                            "\nTimeout reached for target {}. Killing child process {}.",
                            target.name, pid
                        );
                        manager.kill_by_pid(pid).ok();
                        // let mut global = GLOBAL_CHILDREN.lock().unwrap();
                        // if let Some(cargo_process_handle) = global.remove(&pid) {
                        //     let mut cargo_process_handle = cargo_process_handle.lock().unwrap();
                        //     send_ctrl_c(&mut cargo_process_handle.child)?;
                        //     let _ = cargo_process_handle.kill(); // Attempt to kill the process
                        //     // Ignore errors if the process has already terminated.
                        //     // cargo_process_handle.wait_with_output().ok();
                        // }
                        break;
                        // send_ctrl_c(&mut child)?;
                        // child.kill().ok();
                        // break child.wait_with_output().context(format!(
                        //     "Failed to wait on killed process for target {}",
                        //     target.name
                        // ))?;
                    }

                    // Sleep briefly before polling again.
                    std::thread::sleep(Duration::from_millis(500));
                } else if end_time.is_some() {
                    println!("Process finished naturally.");
                    break;
                    // } else {
                    //     // Process is still running.
                    //     println!("Process is still running.");
                }

                std::thread::sleep(Duration::from_millis(100));
            }
        };

        // let output = {
        //     let mut global = GLOBAL_CHILD.lock().unwrap();
        //     if let Some(mut child) = global.take() {
        //         child.wait_with_output().with_context(|| {
        //             format!("Failed to wait on cargo run for target {}", target.name)
        //         })?
        //     } else {
        //         return Err(anyhow::anyhow!("Child process missing"));
        //     }
        // };

        // println!("{:?}",output);
        // if !output.stderr.is_empty() {
        //     eprintln!(
        //         "Target '{}' produced errors:\n{}",
        //         target.name,
        //         String::from_utf8_lossy(&output.stderr)
        //     );
        // }

        // Restore the manifest if it was patched.
        if let Some(original) = maybe_backup {
            fs::write(&target.manifest_path, original)
                .context("Failed to restore patched manifest")?;
        }

        // Check if the user requested to quit.
        if user_requested_quit {
            break;
        }

        // If using a timeout/run_all mechanism, sleep or prompt as needed.
        // For simplicity, we wait for a fixed duration here.
        let run_duration = Duration::from_secs(cli.wait);
        // PROMPT let _ = crate::e_prompts::prompt("waiting", run_duration.as_secs())?;
    }

    Ok(user_requested_quit)
}

// pub fn run_all_examples(cli: &Cli, filtered_targets: &[CargoTarget]) -> Result<()> {
//     // If --quiet was provided, adjust RUSTFLAGS.
//     set_rustflags_if_quiet(cli.quiet);

//     // Factor out the prebuild logic.
//     if cli.pre_build {
//         crate::e_prebuild::prebuild_examples(filtered_targets)
//             .context("Prebuild of targets failed")?;
//     }
//     let mut targets = filtered_targets.to_vec();
//     targets.sort_by(|a, b| a.display_name.cmp(&b.display_name));
//     // For each filtered target, run it with child process management.
//     for target in targets {
//         // Clear the screen before running each target.

//         // use crossterm::{execute, terminal::{Clear, ClearType}};
//         // use std::io::{stdout, Write};
//         //         execute!(stdout(), Clear(ClearType::All), crossterm::cursor::MoveTo(0, 0))?;
//         // std::io::Write::flush(&mut std::io::stdout()).unwrap();
//         println!("Running target: {}", target.name);

//         // Retrieve the current package name (or binary name) at compile time.
//         let current_bin = env!("CARGO_PKG_NAME");
//         // Avoid running our own binary if the target's name is the same.
//         if target.kind == TargetKind::Binary && target.name == current_bin {
//             continue;
//         }

//         // Determine the run flag and whether we need to pass the manifest path.
//         let (run_flag, needs_manifest) = match target.kind {
//             TargetKind::Example => ("--example", false),
//             TargetKind::ExtendedExample => ("--example", true),
//             TargetKind::Binary => ("--bin", false),
//             TargetKind::ExtendedBinary => ("--bin", true),
//             TargetKind::ManifestTauri => ("", true),
//             TargetKind::ManifestTauriExample => ("", true),
//             TargetKind::Test => ("--test", true),
//             TargetKind::Manifest => ("", true),
//             TargetKind::ManifestDioxus => ("", true),
//             TargetKind::ManifestDioxusExample => ("", true),
//             TargetKind::Bench => ("", true),
//         };
//         let mut cmd_parts = vec!["cargo".to_string()];
//         cmd_parts.push("run".to_string());
//         if cli.release {
//             cmd_parts.push("--release".to_string());
//         }
//         // Pass --quiet if requested.
//         if cli.quiet {
//             cmd_parts.push("--quiet".to_string());
//         }
//         cmd_parts.push(run_flag.to_string());
//         cmd_parts.push(target.name.clone());
//         if needs_manifest {
//             cmd_parts.push("--manifest-path".to_string());
//             cmd_parts.push(
//                 target
//                     .manifest_path
//                     .clone()
//                     .to_str()
//                     .unwrap_or_default()
//                     .to_owned(),
//             );
//         }
//         cmd_parts.extend(cli.extra.clone());

//         // // Build a vector of command parts for logging.
//         // let mut cmd_parts = vec!["cargo".to_string(), "run".to_string(), run_flag.to_string(), target.name.clone()];
//         // if needs_manifest {
//         //     cmd_parts.push("--manifest-path".to_string());
//         //     cmd_parts.push(target.manifest_path.clone());
//         // }
//         // // Append any extra CLI arguments.
//         // cmd_parts.extend(cli.extra.clone());

//         // Print out the full command that will be run.
//         let key = prompt(&format!("Full command: {}", cmd_parts.join(" ")), 2)?;
//         if let Some('q') = key {
//             println!("User requested quit.");
//             break;
//         }

//         // Clear the screen before running each target.
//         //println!("\x1B[2J\x1B[H");

//         // Build the command for execution.
//         let mut command = Command::new("cargo");
//         command.arg("run");
//         if cli.release {
//             command.arg("--release");
//         }
//         if cli.quiet {
//             command.arg("--quiet");
//         }
//         command.arg(run_flag).arg(&target.name);
//         if needs_manifest {
//             command.args(&[
//                 "--manifest-path",
//                 &target.manifest_path.to_str().unwrap_or_default().to_owned(),
//             ]);
//         }

//         // --- Inject required-features support using our helper ---
//         if let Some(features) = crate::e_manifest::get_required_features_from_manifest(
//             std::path::Path::new(&target.manifest_path),
//             &target.kind,
//             &target.name,
//         ) {
//             command.args(&["--features", &features]);
//         }
//         // --- End required-features support ---

//         // Append any extra CLI arguments.
//         command.args(&cli.extra);

//         // Spawn the child process.
//         let child = command
//             .spawn()
//             .with_context(|| format!("Failed to spawn cargo run for target {}", target.name))?;
//         {
//             let mut global = crate::e_runner::GLOBAL_CHILD.lock().unwrap();
//             *global = Some(child);
//         }
//         // Let the target run for the specified duration.
//         let run_duration = Duration::from_secs(cli.wait);
//         thread::sleep(run_duration);

//         // Kill the process (ignoring errors if it already terminated).

//         // Decide on the run duration per target and use it accordingly:
//         // Determine behavior based on the run_all flag:
//         let output = {
//             let mut global = crate::e_runner::GLOBAL_CHILD.lock().unwrap();
//             if let Some(mut child) = global.take() {
//                 match cli.run_all {
//                     RunAll::Timeout(timeout_secs) => {
//                         let message = format!(
//                             "Press any key to continue (timeout in {} seconds)...",
//                             timeout_secs
//                         );
//                         let key = prompt(&message, timeout_secs)?;
//                         if let Some('q') = key {
//                             println!("User requested quit.");
//                             // Terminate the process and break out of the loop.
//                             child.kill().ok();
//                             break;
//                         }
//                         child.kill().ok();
//                         child.wait_with_output().with_context(|| {
//                             format!("Failed to wait on cargo run for target {}", target.name)
//                         })?
//                     }
//                     RunAll::Forever => {
//                         let key = prompt(&"", 0)?;
//                         if let Some('q') = key {
//                             println!("User requested quit.");
//                             // Terminate the process and break out of the loop.
//                             child.kill().ok();
//                             break;
//                         } // Run until natural termination.
//                         child.wait_with_output().with_context(|| {
//                             format!("Failed to wait on cargo run for target {}", target.name)
//                         })?
//                     }
//                     RunAll::NotSpecified => {
//                         let key = prompt(&"", cli.wait)?;
//                         if let Some('q') = key {
//                             println!("User requested quit.");
//                             // Terminate the process and break out of the loop.
//                             child.kill().ok();
//                             break;
//                         }
//                         child.kill().ok();
//                         child.wait_with_output().with_context(|| {
//                             format!("Failed to wait on cargo run for target {}", target.name)
//                         })?
//                     }
//                 }
//             } else {
//                 return Err(anyhow::anyhow!("No child process found"));
//             }
//         };

//         if !output.stderr.is_empty() {
//             eprintln!(
//                 "Target '{}' produced errors:\n{}",
//                 target.name,
//                 String::from_utf8_lossy(&output.stderr)
//             );
//         }
//     }
//     Ok(())
// }

use std::{env, fs};

/// If quiet mode is enabled, ensure that RUSTFLAGS contains "-Awarnings".
/// If RUSTFLAGS is already set, and it does not contain "-Awarnings", then append it.
pub fn set_rustflags_if_quiet(quiet: bool) {
    if quiet {
        let current_flags = env::var("RUSTFLAGS").unwrap_or_else(|_| "".to_string());
        if !current_flags.contains("-Awarnings") {
            let new_flags = if current_flags.trim().is_empty() {
                "-Awarnings".to_string()
            } else {
                format!("{} -Awarnings", current_flags)
            };
            env::set_var("RUSTFLAGS", new_flags);
        }
    }
}
