use crate::prelude::*;
// #[cfg(not(feature = "equivalent"))]
// use ctrlc;
use crate::Example;

/// In "equivalent" mode, behave exactly like "cargo run --example <name>"
#[cfg(feature = "equivalent")]
pub fn run_example(example: &Example, extra_args: &[String]) -> Result<(), Box<dyn Error>> {
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
pub fn run_example(example: &Example, extra_args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("cargo");

    if example.extended {
        println!(
            "Running extended example in folder: examples/{}",
            example.name
        );
        cmd.arg("run")
            .current_dir(format!("examples/{}", example.name));
    } else {
        cmd.args(["run", "--release", "--example", &example.name]);
    }

    if !extra_args.is_empty() {
        println!("Extra args: {:?}", extra_args);
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

    let child = cmd.spawn()?;
    use std::sync::{Arc, Mutex};
    let child_arc = Arc::new(Mutex::new(child));
    let child_for_handler = Arc::clone(&child_arc);

    ctrlc::set_handler(move || {
        eprintln!("Ctrl+C pressed, terminating process...");
        let mut child = child_for_handler.lock().unwrap();
        let _ = child.kill();
    })?;

    let status = child_arc.lock().unwrap().wait()?;
    println!("Process exited with status: {:?}", status.code());
    Ok(())
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
