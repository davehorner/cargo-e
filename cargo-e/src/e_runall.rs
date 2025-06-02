use crate::e_cli::RunAll;
use crate::e_command_builder::CargoCommandBuilder;
use crate::e_processmanager::ProcessManager;
use crate::e_target::{CargoTarget, TargetKind};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

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
pub fn send_ctrl_c(child: &mut std::process::Child) -> Result<()> {
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
    // Adjust RUSTFLAGS if --quiet was provided.
    set_rustflags_if_quiet(cli.quiet);

    // Prebuild targets if requested.
    if cli.pre_build {
        crate::e_prebuild::prebuild_examples(filtered_targets)
            .context("Prebuild of targets failed")?;
    }

    let mut targets = filtered_targets.to_vec();
    targets.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    let user_requested_quit = Arc::new(AtomicBool::new(false));

    let chunk_size = cli.run_at_a_time;
    let mut idx = 0;
    while idx < targets.len() {
        let chunk = &targets[idx..std::cmp::min(idx + chunk_size, targets.len())];
        let mut handles = vec![];

        for (chunk_idx, target) in chunk.iter().enumerate() {
            let manager = Arc::clone(&manager);
            let cli = cli.clone();
            let target = target.clone();
            let targets_len = targets.len();
            let idx = idx + chunk_idx;
            let user_requested_quit_thread = Arc::clone(&user_requested_quit);

            // Spawn a thread for each target in the chunk
            let handle = std::thread::spawn(move || {
                // --- Begin: original per-target logic ---
                let current_bin = env!("CARGO_PKG_NAME");
                // Skip running our own binary.
                if target.kind == TargetKind::Binary && target.name == current_bin {
                    return Ok(()) as Result<()>;
                }

                let manifest_path = PathBuf::from(target.manifest_path.clone());
                let builder = CargoCommandBuilder::new(
                    &target.name,
                    &manifest_path,
                    &cli.subcommand,
                    cli.filter,
                    cli.cached
                )
                .with_target(&target)
                .with_cli(&cli)
                .with_extra_args(&cli.extra);

                builder.print_command();

                let maybe_backup =
                    crate::e_manifest::maybe_patch_manifest_for_run(&target.manifest_path)
                        .context("Failed to patch manifest for run")?;

                // let system = Arc::new(Mutex::new(System::new_all()));
                // std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
                // let mut system_guard = system.lock().unwrap();
                // system_guard.refresh_processes_specifics(
                //     ProcessesToUpdate::All,
                //     true,
                //     ProcessRefreshKind::nothing().with_cpu(),
                // );
                // drop(system_guard);

                let pid = Arc::new(builder).run({
                    let manager_ref = Arc::clone(&manager);
                    let t = target.clone();
                    let len = targets_len;
                    // let system_clone = system.clone();
                    move |pid, handle| {
                        let stats = handle.stats.lock().unwrap().clone();
                        let runtime_start = if stats.is_comiler_target {
                            stats.build_finished_time
                        } else {
                            stats.start_time
                        };
                        if !cli.no_status_lines {
                            let status_display = ProcessManager::format_process_status(
                                pid,
                                runtime_start,
                                &t,
                                (idx + 1, len),
                            );
                            ProcessManager::update_status_line(&status_display, true).ok();
                        }
                        manager_ref.register(handle);
                    }
                })?;

                let timeout = match cli.run_all {
                    RunAll::Timeout(secs) => Duration::from_secs(secs),
                    RunAll::Forever => Duration::from_secs(u64::MAX),
                    RunAll::NotSpecified => Duration::from_secs(cli.wait),
                };

                let mut start = None;
                loop {
                    match manager.try_wait(pid) {
                        Ok(Some(_status)) => {
                            manager.remove(pid);
                            break;
                        }
                        _ => {
                            // Process is still running.
                            //println!("Process is still running.");
                        }
                    }
                    if manager.has_signalled() > 0 {
                        println!("Detected Ctrl+C. {}", manager.has_signalled());
                        manager.remove(pid); // Clean up the process handle

                        if manager.has_signalled() > 1 {
                            if let Some(dur) = manager.time_between_signals() {
                                if dur < Duration::from_millis(350) {
                                    println!("User requested quit two times quickly (<350ms).");
                                    user_requested_quit_thread.store(true, Ordering::SeqCst);
                                    break;
                                }
                            }
                        }
                        println!("Dectected Ctrl+C, coninuing to next target.");
                        manager.reset_signalled();
                        break;
                    }

                    let (_stats, runtime_start, end_time, status_display) = {
                        let process_handle = manager.get(pid).unwrap();
                        let handle = process_handle.lock().unwrap();
                        let stats = handle.stats.lock().unwrap().clone();
                        let runtime_start = if stats.is_comiler_target {
                            stats.build_finished_time
                        } else {
                            stats.start_time
                        };
                        let end_time = handle.result.end_time;
                        drop(handle);
                        let status_display = if !cli.no_status_lines {
                            ProcessManager::format_process_status(
                                pid,
                                runtime_start,
                                &target,
                                (idx + 1, targets_len),
                            )
                        } else {
                            String::new()
                        };
                        (stats, runtime_start, end_time, status_display)
                    };

                    if cli.filter && !cli.no_status_lines {
                        // let mut system_guard = system.lock().unwrap();
                        // system_guard.refresh_processes_specifics(
                        //     ProcessesToUpdate::All,
                        //     true,
                        //     ProcessRefreshKind::nothing().with_cpu(),
                        // );
                        // drop(system_guard);
                        ProcessManager::update_status_line(&status_display, true).ok();
                    }
                    if runtime_start.is_some() {
                        if start.is_none() {
                            start = Some(Instant::now());
                        }
                        if start.expect("start should have set").elapsed() >= timeout {
                            println!(
                                "\nTimeout reached for target {}. Killing child process {}.",
                                target.name, pid
                            );
                            manager.kill_by_pid(pid).ok();
                            manager.remove(pid);
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(500));
                    } else if end_time.is_some() {
                        println!("Process finished naturally.");
                        manager.remove(pid);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }

                if let Some(original) = maybe_backup {
                    fs::write(&target.manifest_path, original)
                        .context("Failed to restore patched manifest")?;
                }
                manager.generate_report(cli.gist);

                Ok(())
                // --- End: original per-target logic ---
            });
            if user_requested_quit.load(Ordering::SeqCst) {
                break;
            }
            handles.push(handle);
        }
        // Check if the user requested to quit.
        if user_requested_quit.load(Ordering::SeqCst) {
            break;
        }
        // Wait for all threads in this chunk to finish
        for handle in handles {
            let _ = handle.join();
        }

        idx += chunk_size;
    }

    Ok(Arc::clone(&user_requested_quit).load(Ordering::SeqCst))
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
