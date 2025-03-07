use std::process::{Command, Child};
use std::path::Path;
use std::sync::{Arc, Mutex};
use ctrlc;
use std::error::Error;
use crate::Example;

/// Runs the given example (or binary) target.
pub fn run_example(example: &Example, extra_args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("cargo");

    if example.extended {
        println!("Running extended example in folder: examples/{}", example.name);
        cmd.arg("run").current_dir(format!("examples/{}", example.name));
    } else {
        println!("Running: cargo run --release --example {}", example.name);
        cmd.args(&["run", "--release", "--example", &example.name]);
    }

    if !extra_args.is_empty() {
        cmd.arg("--").args(extra_args);
    }

    let child = cmd.spawn()?;
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
