#![allow(unused_variables)]
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

use cargo_e::e_cli::RunAll;
use cargo_e::e_processmanager::ProcessManager;
use cargo_e::e_runner;
use cargo_e::e_runner::is_active_rust_script;
use cargo_e::e_target::CargoTarget;
use cargo_e::e_target::TargetKind;
use cargo_e::prelude::*;
use cargo_e::Cli;
use clap::Parser;
#[cfg(feature = "tui")]
use crossterm::terminal::size;
#[cfg(feature = "check-version-program-start")]
use e_crate_version_checker::prelude::*;
use once_cell::sync::Lazy;
// Plugin API
// Imports for plugin system
#[cfg(feature = "uses_plugins")]
use cargo_e::e_target::TargetOrigin;
#[cfg(feature = "uses_plugins")]
use cargo_e::plugins::plugin_api::{load_plugins, Target as PluginTarget};
#[cfg(not(target_os = "windows"))]
use std::os::unix::process;
#[cfg(target_os = "windows")]
use std::os::windows::process;
#[cfg(feature = "uses_plugins")]
use std::path::PathBuf;
// Plugin loader modules
// Plugin modules (only when plugin system is enabled)
// Plugin modules moved to the library crate under `cargo_e::plugins`
//#[cfg(feature = "uses_plugins")]
//mod plugins;

static EXPLICIT: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));
static EXTRA_ARGS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off")).init();
    log::trace!(
        "cargo-e starting with args: {:?}",
        std::env::args().collect::<Vec<_>>()
    );

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

    // // Here we run "cargo run --example funny_example" so that the build phase and runtime output are distinct.
    // println!("=== Running: cargo run --example funny_example ===");
    // let mut command = Command::new("cargo");
    // command.args(&[
    //     "run",
    //     "--example",
    //     "funny_example",
    //     "--color", "always",
    //     "--message-format=json-render-diagnostics",
    // ]);

    // // First run without an estimated output size.
    // let cargo_handle = command.spawn_cargo_capture(
    //     Some(stdout_dispatcher.clone()),
    //     Some(stderr_dispatcher.clone()),
    //     Some(progress_dispatcher.clone()),
    //     Some(stage_dispatcher.clone()),
    //     None, // no estimate provided
    // );
    // let result = cargo_handle.wait().expect("Failed during run");

    // let _ = cargo_e::e_runner::register_ctrlc_handler();
    #[cfg(feature = "check-version-program-start")]
    {
        e_crate_version_checker::register_user_crate!();
        // Attempt to retrieve the version from `cargo-e -v`
        let version = local_crate_version_via_executable("cargo-e")
            .map(|(_, version)| version)
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

        // Use the version from `lookup_cargo_e_version` if valid,
        // otherwise fallback to the compile-time version.
        let _ = interactive_crate_upgrade(env!("CARGO_PKG_NAME"), &version, cli.wait);
    }

    let manager = ProcessManager::new(&cli);
    // Control the maximum number of Cargo processes running concurrently.
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    // Collect built-in Cargo targets
    #[allow(unused_mut)]
    let mut examples =
        cargo_e::e_collect::collect_all_targets(cli.workspace, num_threads).unwrap_or_default();
    // Collect plugin-provided targets and merge
    #[cfg(feature = "uses_plugins")]
    {
        #[allow(unused_imports)]
        use cargo_e::e_target::{CargoTarget, TargetKind, TargetOrigin};
        use cargo_e::plugins::plugin_api::{load_plugins, Target as PluginTarget};
        use std::path::PathBuf;
        let cwd = std::env::current_dir()?;
        log::trace!("Invoking load_plugins() to discover plugins");
        for plugin in load_plugins(&cli, manager.clone())? {
            if plugin.matches(&cwd) {
                let plugin_path = plugin.source().map(PathBuf::from).unwrap_or(cwd.clone());
                for mut pt in plugin.collect_targets(&cwd)? {
                    // If plugin provided a full CargoTarget, use it directly.
                    if let Some(ct) = pt.cargo_target.take() {
                        examples.push(ct);
                    } else {
                        let reported = pt
                            .metadata
                            .as_ref()
                            .map(PathBuf::from)
                            .unwrap_or_else(|| cwd.clone());
                        examples.push(CargoTarget {
                            name: pt.name.clone(),
                            display_name: pt.name.clone(),
                            manifest_path: cwd.clone(),
                            kind: TargetKind::Plugin,
                            extended: false,
                            toml_specified: false,
                            origin: Some(TargetOrigin::Plugin {
                                plugin_path: plugin_path.clone(),
                                reported,
                            }),
                        });
                    }
                }
            }
        }
    }
    use std::collections::HashSet;

    // After collecting all samples, deduplicate them.
    let mut seen = HashSet::new();
    let unique_examples: Vec<CargoTarget> = examples
        .clone()
        .into_iter()
        .filter(|e| {
            let key = (e.name.clone(), e.extended, e.kind, e.toml_specified);
            seen.insert(key)
        })
        .collect();

    let builtin_examples: Vec<&CargoTarget> = examples
        .iter()
        .filter(|e| e.toml_specified && matches!(e.kind, TargetKind::Example))
        .collect();

    let builtin_binaries: Vec<&CargoTarget> = examples
        .iter()
        .filter(|e| e.toml_specified && e.kind == TargetKind::Binary)
        .collect();
    if let Some(explicit) = cli.explicit_example.clone() {
        if let Some(explicit_example) = cli.explicit_example.clone() {
            let mut explicit_lock = EXPLICIT.lock().unwrap();
            *explicit_lock = explicit_example;
        }
        {
            let mut extra_lock = EXTRA_ARGS.lock().unwrap();
            *extra_lock = cli.extra.clone();
        }
        println!("Explicit: {:?}", explicit);
        // Now call run_rust_script_with_ctrlc_handling
        e_runner::run_scriptisto_with_ctrlc_handling(explicit.clone(), cli.extra.clone());
        e_runner::run_rust_script_with_ctrlc_handling(explicit.clone(), cli.extra.clone());
        // Search the discovered targets for one with the matching name.
        // Try examples first.
        if let Some(target) = examples.iter().find(|t| t.name == explicit) {
            // Plugin target in explicit mode
            if target.kind == TargetKind::Plugin {
                #[cfg(feature = "uses_plugins")]
                {
                    use cargo_e::plugins::plugin_api::{load_plugins, Target as PluginTarget};
                    // Find corresponding plugin and run in-process
                    let cwd = std::env::current_dir()?;
                    if let Some(origin) = &target.origin {
                        if let TargetOrigin::Plugin {
                            plugin_path,
                            reported,
                        } = origin
                        {
                            let pt = PluginTarget {
                                name: target.name.clone(),
                                metadata: Some(reported.to_string_lossy().to_string()),
                                cargo_target: None,
                            };
                            for plugin in load_plugins(&cli, manager.clone())? {
                                if plugin.source().map(|s| PathBuf::from(s))
                                    == Some(plugin_path.clone())
                                {
                                    // Delegate execution to run_with_manager
                                    let result =
                                        plugin.run_with_manager(manager.clone(), &cli, target)?;
                                    if let Some(status) = result {
                                        println!("Plugin exited with code: {:?}", status.code());
                                    }
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            } else {
                #[cfg(feature = "tui")]
                if cli.tui {
                    do_tui_and_exit(manager, &cli, &unique_examples);
                }
                cargo_e::e_runner::run_example(manager.clone(), &cli, target)?;
            }
        }
        // If not found among examples, search for a binary with that name.
        else if let Some(target) = examples
            .iter()
            .find(|t| t.kind == TargetKind::Binary && t.name == explicit)
        {
            #[cfg(feature = "tui")]
            if cli.tui {
                do_tui_and_exit(manager, &cli, &unique_examples);
            }
            cargo_e::e_runner::run_example(manager.clone(), &cli, target)?;
        } else {
            eprintln!(
                "error: 0 named '{}' found in examples or binaries.",
                explicit
            );

            // no exact match found: perform a partial search over the unique examples.
            let query = explicit.to_lowercase();
            let fuzzy_matches: Vec<CargoTarget> = unique_examples
                .iter()
                .filter(|t| t.name.to_lowercase().contains(&query))
                .cloned()
                .collect();
            if fuzzy_matches.is_empty() {
                std::process::exit(1);
            } else {
                println!(
                    "partial search results {} for '{}':",
                    fuzzy_matches.len(),
                    explicit
                );
                if cli.run_all != RunAll::NotSpecified {
                    //PROMPT cargo_e::e_prompts::prompt(&"", 2).ok();
                    // Pass in your default packages, which are now generic.
                    cargo_e::e_runall::run_all_examples(manager, &cli, &fuzzy_matches)?;
                    return Ok(());
                }

                #[cfg(feature = "tui")]
                if cli.tui {
                    do_tui_and_exit(manager, &cli, &fuzzy_matches);
                }
                cli_loop(manager, &cli, &fuzzy_matches, &[], &[]);
            }
            std::process::exit(1);
        }
    }

    if cli.run_all != RunAll::NotSpecified {
        cargo_e::e_runall::run_all_examples(manager, &cli, &unique_examples)?;
        return Ok(());
    }

    if builtin_examples.len() == 1 {
        #[cfg(feature = "tui")]
        if cli.tui {
            do_tui_and_exit(manager, &cli, &unique_examples);
        }

        let example = builtin_examples[0];
        let message = format!(
            "{} example found. run? (Yes / no / edit / tui / info)     waiting {} seconds.",
            example.name, cli.wait
        );
        match cargo_e::e_prompts::prompt(&message, cli.wait.max(3))? {
            Some('y') | Some(' ') | Some('\n') => {
                println!("running {}...", example.name);
                cargo_e::e_runner::run_example(manager, &cli, &example)?;
            }
            Some('n') => {
                //println!("exiting without running.");
                cli_loop(
                    manager,
                    &cli,
                    &unique_examples,
                    &builtin_examples,
                    &builtin_binaries,
                );
                std::process::exit(0);
            }
            Some('e') => {
                use futures::executor::block_on;
                block_on(cargo_e::e_findmain::open_vscode_for_sample(&example));
            }
            Some('i') => {
                futures::executor::block_on(crate::e_runner::open_ai_summarize_for_target(example));
                cargo_e::e_prompts::prompt_line("", 120).ok();
                cli_loop(
                    manager,
                    &cli,
                    &unique_examples,
                    &builtin_examples,
                    &builtin_binaries,
                );
            }
            Some('t') => {
                #[cfg(feature = "tui")]
                do_tui_and_exit(manager, &cli, &examples);
                #[cfg(not(feature = "tui"))]
                {
                    println!("tui not enabled.");
                    cli_loop(
                        manager,
                        &cli,
                        &unique_examples,
                        &builtin_examples,
                        &builtin_binaries,
                    );
                    std::process::exit(0);
                }
            }
            Some(other) => {
                println!("{}. exiting.", other);
                std::process::exit(1);
            }
            None => {
                cargo_e::e_runner::run_example(manager, &cli, builtin_examples[0])?;
                std::process::exit(0);
            }
        }
        // Only one example exists: run it.
    } else if builtin_examples.is_empty() && builtin_binaries.len() == 1 {
        //provide_notice_of_no_examples(manager.clone(), &cli, &unique_examples).ok();
        #[cfg(feature = "tui")]
        if cli.tui {
            do_tui_and_exit(manager.clone(), &cli, &unique_examples);
        }
        // No examples, but one binary exists.
        let binary = builtin_binaries[0];
        // Prompt the user for what to do.
        let message = format!(
            "{} binary found.  run? (Yes / no / edit / tui / info)     waiting {} seconds.",
            binary.name, cli.wait
        );
        match cargo_e::e_prompts::prompt(&message, cli.wait) {
            Ok(Some('y')) | Ok(Some(' ')) => {
                cargo_e::e_runner::run_example(manager.clone(), &cli, binary)?;
            }
            Ok(Some('i')) => {
                futures::executor::block_on(crate::e_runner::open_ai_summarize_for_target(binary));
                cargo_e::e_prompts::prompt_line("", 120).ok();
            }
            Ok(Some('n')) => {
                cli_loop(
                    manager,
                    &cli,
                    &unique_examples,
                    &builtin_examples,
                    &builtin_binaries,
                );
                std::process::exit(0);
            }
            Ok(Some('e')) => {
                use futures::executor::block_on;
                block_on(cargo_e::e_findmain::open_vscode_for_sample(binary));
            }
            Ok(Some('t')) => {
                #[cfg(feature = "tui")]
                {
                    do_tui_and_exit(manager, &cli, &examples);
                }
            }
            Ok(Some(other)) => {
                println!("Unrecognized option: {:?}. Exiting.", other);
                std::process::exit(0);
            }
            Ok(None) => {
                cargo_e::e_runner::run_example(manager.clone(), &cli, binary)?;
            }
            Err(err) => {
                eprintln!("Failed to read prompt: {}", err);
                // either exit or propagate
                return Err(err.into());
            }
        }
        // match cargo_e::e_prompts::prompt(&message, cli.wait) {
        //     Some('y') => {
        //         // Run the binary.
        //         cargo_e::e_runner::run_example(manager.clone(),&cli, binary)?;
        //     }
        //     Some('i') => {
        //         futures::executor::block_on(crate::e_runner::open_ai_summarize_for_target(binary));
        //         cargo_e::e_prompts::prompt_line("", 120).ok();
        //     }
        //     Some('n') => {
        //         //println!("exiting without running.");
        //         cli_loop(manager, &cli, &unique_examples, &builtin_examples, &builtin_binaries);
        //         std::process::exit(0);
        //     }
        //     Some('e') => {
        //         use futures::executor::block_on;
        //         block_on(cargo_e::e_findmain::open_vscode_for_sample(binary));
        //     }
        //     Some('t') => {
        //         // Open the TUI.
        //         #[cfg(feature = "tui")]
        //         {
        //             do_tui_and_exit(manager, &cli, &examples);
        //         }
        //     }
        //     _ => {
        //         println!("Unrecognized option: {:?}. Exiting.", other);
        //         std::process::exit(0);
        //     }
        // }
    } else {
        provide_notice_of_no_examples(manager.clone(), &cli, &unique_examples).ok();

        #[cfg(feature = "tui")]
        if cli.tui {
            do_tui_and_exit(manager, &cli, &unique_examples);
        }
        cli_loop(
            manager,
            &cli,
            &unique_examples,
            &builtin_examples,
            &builtin_binaries,
        );
        // if builtin_examples.len() + builtin_binaries.len() > 1 {
        //     //select_and_run_target(&cli, &examples, &builtin_examples, &builtin_binaries)?;
        // } else {
        //     eprintln!("Available: {:#?}", examples);
        //     std::process::exit(1);
        // }
    }
    Ok(())
}

#[allow(dead_code)]
fn do_tui_and_exit(manager: Arc<ProcessManager>, cli: &Cli, unique_examples: &[CargoTarget]) -> ! {
    #[cfg(feature = "tui")]
    {
        let ret = cargo_e::e_tui::tui_interactive::launch_tui(manager, cli, unique_examples);
        if let Err(e) = ret {
            eprintln!("TUI Error: {:?}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }
    #[cfg(not(feature = "tui"))]
    {
        // If TUI is not enabled, just print a message and exit.
        eprintln!("TUI is not supported in this build. Exiting.");
        cli_loop(manager, &cli, &unique_examples, &[], &[]);
        std::process::exit(0);
    }
}

fn provide_notice_of_no_examples(
    manager: Arc<ProcessManager>,
    cli: &Cli,
    examples: &[CargoTarget],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ex_count = examples
        .iter()
        .filter(|e| matches!(e.kind, TargetKind::Example))
        .count();
    let bin_count = examples
        .iter()
        .filter(|e| e.kind == TargetKind::Binary)
        .count();

    println!(
        "({} targets: {} examples, {} binaries)",
        examples.len(),
        ex_count,
        bin_count
    );

    if cli.wait != 0 {
        println!(
            "== press q, t, wait for {} seconds, or other key to continue.",
            cli.wait
        );
    }
    if let Some(line) =
        cargo_e::e_prompts::prompt_line_with_poll_opts(cli.wait, &[' ', 'c', 't', 'q', '\n'], None)?
    {
        let trimmed = line.trim();
        // Continue if input is empty or "c"; otherwise, quit.
        #[cfg(feature = "tui")]
        {
            if trimmed.eq_ignore_ascii_case("t") {
                do_tui_and_exit(manager, cli, examples);
            }
        }

        if trimmed.eq_ignore_ascii_case("q") {
            println!("quit.");
            std::process::exit(0);
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

/// The result returned by the selection loop.
enum LoopResult {
    Quit,
    Run(std::process::ExitStatus, usize), // second value is the current offset/page index
}

/// The selection function: displays targets, waits for input, and returns a LoopResult.
fn select_and_run_target_loop(
    manager: Arc<ProcessManager>,
    cli: &Cli,
    unique_targets: &[CargoTarget],
    builtin_examples: &[&CargoTarget],
    builtin_binaries: &[&CargoTarget],
    start_offset: usize, // new parameter for starting page offset
) -> Result<LoopResult, Box<dyn Error + Send + Sync>> {
    let current_offset = start_offset;
    let prompt_loop = format!(
        "== # to run, tui, e<#> edit, i<#> info, 'q' to quit (waiting {} seconds) ",
        cli.wait
    );
    // Build a combined list: examples first, then binaries.
    let mut combined: Vec<(String, &CargoTarget)> = Vec::new();
    for target in unique_targets {
        let label = target.display_label();
        combined.push((label, target));
    }

    combined.sort_by(|(a, ex_a), (b, ex_b)| {
        if a == b {
            ex_a.name.cmp(&ex_b.name)
        } else {
            a.cmp(b)
        }
    });
    // Determine the required padding width based on the number of targets
    let pad_width = combined.len().to_string().len();
    // Load run history from file.
    let manifest_dir = cargo_e::e_manifest::find_manifest_dir()?;
    let history_path = manifest_dir.join("run_history.txt");
    let run_history = cargo_e::e_parser::read_run_history(&history_path);

    // Print the list.
    if cli.paging {
        //println!("Available:");
        let total = combined.len();
        let mut current_index = current_offset;
        while current_index < total {
            // Reserve two lines for the prompt/status.
            let rows = {
                #[cfg(feature = "tui")]
                {
                    let (_, term_rows) = size()?; // get terminal size
                    std::cmp::min(term_rows, 9) // choose the lesser of terminal rows and 9
                }
                #[cfg(not(feature = "tui"))]
                {
                    9
                }
            };
            let page_lines = {
                #[cfg(feature = "tui")]
                {
                    let (_, term_rows) = size()?;
                    // If the terminal has fewer than 9 rows, subtract 2 for the prompt/input,
                    // but ensure that we display at least one line.
                    if term_rows < 9 {
                        std::cmp::max((term_rows.saturating_sub(2)) as usize, 1)
                    } else {
                        9
                    }
                }
                #[cfg(not(feature = "tui"))]
                {
                    9
                }
            };
            let end_index = usize::min(current_index + page_lines as usize, total);
            for (i, (target_type, target)) in combined[current_index..end_index].iter().enumerate()
            {
                // let base_line = format!(
                //     "  {:>2}: [{}] {}",
                //     current_index + i + 1,
                //     target_type,
                //     target.name
                // );
                // Use relative numbering if enabled (reset per page), otherwise absolute numbering.
                let line_number = if cli.relative_numbers {
                    i + 1
                } else {
                    current_index + i + 1
                };
                let base_line = format!(
                    "  {:>width$}: [{}] {} ", //{:?} {:?}",
                    line_number,
                    target_type,
                    target.display_name,
                    //target.origin.as_ref().unwrap(),
                    //target.name,
                    width = pad_width
                );
                let styled_line = if let Some(count) = run_history.get(&target.name) {
                    // If the target was run before, highlight in blue and append run count.
                    let line_with_count = format!(
                        "{} ({} run{})",
                        base_line,
                        count,
                        if *count == 1 { "" } else { "s" }
                    );
                    #[cfg(feature = "tui")]
                    crossterm::style::Stylize::bold(crossterm::style::Stylize::with(
                        line_with_count,
                        crossterm::style::Color::Blue,
                    ))
                } else {
                    // Otherwise, print in default white.
                    #[cfg(feature = "tui")]
                    crossterm::style::Stylize::with(base_line, crossterm::style::Color::White)
                };
                #[cfg(not(feature = "tui"))]
                let styled_line = base_line;
                println!("{}", styled_line);

                //                println!("  {:>2}: [{}] {}", current_index + i + 1, target_type, target.name);
            }
            if end_index < total {
                println!("{}", &prompt_loop);
                io::Write::flush(&mut io::stdout())?;
                let mut quick_exit_keys = vec!['q', 't', ' '];
                if cli.relative_numbers {
                    // If relative numbering is enabled, add all digits as additional quick exit keys.
                    quick_exit_keys.extend('0'..='9');
                }
                let mut allowed_chars: Vec<char> = ('0'..='9').collect();
                allowed_chars.push('e');
                allowed_chars.push('E');
                allowed_chars.push('i');
                allowed_chars.push('I');
                if let Some(line) = cargo_e::e_prompts::prompt_line_with_poll_opts(
                    cli.wait.max(3),
                    &quick_exit_keys,
                    Some(&allowed_chars),
                )? {
                    if !line.trim().is_empty() {
                        // Early selection.
                        let selection = line;
                        // current_index = total; // break out of paging loop.
                        // println!("{} currindex", current_index);
                        return process_input(manager, &selection, &combined, cli, current_index);
                    }
                }
                current_index = end_index;
            } else {
                break;
            }
        }
    } else {
        // Non-paging mode: print all targets.
        //println!("Available:");
        for (i, (target_type, target)) in combined.iter().enumerate() {
            let base_line = format!(
                "  {:>width$}: [{}] {}",
                i + 1,
                target_type,
                target.name,
                width = pad_width
            );

            // let base_line = format!("  {:>2}: [{}] {}", i + 1, target_type, target.name);
            let styled_line = if let Some(count) = run_history.get(&target.name) {
                // If the target was run before, highlight in blue and append run count.
                let line_with_count = format!(
                    "{} ({} run{})",
                    base_line,
                    count,
                    if *count == 1 { "" } else { "s" }
                );
                #[cfg(feature = "tui")]
                crossterm::style::Stylize::bold(crossterm::style::Stylize::with(
                    line_with_count,
                    crossterm::style::Color::Blue,
                ))
            } else {
                // Otherwise, print in default white.
                #[cfg(feature = "tui")]
                crossterm::style::Stylize::with(base_line, crossterm::style::Color::White)
            };
            #[cfg(not(feature = "tui"))]
            let styled_line = base_line;
            println!("{}", styled_line);

            //println!("  {:>2}: [{}] {}", i + 1, target_type, target.name);
        }
    }

    // Final prompt.
    println!("* {}", &prompt_loop);
    io::Write::flush(&mut io::stdout())?;
    let final_input = if combined.len() > 9 {
        let quick_exit_keys = ['q', 't', ' '];
        let allowed_digits: Vec<char> = ('0'..='9').collect();
        cargo_e::e_prompts::prompt_line_with_poll_opts(
            cli.wait.max(3),
            &quick_exit_keys,
            Some(&allowed_digits),
        )?
    } else {
        // For fewer targets, use a simple single-character prompt.
        let mut allowed: Vec<char> = ('0'..='9').collect();
        allowed.extend(['e']);
        allowed.extend(['i']);
        allowed.extend(['I']);
        allowed.extend(['E']);
        let mut quick_exit_keys: Vec<char> = vec!['q', 't', ' '];
        quick_exit_keys.extend('0'..='9');
        cargo_e::e_prompts::prompt_line_with_poll_opts(
            cli.wait.max(3),
            &quick_exit_keys,
            Some(&allowed),
        )?
    }
    .unwrap_or_default();
    println!("{}", &final_input);
    process_input(manager, &final_input, &combined, cli, 0)
}
pub fn append_run_history(target_name: &str) -> io::Result<()> {
    use std::io::Write;
    let manifest_dir = cargo_e::e_manifest::find_manifest_dir()?;
    let history_path = manifest_dir.join("run_history.txt");

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)?;
    writeln!(file, "{}", target_name)?;
    Ok(())
}
/// Processes the final input string and returns a LoopResult.
fn process_input(
    manager: Arc<ProcessManager>,
    input: &str,
    combined: &[(String, &CargoTarget)],
    cli: &Cli,
    offset: usize, // offset for the current page
) -> Result<LoopResult, Box<dyn Error + Send + Sync>> {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("q") {
        Ok(LoopResult::Quit)
    } else if trimmed.eq_ignore_ascii_case("t") {
        #[cfg(feature = "tui")]
        {
            let tui_examples: Vec<CargoTarget> =
                combined.iter().map(|&(_, ex)| ex.clone()).collect();
            do_tui_and_exit(manager, cli, &tui_examples);
        }
        #[cfg(not(feature = "tui"))]
        {
            Ok(LoopResult::Quit)
        }
    } else if trimmed.to_lowercase().starts_with("i") {
        // Handle the "edit" command: e<num>
        let num_str = trimmed[1..].trim();
        if let Ok(rel_index) = num_str.parse::<usize>() {
            let abs_index = if cli.relative_numbers {
                offset + rel_index - 1
            } else {
                rel_index - 1
            };
            if abs_index > combined.len() {
                eprintln!("error: Invalid target number for edit: {}", trimmed);
                Ok(LoopResult::Quit)
            } else {
                let (target_type, target) = &combined[abs_index];
                println!("info {} \"{}\"...", target_type, target.name);
                futures::executor::block_on(crate::e_runner::open_ai_summarize_for_target(target));
                cargo_e::e_prompts::prompt_line("", 120).ok();
                // After editing, you might want to pause briefly or simply return to the menu.
                Ok(LoopResult::Run(
                    <std::process::ExitStatus as process::ExitStatusExt>::from_raw(0),
                    offset,
                ))
            }
        } else {
            eprintln!("error: Invalid edit command: {}", trimmed);
            Ok(LoopResult::Quit)
        }
    } else if trimmed.to_lowercase().starts_with("e") {
        // Handle the "edit" command: e<num>
        let num_str = trimmed[1..].trim();
        if let Ok(rel_index) = num_str.parse::<usize>() {
            let abs_index = if cli.relative_numbers {
                offset + rel_index - 1
            } else {
                rel_index - 1
            };
            if abs_index > combined.len() {
                eprintln!("error: Invalid target number for edit: {}", trimmed);
                Ok(LoopResult::Quit)
            } else {
                let (target_type, target) = &combined[abs_index];
                println!("editing {} \"{}\"...", target_type, target.name);
                // Call the appropriate function to open the target for editing.
                // For example, if using VSCode:
                use futures::executor::block_on;
                block_on(cargo_e::e_findmain::open_vscode_for_sample(target));
                // After editing, you might want to pause briefly or simply return to the menu.
                Ok(LoopResult::Run(
                    <std::process::ExitStatus as process::ExitStatusExt>::from_raw(0),
                    offset,
                ))
            }
        } else {
            eprintln!("error: Invalid edit command: {}", trimmed);
            Ok(LoopResult::Quit)
        }
    } else if let Ok(rel_index) = trimmed.parse::<usize>() {
        let abs_index = if cli.relative_numbers {
            offset + rel_index - 1
        } else {
            rel_index - 1
        };
        if abs_index > combined.len() {
            eprintln!("invalid number: {}", trimmed);
            Ok(LoopResult::Quit)
        } else {
            let (target_type, target) = &combined[abs_index];
            if cli.print_program_name {
                println!("running {} \"{}\"...", target_type, target.name);
            }
            let status = e_runner::run_example(manager, &cli, target)?;
            let _ = append_run_history(&target.name.clone());
            let message = if cli.print_exit_code {
                format!(
                    "Exitcode {:?}. Press any key to continue...",
                    status.unwrap().code()
                )
            } else {
                "".to_string()
            };
            //PROMPT let _ = cargo_e::e_prompts::prompt(&message, cli.wait)?;

            Ok(LoopResult::Run(
                status.unwrap_or(<std::process::ExitStatus as process::ExitStatusExt>::from_raw(0)),
                offset,
            ))
        }
    } else {
        Ok(LoopResult::Quit)
    }
}

/// The outer CLI loop. It repeatedly calls the selection loop.
/// If a target exits with an "interrupted" code (e.g. 130), it re‑displays the menu.
/// If the user quits (input "q"), it exits.
fn cli_loop(
    manager: Arc<ProcessManager>,
    cli: &Cli,
    unique_examples: &[CargoTarget],
    builtin_examples: &[&CargoTarget],
    builtin_binaries: &[&CargoTarget],
) {
    let mut current_offset = 0; // persist the current page offset
    let manager_clone = manager.clone();
    loop {
        match select_and_run_target_loop(
            manager_clone.clone(),
            cli,
            unique_examples,
            builtin_examples,
            builtin_binaries,
            current_offset,
        ) {
            Ok(LoopResult::Quit) => {
                println!("quitting.");
                break;
            }
            Ok(LoopResult::Run(status, new_offset)) => {
                // Update the offset so the next iteration starts at the same page
                current_offset = new_offset;
                if status.code() == Some(130) {
                    println!("interrupted (Ctrl+C). returning to menu.");
                    continue;
                } else {
                    println!("finished with status: {:?}", status.code());
                    continue;
                }
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                std::process::exit(1); // Exit with an error code
            }
        }
    }
}

trait JoinTimeout {
    fn join_timeout(self, timeout: std::time::Duration) -> Result<(), ()>;
}

impl<T> JoinTimeout for std::thread::JoinHandle<T> {
    fn join_timeout(self, timeout: std::time::Duration) -> Result<(), ()> {
        let result = std::thread::sleep(timeout);
        match self.join() {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

pub fn run_rust_script_with_ctrlc_handling() {
    let explicit = {
        let lock = EXPLICIT.lock().unwrap_or_else(|e| {
            eprintln!("Failed to acquire lock: {}", e);
            std::process::exit(1); // Exit the program if the lock cannot be obtained
        });
        lock.clone() // Clone the data to move it into the thread
    };

    let explicit_path = Path::new(&explicit); // Construct Path outside the lock

    if explicit_path.exists() {
        match is_active_rust_script(&explicit_path) {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                // Handle the error locally without propagating it
                eprintln!("Failed to check if the file is a rust-script");
                std::process::exit(1); // Exit with an error code
            }
        }

        let extra_args = EXTRA_ARGS.lock().unwrap(); // Locking the Mutex to access the data
        let extra_str_slice: Vec<String> = extra_args.iter().cloned().collect();

        // Run the child process in a separate thread to allow Ctrl+C handling
        let handle = std::thread::spawn(move || {
            let extra_str_slice_cloned = extra_str_slice.clone();
            let child = e_runner::run_rust_script(
                &explicit,
                &extra_str_slice_cloned
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|| {
                eprintln!("Failed to run rust-script: {:?}", &explicit);
                std::process::exit(1); // Exit with an error code
            });
        });

        // Wait for the thread to complete, but with a timeout
        let timeout = std::time::Duration::from_secs(10);
        match handle.join_timeout(timeout) {
            Ok(_) => {
                println!("Child process finished successfully.");
            }
            Err(_) => {
                eprintln!("Child process took too long to finish. Exiting...");
                std::process::exit(1); // Exit if the process takes too long
            }
        }
    }
}
