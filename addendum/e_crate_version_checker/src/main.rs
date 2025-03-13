//! Main entry point for the `crate_version_checker` application.
//!
//! This application accepts a crate name as a command-line argument,
//! prints the current User-Agent, queries crates.io for the latest version,
//! compares it with the currently running version, and prompts the user
//! to update the crate if a new version is available.

mod e_crate_info;
mod e_crate_update;

use e_crate_update::show_current_version;
use e_crate_version_checker::e_interactive_crate_upgrade::interactive_crate_upgrade;
use std::env;
use std::process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print the current User-Agent string.
    println!("{}", show_current_version());

    // Collect command-line arguments.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <crate_name>", args[0]);
        process::exit(1);
    }
    let crate_name = &args[1];

    // Interrogate Cargo for the local version of the target crate.
    let current_version =
        lookup_local_version_via_cargo(crate_name).unwrap_or_else(|| "0.0.0".to_string());
    interactive_crate_upgrade(crate_name, &current_version)?;
    Ok(())
}

use serde::Deserialize;
use std::process::Command;

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    _manifest_path: String,
}

/// Looks up the local version of a crate in the current workspace by running `cargo metadata`.
///
/// Returns `Some(version)` if a package with the given name is found, or `None` otherwise.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to look up.
///
/// # Example
///
/// ```rust,no_run
/// let version = lookup_local_version_via_cargo("mkcmt").expect("Crate not found");
/// println!("Local version of mkcmt is {}", version);
/// ```
pub fn lookup_local_version_via_cargo(crate_name: &str) -> Option<String> {
    // Run `cargo metadata` with no dependencies.
    let output = Command::new("cargo")
        .args(&["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .ok()?;
    if !output.status.success() {
        eprintln!("cargo metadata failed");
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let metadata: Metadata = serde_json::from_str(&stdout).ok()?;
    // Find the package with the matching name.
    metadata
        .packages
        .into_iter()
        .find(|pkg| pkg.name == crate_name)
        .map(|pkg| pkg.version)
}
