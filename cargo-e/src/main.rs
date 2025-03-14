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

#[cfg(feature = "check-version-program-start")]
use e_crate_version_checker::prelude::*;

use cargo_e::prelude::*;
use cargo_e::{Cli, Example, TargetKind};
use clap::Parser;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off")).init();

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
    run_equivalent_example(&cli).ok(); // this std::process::exit()s

    #[cfg(feature = "check-version-program-start")]
    {
        // Attempt to retrieve the version from `cargo-e -v`
        let version = local_crate_version_via_executable("cargo-e")
            .map(|(_, version)| version)
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

        // Use the version from `lookup_cargo_e_version` if valid,
        // otherwise fallback to the compile-time version.
        interactive_crate_upgrade(env!("CARGO_PKG_NAME"), &version, cli.wait)?;
    }

    //let manifest_current =cargo_e::locate_manifest(false).unwrap_or_default();
    //let manifest_workspace = cargo_e::locate_manifest(true).unwrap_or_default();
    // let manifest_current = PathBuf::from(&cargo_e::locate_manifest(cli.workspace)?);
    // println!("{}",manifest_current.display());
    // let prefix = "** ".to_string();
    // let mut manifest_infos = Vec::new();
    // manifest_infos.push((prefix, manifest_current, false));
    // let manifest_infos = vec![(prefix, manifest_current, false)];

    //let cwd = env::current_dir()?;
    // let built_in_manifest = cwd.join("Cargo.toml");
    // if built_in_manifest.exists() {
    //     debug!("Cargo.toml in current directory: {}", cwd.display());
    // } else if let Ok(manifest_dir) = env::var("CARGO_MANIFEST") {
    //     let manifest_path = Path::new(&manifest_dir);
    //     if manifest_path.join("Cargo.toml").exists() {
    //         info!("cwd CARGO_MANIFEST folder: {}", manifest_path.display());
    //         env::set_current_dir(manifest_path)?;
    //     } else {
    //         eprintln!(
    //             "error: CARGO_MANIFEST is set to '{}', but no Cargo.toml found there.",
    //             manifest_dir
    //         );
    //         return Err("No Cargo.toml found in CARGO_MANIFEST folder.".into());
    //     }
    // } else {
    //     if(manifest_)
    //     eprintln!(
    //         "error: No Cargo.toml found in the current directory and CARGO_MANIFEST is not set."
    //     );
    //     return Err("No Cargo.toml found.".into());
    // }
    // let prefix = "** ".to_string();
    // manifest_infos.push((prefix, built_in_manifest, false));

    // Extended samples: assume they are located in the "examples" folder relative to cwd.
    // let extended_root = cwd.join("examples");
    // if extended_root.exists() {
    //     // Each subdirectory with a Cargo.toml is an extended sample.
    //     for entry in fs::read_dir(&extended_root)? {
    //         let entry = entry?;
    //         let path = entry.path();
    //         if path.is_dir() && path.join("Cargo.toml").exists() {
    //             // Use the directory name as the display prefix.
    //             let prefix = path.file_name().unwrap().to_string_lossy().to_string();
    //             let manifest_path = path.join("Cargo.toml");
    //             if !manifest_path.exists() {
    //                 debug!("manifest path {:?} does not exist", manifest_path);
    //                 continue;
    //             }
    //             manifest_infos.push((prefix, manifest_path, true));
    //         }
    //     }
    // } else {
    //     debug!(
    //         "extended samples directory {:?} does not exist.",
    //         extended_root
    //     );
    // }

    // Control the maximum number of Cargo processes running concurrently.
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let examples = cargo_e::e_collect::collect_all_samples(cli.workspace, num_threads)?;

    let builtin_examples: Vec<&Example> = examples
        .iter()
        .filter(|e| !e.extended && matches!(e.kind, TargetKind::Example))
        .collect();
    let builtin_binaries: Vec<&Example> = examples
        .iter()
        .filter(|e| e.kind == TargetKind::Binary && !e.extended)
        .collect();

    if let Some(explicit) = cli.explicit_example {
        // Run the explicitly specified target.
        let ex = Example {
            name: explicit.clone(),
            display_name: explicit.clone(),
            manifest_path: "Cargo.toml".to_string(),
            kind: TargetKind::Example,
            extended: false, // assume standard example for simplicity
        };
        cargo_e::run_example(&ex, &cli.extra)?;
    } else if builtin_examples.len() == 1 {
        let example = builtin_examples[0];
        let message = format!(
            "one example \"{}\" found. (Yes / no / edit / tui)     waiting {} seconds.",
            example.name, cli.wait
        );
        match cargo_e::e_prompts::prompt(&message, cli.wait)? {
            Some('y') => {
                println!("Running example {}...", example.name);
                cargo_e::run_example(example, &cli.extra)?;
            }
            Some('n') => {
                println!("Exiting without running the example.");
                std::process::exit(0);
            }
            Some('e') => {
                use futures::executor::block_on;
                block_on(cargo_e::e_findmain::open_vscode_for_sample(example));
            }
            Some('t') => {
                #[cfg(feature = "tui")]
                {
                    cargo_e::e_tui::tui_interactive::launch_tui(&cli, &examples)?;
                }
            }
            Some(other) => {
                println!("Unrecognized option: {}. Exiting.", other);
                std::process::exit(1);
            }
            None => {
                cargo_e::run_example(builtin_examples[0], &cli.extra)?;
                std::process::exit(0);
            }
        }
        // Only one example exists: run it.
    } else if builtin_examples.is_empty() && builtin_binaries.len() == 1 {
        // No examples, but one binary exists.
        let binary = builtin_binaries[0];
        // Prompt the user for what to do.
        let message = format!(
            "one binary \"{}\" found.  continue? (yes / No / edit / tui)     waiting {} seconds.",
            binary.name, cli.wait
        );
        match cargo_e::e_prompts::prompt(&message, cli.wait)? {
            Some('y') => {
                // Run the binary.
                cargo_e::run_example(binary, &cli.extra)?;
            }
            Some('n') => {
                println!("Exiting without running the binary.");
                std::process::exit(0);
            }
            Some('e') => {
                use futures::executor::block_on;
                block_on(cargo_e::e_findmain::open_vscode_for_sample(binary));
            }
            Some('t') => {
                // Open the TUI.
                #[cfg(feature = "tui")]
                {
                    cargo_e::e_tui::tui_interactive::launch_tui(&cli, &examples)?;
                }
            }
            _ => {
                //                println!("Unrecognized option: {:?}. Exiting.", other);
                std::process::exit(1);
            }
        }
    } else if examples.is_empty() && !cli.tui {
        println!("No examples or binaries available.");
    } else {
        // Otherwise, show the TUI.
        if cli.tui {
            #[cfg(feature = "tui")]
            {
                if let Err(e) = cargo_e::e_tui::tui_interactive::launch_tui(&cli, &examples) {
                    eprintln!("error launching TUI: {:?}", e);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Available targets: {:#?}", examples);
            std::process::exit(1);
        }
    }
    Ok(())
}

#[cfg(feature = "equivalent")]
fn run_equivalent_example(cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--example");
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
