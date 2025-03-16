//! Main entry point for the `crate_version_checker` application.
//!
//! This application accepts a crate name as a command-line argument,
//! prints the current User-Agent, queries crates.io for the latest version,
//! compares it with the currently running version, and prompts the user
//! to update the crate if a new version is available.

//mod e_crate_info;
mod e_crate_update;

use e_crate_version_checker::e_interactive_crate_upgrade::interactive_crate_upgrade;
use e_crate_version_checker::prelude::*;
use std::env;
use std::process;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // required : register the current crate in the User-Agent string.
    register_user_crate!();

    // Collect command-line arguments.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <crate_name>", args[0]);
        process::exit(1);
    }
    let crate_name = &args[1];

    // Interrogate Cargo for the local version of the target crate.
    let current_version =
        version::lookup_local_version_via_cargo(crate_name).unwrap_or_else(|| "0.0.0".to_string());
    interactive_crate_upgrade(crate_name, &current_version, 5)?;
    Ok(())
}
