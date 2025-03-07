// src/e_bacon.rs

use crate::e_types::Example;
use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// src/e_bacon.rs

// #[cfg(windows)]
// use std::os::windows::io::AsRawHandle;

/// Runs the "bacon" command on the given sample in detached mode,
/// capturing the output (stdout and stderr) into "output_bacon.txt".
/// It passes the project directory (derived from the sample’s manifest_path)
/// via the "--path" flag and appends any extra arguments.
///
/// On Windows, if the environment variable DEBUG_BACON is set, it uses `/K` and echoes
/// the folder parameter so that you can inspect it; otherwise it uses normal detached flags.
pub fn run_bacon(sample: &Example, extra_args: &[String]) -> Result<(), Box<dyn Error>> {
    // Disable raw mode for debug printing.
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;

    println!("Running bacon for sample: {}", sample.name);

    // Determine the project directory from the sample's manifest_path.
    let manifest_path = Path::new(&sample.manifest_path);
    let project_dir = manifest_path.parent().unwrap_or(manifest_path);

    let mut cmd = Command::new("cmd");
    let args = &[
        "/c",
        "START",
        &format!(""),
        "bacon",
        &format!("--path={}", project_dir.to_str().unwrap_or_default()),
    ];
    cmd.args(args);

    let output_file = std::fs::File::create("output_bacon.txt")?;

    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::HANDLE;
        // use windows::Win32::System::Console::{SetHandleInformation, HANDLE_FLAG_INHERIT};

        // Mark the file handle as inheritable.
        // let raw_handle: HANDLE = output_file.as_raw_handle().into();
        // unsafe {
        //     // SetHandleInformation returns a BOOL.
        //     if !SetHandleInformation(raw_handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT).as_bool() {
        //         eprintln!("Failed to set handle inheritance for file {}", project_dir.display());
        //     }
        // }
        // Continue with setting stdout and stderr...
        // cmd.stdout(Stdio::from(output_file.try_clone()?))
        //    .stderr(Stdio::from(output_file));

        //  const DETACHED_PROCESS: u32 = 0x00000008;
        //  const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        //  cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }

    #[cfg(not(windows))]
    {
        // On Unix-like systems, redirect standard I/O to our file.
        cmd.stdout(Stdio::from(output_file.try_clone()?))
            .stderr(Stdio::from(output_file));
    }

    // Spawn the bacon process detached. We do not wait on it.
    let child = cmd.spawn()?;
    std::mem::forget(child);
    println!("Bacon process detached; output is captured in 'output_bacon.txt'.");

    // crossterm::terminal::enable_raw_mode()?;
    // crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    Ok(())
}

// /// Runs the "bacon" command on the given sample in detached mode,
// /// capturing the output (stdout and stderr) into "output_bacon.txt".
// /// It passes the project directory (derived from the sample’s manifest_path)
// /// via the "--path" flag and appends any extra arguments.
// pub fn run_bacon(sample: &Example, extra_args: &[String]) -> Result<(), Box<dyn Error>> {
//     println!("Running bacon for sample: {}", sample.name);

//     // Determine the project directory from the sample's manifest path.
//     let manifest_path = Path::new(&sample.manifest_path);
//     let project_dir = manifest_path.parent().unwrap_or(manifest_path);

//     // Build the command.
//     let mut cmd = Command::new("bacon");
//     cmd.args(&["--path", project_dir.to_str().unwrap_or_default()]);
//     if !extra_args.is_empty() {
//         cmd.args(extra_args);
//     }

//     // Open an output file to capture the output.
//     // This file will be created (or overwritten) in the current working directory.
//     let output_file = std::fs::File::create("output_bacon.txt")?;

//     // Redirect stdout and stderr to the output file.
//     cmd.stdout(Stdio::from(output_file.try_clone()?))
//        .stderr(Stdio::from(output_file));

//     // Platform-specific process detachment.
//     #[cfg(windows)]
//     {
//         // DETACHED_PROCESS (0x00000008) and CREATE_NEW_PROCESS_GROUP (0x00000200)
//         // are required on Windows to detach the child process.
//         const DETACHED_PROCESS: u32 = 0x00000008;
//         const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
//         cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
//     }
//     #[cfg(not(windows))]
//     {
//         // On Unix-like systems, simply redirecting the standard I/O is enough to detach.
//         // (Optionally you could also use a daemonizing library or fork the process.)
//     }

//     // Spawn the bacon process detached. We do not wait on it.
//     let _child = cmd.spawn()?;
//     println!("Bacon process detached; output is captured in 'output_bacon.txt'.");
//     Ok(())
// }

// src/e_bacon.rs

/// Runs the "bacon" command on the given sample in detached mode.
/// It passes the project directory (derived from the sample’s manifest_path) via the "--path" flag,
/// and appends any extra arguments. The process is detached so that the caller does not wait for it.
// pub fn run_bacon(sample: &Example, extra_args: &[String]) -> Result<(), Box<dyn Error>> {
//     println!("Running bacon for sample: {}", sample.name);

//     // Determine the project directory from the sample's manifest_path.
//     let manifest_path = std::path::Path::new(&sample.manifest_path);
//     let project_dir = manifest_path.parent().unwrap_or(manifest_path);

//     let mut cmd = Command::new("bacon");
//     cmd.args(&["--path", project_dir.to_str().unwrap_or_default()]);
//     if !extra_args.is_empty() {
//         cmd.args(extra_args);
//     }

//     // Detach on Windows.
//     #[cfg(windows)]
//     {
//         const DETACHED_PROCESS: u32 = 0x00000008;
//         const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
//         cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
//     }

//     // On Unix-like systems, detach by redirecting standard I/O.
//     #[cfg(not(windows))]
//     {
//         cmd.stdin(Stdio::null())
//             .stdout(Stdio::null())
//             .stderr(Stdio::null());
//     }

//     // Spawn the bacon process (detached).
//     let _child = cmd.spawn()?;
//     println!("Bacon process detached.");
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    // use regex::Regex;
    // use std::process::Command as StdCommand;

    #[test]
    fn test_bacon_version() {
        // Create a command for the binary "bacon" that is assumed to be in the environment.
        Command::new("bacon")
            .arg("--version")
            .assert()
            .stdout(predicate::str::is_match(r"^bacon\s+\d+\.\d+\.\d+").unwrap());
    }

    #[test]
    fn test_bacon_help() {
        Command::new("bacon")
            .arg("--help")
            .assert()
            .stdout(predicate::str::contains("Usage:"));
    }
}
