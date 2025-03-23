use crate::{e_cli::RunAll, e_prompts::prompt, Cli, Example, TargetKind};
use anyhow::{Context, Result};
use std::{process::Command, thread, time::Duration};

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
pub fn run_all_examples(cli: &Cli, filtered_targets: &[Example]) -> Result<()> {
    // If --quiet was provided, adjust RUSTFLAGS.
    set_rustflags_if_quiet(cli.quiet);

    // Factor out the prebuild logic.
    if cli.pre_build {
        crate::e_prebuild::prebuild_examples(filtered_targets)
            .context("Prebuild of targets failed")?;
    }
    let mut targets = filtered_targets.to_vec();
    targets.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    // For each filtered target, run it with child process management.
    for target in targets {
        // Clear the screen before running each target.

        // use crossterm::{execute, terminal::{Clear, ClearType}};
        // use std::io::{stdout, Write};
        //         execute!(stdout(), Clear(ClearType::All), crossterm::cursor::MoveTo(0, 0))?;
        // std::io::Write::flush(&mut std::io::stdout()).unwrap();
        println!("Running target: {}", target.name);

        // Retrieve the current package name (or binary name) at compile time.
        let current_bin = env!("CARGO_PKG_NAME");
        // Avoid running our own binary if the target's name is the same.
        if target.kind == crate::TargetKind::Binary && target.name == current_bin {
            continue;
        }

        // Determine the run flag and whether we need to pass the manifest path.
        let (run_flag, needs_manifest) = match target.kind {
            TargetKind::Example => ("--example", false),
            TargetKind::ExtendedExample => ("--example", true),
            TargetKind::Binary => ("--bin", false),
            TargetKind::ExtendedBinary => ("--bin", true),
        };
        let mut cmd_parts = vec!["cargo".to_string()];
        cmd_parts.push("run".to_string());
        if cli.release {
            cmd_parts.push("--release".to_string());
        }
        // Pass --quiet if requested.
        if cli.quiet {
            cmd_parts.push("--quiet".to_string());
        }
        cmd_parts.push(run_flag.to_string());
        cmd_parts.push(target.name.clone());
        if needs_manifest {
            cmd_parts.push("--manifest-path".to_string());
            cmd_parts.push(target.manifest_path.clone());
        }
        cmd_parts.extend(cli.extra.clone());

        // // Build a vector of command parts for logging.
        // let mut cmd_parts = vec!["cargo".to_string(), "run".to_string(), run_flag.to_string(), target.name.clone()];
        // if needs_manifest {
        //     cmd_parts.push("--manifest-path".to_string());
        //     cmd_parts.push(target.manifest_path.clone());
        // }
        // // Append any extra CLI arguments.
        // cmd_parts.extend(cli.extra.clone());

        // Print out the full command that will be run.
        let key = prompt(&format!("Full command: {}", cmd_parts.join(" ")), 2)?;
        if let Some('q') = key {
            println!("User requested quit.");
            break;
        }

        // Clear the screen before running each target.
        //println!("\x1B[2J\x1B[H");

        // Build the command for execution.
        let mut command = Command::new("cargo");
        command.arg("run");
        if cli.release {
            command.arg("--release");
        }
        if cli.quiet {
            command.arg("--quiet");
        }
        command.arg(run_flag).arg(&target.name);
        if needs_manifest {
            command.args(&["--manifest-path", &target.manifest_path]);
        }
        command.args(&cli.extra);

        // Spawn the child process.
        let child = command
            .spawn()
            .with_context(|| format!("Failed to spawn cargo run for target {}", target.name))?;
        {
            let mut global = crate::e_runner::GLOBAL_CHILD.lock().unwrap();
            *global = Some(child);
        }
        // Let the target run for the specified duration.
        let run_duration = Duration::from_secs(cli.wait);
        thread::sleep(run_duration);

        // Kill the process (ignoring errors if it already terminated).

        // Decide on the run duration per target and use it accordingly:
        // Determine behavior based on the run_all flag:
        let output = {
            let mut global = crate::e_runner::GLOBAL_CHILD.lock().unwrap();
            if let Some(mut child) = global.take() {
                match cli.run_all {
                    RunAll::Timeout(timeout_secs) => {
                        let message = format!(
                            "Press any key to continue (timeout in {} seconds)...",
                            timeout_secs
                        );
                        let key = prompt(&message, timeout_secs)?;
                        if let Some('q') = key {
                            println!("User requested quit.");
                            // Terminate the process and break out of the loop.
                            child.kill().ok();
                            break;
                        }
                        child.kill().ok();
                        child.wait_with_output().with_context(|| {
                            format!("Failed to wait on cargo run for target {}", target.name)
                        })?
                    }
                    RunAll::Forever => {
                        let key = prompt(&"", 0)?;
                        if let Some('q') = key {
                            println!("User requested quit.");
                            // Terminate the process and break out of the loop.
                            child.kill().ok();
                            break;
                        } // Run until natural termination.
                        child.wait_with_output().with_context(|| {
                            format!("Failed to wait on cargo run for target {}", target.name)
                        })?
                    }
                    RunAll::NotSpecified => {
                        let key = prompt(&"", cli.wait)?;
                        if let Some('q') = key {
                            println!("User requested quit.");
                            // Terminate the process and break out of the loop.
                            child.kill().ok();
                            break;
                        }
                        child.kill().ok();
                        child.wait_with_output().with_context(|| {
                            format!("Failed to wait on cargo run for target {}", target.name)
                        })?
                    }
                }
            } else {
                return Err(anyhow::anyhow!("No child process found"));
            }
        };

        if !output.stderr.is_empty() {
            eprintln!(
                "Target '{}' produced errors:\n{}",
                target.name,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

use std::env;

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
