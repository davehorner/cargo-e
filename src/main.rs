//! # cargo-e
//!
//! `cargo-e` is a command-line tool to run and explore examples and binaries from Rust projects.
//! Unlike `cargo run --example`, it will run the example directly if only one exists.
//! 
//! ## Features
//! - Runs single examples automatically
//! - Supports examples in different locations (bins, workspaces, etc.)
//! - Provides better navigation for Rust projects
//!
//! ## Quick Start
//! ```sh
//! cargo install cargo-e
//! cargo e
//! ```
//!
//! See the [GitHub repository](https://github.com/davehorner/cargo-e) for more details.

use cargo_e::prelude::*;
use cargo_e::{locate_manifest, Cli, Example, TargetKind};
use clap::Parser;


pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();

    // If the first argument after the binary name is "e", remove it.
    if args.len() > 1 && args[1] == "e" {
        args.remove(1);
    }
    let cli = Cli::parse_from(args);
    if cli.version {
        cargo_e::e_cli::print_version_and_features();
        exit(0);
    }
    

    #[cfg(feature = "equivalent")]
    run_equivalent_example(&cli); // this std::process::exit()s

    debug!("CLI options: {:?}", cli);

    let manifest_current = locate_manifest(false).unwrap_or_default();
    debug!("Nearest        Cargo.toml: {}", manifest_current);

    let manifest_workspace = locate_manifest(true).unwrap_or_default();
    debug!("Workspace root Cargo.toml: {}", manifest_workspace);

    let mut manifest_infos = Vec::new();
    let cwd = env::current_dir()?;
    let built_in_manifest = cwd.join("Cargo.toml");
    if built_in_manifest.exists() {
        // Cargo.toml exists in the current working directory.
        info!("Found Cargo.toml in current directory: {}", cwd.display());
    } else if let Ok(manifest_dir) = env::var("CARGO_MANIFEST") {
        let manifest_path = Path::new(&manifest_dir);
        if manifest_path.join("Cargo.toml").exists() {
            info!(
                "Changing working directory to manifest folder: {}",
                manifest_path.display()
            );
            env::set_current_dir(manifest_path)?;
        } else {
            eprintln!(
                "Error: CARGO_MANIFEST is set to '{}', but no Cargo.toml found there.",
                manifest_dir
            );
            return Err("No Cargo.toml found in CARGO_MANIFEST folder.".into());
        }
    } else {
        eprintln!(
            "Error: No Cargo.toml found in the current directory and CARGO_MANIFEST is not set."
        );
        return Err("No Cargo.toml found.".into());
    }
    let prefix = "** ".to_string();
    manifest_infos.push((prefix, built_in_manifest, false));

    // Extended samples: assume they are located in the "examples" folder relative to cwd.
    let extended_root = cwd.join("examples");
    if extended_root.exists() {
        // Each subdirectory with a Cargo.toml is an extended sample.
        for entry in fs::read_dir(&extended_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("Cargo.toml").exists() {
                // Use the directory name as the display prefix.
                let prefix = path.file_name().unwrap().to_string_lossy().to_string();
                let manifest_path = path.join("Cargo.toml");
                if !manifest_path.exists() {
                    debug!("DEBUG: Manifest path {:?} does not exist", manifest_path);
                    continue;
                }
                manifest_infos.push((prefix, manifest_path, true));
            }
        }
    } else {
        debug!(
            "DEBUG: Extended samples directory {:?} does not exist.",
            extended_root
        );
    }

    debug!("DEBUG: manifest infos: {:?}", manifest_infos);

    // Control the maximum number of Cargo processes running concurrently.
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    // let samples = collect_samples_concurrently(manifest_infos, max_concurrency)?;
    let examples = cargo_e::e_collect::collect_all_samples(cli.workspace, num_threads)?;
    // let examples = collect_samples(manifest_infos, num_threads)?;
    // println!("Collected {} samples:", examples.len());
    // for sample in &examples {
    //     println!("{:?}", sample);
    // }

    let builtin_examples: Vec<&Example> = examples
        .iter()
        .filter(|e| !e.extended && matches!(e.kind, TargetKind::Example))
        .collect();
    if builtin_examples.is_empty() && !cli.tui {
        println!("No examples found!");
        exit(1);
    }

    if let Some(ref ex) = cli.explicit_example {
        let ex = Example {
            name: ex.to_string(),
            display_name: format!("explicit example"),
            manifest_path: "Cargo.toml".to_string(),
            kind: TargetKind::Example,
            extended: false, // assume it's a standard example
        };
        cargo_e::run_example(&ex, &cli.extra)?;
    } else if builtin_examples.len() == 1 && !cli.tui {
        cargo_e::run_example(&builtin_examples[0], &Vec::new())?;
    } else {
        println!("DEBUG: Launching TUI with examples: {:?}", examples);
        // Multiple examples available.
        if cli.tui {
            // #[cfg(feature = "tui_autolaunch")]
            // {
            //     // Launch browser-based TUI.
            //     if let Err(e) = ebrowser_tui::main() {
            //         eprintln!("Error launching browser TUI: {:?}", e);
            //         exit(1);
            //     }
            // }

            #[cfg(feature = "tui")]
            {
                if cli.tui {
                    // If the tui flag is active, also add binaries.
                    // println!("DEBUG: Launching TUI with examples: {:?}", examples);
                    // match collect_binaries("builtin bin", &PathBuf::from("Cargo.toml"), false) {
                    //     Ok(bins) => {
                    //         examples.extend(bins);
                    //         eprintln!(
                    //             "DEBUG: After collecting binaries, examples = {:?}",
                    //             examples
                    //         );
                    //     }
                    //     Err(e) => eprintln!("DEBUG: Failed to collect binaries: {:?}", e),
                    // }
                    //     let extended_targets: Vec<Example> = examples
                    // .iter()
                    // .filter(|ex| ex.extended)
                    // .cloned()
                    // .collect();

                    // for target in extended_targets {
                    //     let folder_path = Path::new("examples").join(&target.name);
                    //     match collect_extended_binaries(&folder_path, &target.name) {
                    //         Ok(mut bins) => {
                    //             examples.extend(bins);
                    //             eprintln!("DEBUG: Extended target '{}' binaries added", target.name);
                    //         }
                    //         Err(e) => {
                    //             eprintln!("DEBUG: Failed to collect binaries for folder '{}': {:?}", target.name, e);
                    //         }
                    //     }
                    // }
                }
            }

            #[cfg(all(feature = "tui"))]
            {
                if let Err(e) = cargo_e::e_tui::tui_interactive::launch_tui(&cli, &examples) {
                    eprintln!("Error launching interactive TUI: {:?}", e);
                    exit(1);
                }
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!(
                    "Available examples: {:?}",
                    examples
                );
                exit(1);
            }
        } else {
            eprintln!("Multiple examples found: {:?}", examples);
            eprintln!("Please specify which example to run.");
            exit(1);
        }
    }
    Ok(())
}

#[cfg(feature = "equivalent")]
fn run_equivalent_example(cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
       .arg("--example");
    if let Some(explicit) = &cli.explicit_example {
        cmd.arg(explicit);
    }
    if !cli.extra.is_empty() {
        cmd.arg("--").args(cli.extra.clone());
    }
    cmd.stdin(std::process::Stdio::inherit())
       .stdout(Stdio::inherit())
       .stderr(Stdio::inherit());
    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}

