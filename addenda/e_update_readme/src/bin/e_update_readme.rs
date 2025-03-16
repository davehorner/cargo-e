use regex::Regex;
use std::env;
use std::fs;
use std::path::Path;

/// Prints usage information.
fn print_usage() {
    println!("Usage: e_update_readme [-p] [FILE]");
    println!("  -p    Also update the parent's README.md (assumed to be at ../README.md) but only if its content matches the local file.");
    println!("  FILE  The README.md file to update (defaults to ./README.md if not specified).");
}

/// Updates the specified README.md file with the version from Cargo.toml.
/// If `update_parent` is true, it first compares the parent's README.md with the local file and errors out if they differ.
///
/// # Arguments
///
/// * `target_file` - The path to the README.md file to update.
/// * `update_parent` - If true, also check and update the parent's README.md.
pub fn update_readme(target_file: &str, update_parent: bool) {
    // Read Cargo.toml and extract the version string.
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let version_line = cargo_toml
        .lines()
        .find(|line| line.trim_start().starts_with("version"))
        .expect("Version not found in Cargo.toml");
    let version = version_line
        .split('=')
        .nth(1)
        .expect("Malformed version line")
        .trim()
        .trim_matches('"');
    println!("Found version: {}", version);

    // Read the specified local README.md.
    let local_readme = fs::read_to_string(target_file)
        .unwrap_or_else(|_| panic!("Failed to read {}", target_file));

    // If -p flag is provided, compare parent's README.md with the local file before updating.
    if update_parent {
        let parent_path = Path::new("..").join("README.md");
        let parent_readme = fs::read_to_string(&parent_path)
            .unwrap_or_else(|_| panic!("Failed to read parent's README.md at {:?}", parent_path));
        if parent_readme != local_readme {
            eprintln!("Error: Parent README.md content differs from local file. Aborting update.");
            std::process::exit(1);
        }
    }

    // Create a new README content by replacing any semantic version pattern (e.g. >0.0.0<) with the version from Cargo.toml.
    let re = Regex::new(r">(?P<version>\d+\.\d+\.\d+)<").expect("Invalid regex");
    let new_readme = re.replace_all(&local_readme, |_: &regex::Captures| {
        format!(">{}<", version)
    });

    // Write the updated content back to the local file.
    fs::write(target_file, new_readme.as_ref())
        .unwrap_or_else(|_| panic!("Failed to write updated {}", target_file));
    println!("Updated {} with version {}", target_file, version);

    if update_parent {
        let parent_path = Path::new("..").join("README.md");
        // Write the updated content to the parent's README.md.
        fs::write(&parent_path, new_readme.as_ref()).unwrap_or_else(|_| {
            panic!(
                "Failed to write updated parent's README.md at {:?}",
                parent_path
            )
        });
        println!("Copied updated content to {:?}", parent_path);

        // Verify that the parent's README.md matches the updated content.
        let updated_parent =
            fs::read_to_string(&parent_path).expect("Failed to read updated parent's README.md");
        if updated_parent == new_readme.as_ref() {
            println!("Parent README.md content matches local update.");
        } else {
            eprintln!("Error: Parent README.md content does not match local update.");
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // If no arguments (other than the program name) are given, print usage.
    if args.len() == 1 {
        print_usage();
        std::process::exit(0);
    }

    // Defaults: update local README.md.
    let mut update_parent = false;
    let mut target_file = String::from("README.md");

    // Process command-line arguments.
    for arg in &args[1..] {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "-p" => {
                update_parent = true;
            }
            file => {
                // If an argument is not a flag, treat it as the file path.
                target_file = file.to_string();
            }
        }
    }

    update_readme(&target_file, update_parent);
}
