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

#[cfg(feature = "tui")]
use crossterm::terminal::size;
#[cfg(feature = "check-version-program-start")]
use e_crate_version_checker::prelude::*;

#[cfg(not(target_os = "windows"))]
use std::os::unix::process;
#[cfg(target_os = "windows")]
use std::os::windows::process;

use cargo_e::{prelude::*, Example};
use cargo_e::{Cli, TargetKind};
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

    let _ = cargo_e::e_runner::register_ctrlc_handler();
    #[cfg(feature = "check-version-program-start")]
    {
        e_crate_version_checker::register_user_crate!();
        // Attempt to retrieve the version from `cargo-e -v`
        let version = local_crate_version_via_executable("cargo-e")
            .map(|(_, version)| version)
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

        // Use the version from `lookup_cargo_e_version` if valid,
        // otherwise fallback to the compile-time version.
        interactive_crate_upgrade(env!("CARGO_PKG_NAME"), &version, cli.wait)?;
    }

    // Control the maximum number of Cargo processes running concurrently.
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let examples = cargo_e::e_collect::collect_all_samples(cli.workspace, num_threads)?;
    use std::collections::HashSet;

    // After collecting all samples, deduplicate them.
    let mut seen = HashSet::new();
    let unique_examples: Vec<cargo_e::Example> = examples
        .clone()
        .into_iter()
        .filter(|e| {
            // Create a key using a unique combination of properties.
            // You may need to adjust this depending on what distinguishes duplicates in your context.
            let key = (e.name.clone(), e.extended, e.kind);
            seen.insert(key)
        })
        .collect();

    let builtin_examples: Vec<&cargo_e::Example> = unique_examples
        .iter()
        .filter(|e| !e.extended && matches!(e.kind, cargo_e::TargetKind::Example))
        .collect();

    let builtin_binaries: Vec<&cargo_e::Example> = unique_examples
        .iter()
        .filter(|e| !e.extended && e.kind == cargo_e::TargetKind::Binary)
        .collect();

    if let Some(explicit) = cli.explicit_example.clone() {
        // Search the discovered targets for one with the matching name.
        // Try examples first.
        if let Some(target) = examples.iter().find(|t| t.name == explicit) {
            cargo_e::run_example(target, &cli.extra)?;
        }
        // If not found among examples, search for a binary with that name.
        else if let Some(target) = examples
            .iter()
            .find(|t| t.kind == TargetKind::Binary && t.name == explicit)
        {
            cargo_e::run_example(target, &cli.extra)?;
        } else {
            eprintln!(
                "error: 0 named '{}' found in examples or binaries.",
                explicit
            );

            // no exact match found: perform a partial search over the unique examples.
            let query = explicit.to_lowercase();
            let fuzzy_matches: Vec<Example> = unique_examples
                .iter()
                .filter(|t| t.name.to_lowercase().contains(&query))
                .cloned()
                .collect();
            if fuzzy_matches.is_empty() {
                std::process::exit(1);
            } else {
                println!("partial search results for '{}':", explicit);
                cli_loop(&cli, &fuzzy_matches, &[], &[])?;
            }
            std::process::exit(1);
        }
    } else if builtin_examples.len() == 1 {
        let example = builtin_examples[0];
        let message = format!(
            "{} example found. run? (Yes / no / edit / tui)     waiting {} seconds.",
            example.name, cli.wait
        );
        match cargo_e::e_prompts::prompt(&message, cli.wait)? {
            Some('y') => {
                println!("running {}...", example.name);
                cargo_e::run_example(example, &cli.extra)?;
            }
            Some('n') => {
                println!("exiting without running.");
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
                println!("{}. exiting.", other);
                std::process::exit(1);
            }
            None => {
                cargo_e::run_example(builtin_examples[0], &cli.extra)?;
                std::process::exit(0);
            }
        }
        // Only one example exists: run it.
    } else if builtin_examples.is_empty() && builtin_binaries.len() == 1 {
        provide_notice_of_no_examples(&cli, &unique_examples)?;
        // No examples, but one binary exists.
        let binary = builtin_binaries[0];
        // Prompt the user for what to do.
        let message = format!(
            "{} binary found.  run? (yes / No / edit / tui)     waiting {} seconds.",
            binary.name, cli.wait
        );
        match cargo_e::e_prompts::prompt(&message, cli.wait)? {
            Some('y') => {
                // Run the binary.
                cargo_e::run_example(binary, &cli.extra)?;
            }
            Some('n') => {
                println!("exiting without running.");
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
                std::process::exit(0);
            }
        }
    } else {
        provide_notice_of_no_examples(&cli, &unique_examples)?;
        if cli.tui {
            #[cfg(feature = "tui")]
            {
                cargo_e::e_tui::tui_interactive::launch_tui(&cli, &unique_examples)?;
                std::process::exit(0);
            }
        }

        cli_loop(&cli, &unique_examples, &builtin_examples, &builtin_binaries)?;
        // if builtin_examples.len() + builtin_binaries.len() > 1 {
        //     //select_and_run_target(&cli, &examples, &builtin_examples, &builtin_binaries)?;
        // } else {
        //     eprintln!("Available: {:#?}", examples);
        //     std::process::exit(1);
        // }
    }
    Ok(())
}

fn provide_notice_of_no_examples(cli: &Cli, examples: &[Example]) -> Result<(), Box<dyn Error>> {
    let ex_count = examples
        .iter()
        .filter(|e| matches!(e.kind, cargo_e::TargetKind::Example))
        .count();
    let bin_count = examples
        .iter()
        .filter(|e| e.kind == cargo_e::TargetKind::Binary)
        .count();

    println!(
        "0 built-in examples ({} alternatives: {} examples, {} binaries).\n\
        == press q, t, wait for 3 seconds, or other key to continue.",
        examples.len(),
        ex_count,
        bin_count
    );
    if let Some(line) =
        cargo_e::e_prompts::prompt_line_with_poll_opts(3, &[' ', 'c', 't', 'q'], None)?
    {
        let trimmed = line.trim();
        // Continue if input is empty or "c"; otherwise, quit.
        #[cfg(feature = "tui")]
        {
            if trimmed.eq_ignore_ascii_case("t") {
                cargo_e::e_tui::tui_interactive::launch_tui(cli, examples)?;
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

// Prompts the user with the available targets and then runs the selected target.
// Examples are numbered first, followed by binaries, and the user can also press 't' to start the TUI.
// #[allow(dead_code)]
// fn select_and_run_target(
//     cli: &cargo_e::Cli,
//     examples: &[cargo_e::Example],
//     builtin_examples: &[&cargo_e::Example],
//     builtin_binaries: &[&cargo_e::Example],
// ) -> Result<(), Box<dyn std::error::Error>> {
//     // Build a combined list with a label indicating its type.
//     let mut combined: Vec<(&str, &cargo_e::Example)> = Vec::new();
//     // First add examples.
//     for ex in builtin_examples {
//         combined.push(("ex.", ex));
//     }
//     // Then add binaries.
//     for bin in builtin_binaries {
//         combined.push(("bin", bin));
//     }

//     // Optionally sort each group by name; examples come before binaries.
//     combined.sort_by(|(type_a, ex_a), (type_b, ex_b)| {
//         if type_a == type_b {
//             ex_a.name.cmp(&ex_b.name)
//         } else {
//             type_a.cmp(type_b) // "example" sorts before "binary"
//         }
//     });

//     // Load run history from file.
//     let manifest_dir = env!("CARGO_MANIFEST_DIR");
//     let history_path = format!("{}/run_history.txt", manifest_dir);
//     let run_history = cargo_e::e_parser::read_run_history(&history_path);

//     let mut selection_input: Option<String> = None;
//     if cli.paging {
//         #[cfg(feature = "tui")]
//         use crossterm::style::{Color, Stylize};
//         #[cfg(feature = "tui")]
//         use crossterm::terminal;
//         // Get terminal size for paging.
//         #[cfg(feature = "tui")]
//         let (_cols, rows) = terminal::size()?;
//         // Reserve two lines for the prompt/status.
//         #[cfg(not(feature = "tui"))]
//         let rows = 20;
//         let page_lines = if rows > 3 {
//             (rows - 2) as usize
//         } else {
//             rows as usize
//         };

//         //println!("Available:");
//         let total = combined.len();
//         let mut current_index = 0;
//         // Print targets page by page.
//         while current_index < total {
//             let end_index = usize::min(current_index + page_lines, total);
//             for (i, (target_type, target)) in combined[current_index..end_index].iter().enumerate()
//             {
//                 //println!("  {:>2}: [{}] {}", current_index + i + 1, target_type, target.name);
//                 let base_line = format!(
//                     "  {:>2}: [{}] {}",
//                     current_index + i + 1,
//                     target_type,
//                     target.name
//                 );
//                 #[cfg(feature = "tui")]
//                 let styled_line = if let Some(count) = run_history.get(&target.name) {
//                     // If the target was run before, highlight in blue and append run count.
//                     let line_with_count = format!(
//                         "{} ({} run{})",
//                         base_line,
//                         count,
//                         if *count == 1 { "" } else { "s" }
//                     );
//                     #[cfg(feature = "tui")]
//                     line_with_count.with(Color::Blue).bold()
//                 } else {
//                     // Otherwise, print in default white.
//                     #[cfg(feature = "tui")]
//                     base_line.with(Color::White)
//                 };
//                 #[cfg(not(feature = "tui"))]
//                 let styled_line = base_line;
//                 println!("{}", styled_line);
//             }
//             // If there are more targets, allow early selection.
//             if end_index < total {
//                 println!("type number(s) to run, 't' to start TUI (waiting {} seconds)  (wait or press return/' '): ",cli.wait);
//                 io::Write::flush(&mut io::stdout())?;
//                 if let Some(line) = cargo_e::e_prompts::prompt_line_with_poll(cli.wait)? {
//                     if !line.trim().is_empty() {
//                         selection_input = Some(line);
//                         break;
//                     }
//                 }
//                 current_index = end_index;
//             } else {
//                 break;
//             }
//         }
//     } else {
//         // Print the list of available targets.
//         println!("Available:");
//         for (i, (target_type, target)) in combined.iter().enumerate() {
//             println!("  {:>2}: [{}] {}", i + 1, target_type, target.name);
//         }
//     }
//     let message = format!(
//         "press number to run, 't' to start TUI (waiting {} seconds):",
//         cli.wait
//     );

//     let final_input = if let Some(input) = selection_input {
//         input
//     } else if combined.len() > 9 {
//         cargo_e::e_prompts::prompt_line(&message, cli.wait)?.unwrap_or_default()
//     } else {
//         // For fewer targets, use a simple single-character prompt.
//         cargo_e::e_prompts::prompt(&message, cli.wait)?
//             .map(|c| c.to_string())
//             .unwrap_or_default()
//     };
//     match Some(final_input.trim()) {
//         Some(input) if input.eq_ignore_ascii_case("t") => {
//             #[cfg(feature = "tui")]
//             {
//                 cargo_e::e_tui::tui_interactive::launch_tui(cli, examples)?;
//             }
//             #[cfg(not(feature = "tui"))]
//             {
//                 eprintln!("TUI not supported in this build.");
//                 std::process::exit(1);
//             }
//         }
//         Some(input) => {
//             if !input.is_empty() {
//                 match input.parse::<usize>() {
//                     Ok(index) if index > 0 && index <= combined.len() => {
//                         let (target_type, target) = combined[index - 1];
//                         println!("Running {} \"{}\"...", target_type, target.name);
//                         cargo_e::run_example(target, &cli.extra)?;
//                     }
//                     _ => {
//                         eprintln!("Error: Invalid target number. ({})", input);
//                         std::process::exit(1);
//                     }
//                 }
//             }
//         }
//         None => {
//             println!("ok.");
//             std::process::exit(0);
//         }
//     }
//     Ok(())
// }

/// The result returned by the selection loop.
enum LoopResult {
    Quit,
    Run(std::process::ExitStatus),
}

/// The selection function: displays targets, waits for input, and returns a LoopResult.
fn select_and_run_target_loop(
    cli: &Cli,
    unique_targets: &[Example],
    builtin_examples: &[&Example],
    builtin_binaries: &[&Example],
) -> Result<LoopResult, Box<dyn Error>> {
    let prompt_loop = format!(
        "== # to run, tui, e<#> edit, 'q' to quit (waiting {} seconds) ",
        cli.wait
    );
    // Build a combined list: examples first, then binaries.
    let mut combined: Vec<(&str, &Example)> = Vec::new();
    for target in unique_targets {
        let label = if target.kind == cargo_e::TargetKind::Example {
            if target.extended {
                "exx"
            } else {
                "ex."
            }
        } else if target.kind == cargo_e::TargetKind::Binary {
            if target.extended {
                "binx"
            } else {
                "bin"
            }
        } else {
            "other"
        };
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
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let history_path = format!("{}/run_history.txt", manifest_dir);
    let run_history = cargo_e::e_parser::read_run_history(&history_path);

    // Print the list.
    if cli.paging {
        // Reserve two lines for the prompt/status.
        #[cfg(not(feature = "tui"))]
        let rows = 10;

        #[cfg(feature = "tui")]
        let (_cols, rows) = size()?;

        // Reserve two lines for prompt/status.
        let page_lines = if rows > 3 {
            (rows - 2) as usize
        } else {
            rows as usize
        };
        //println!("Available:");
        let total = combined.len();
        let mut current_index = 0;
        while current_index < total {
            let end_index = usize::min(current_index + page_lines, total);
            for (i, (target_type, target)) in combined[current_index..end_index].iter().enumerate()
            {
                // let base_line = format!(
                //     "  {:>2}: [{}] {}",
                //     current_index + i + 1,
                //     target_type,
                //     target.name
                // );
                let base_line = format!(
                    "  {:>width$}: [{}] {}",
                    current_index + i + 1,
                    target_type,
                    target.name,
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
                let quick_exit_keys = ['q', 't', ' '];
                let mut allowed_chars: Vec<char> = ('0'..='9').collect();
                allowed_chars.push('e');
                allowed_chars.push('E');
                if let Some(line) = cargo_e::e_prompts::prompt_line_with_poll_opts(
                    cli.wait,
                    &quick_exit_keys,
                    Some(&allowed_chars),
                )? {
                    if !line.trim().is_empty() {
                        // Early selection.
                        let selection = line;
                        // current_index = total; // break out of paging loop.
                        return process_input(&selection, &combined, cli);
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
            cli.wait,
            &quick_exit_keys,
            Some(&allowed_digits),
        )?
    } else {
        // For fewer targets, use a simple single-character prompt.
        let mut allowed: Vec<char> = ('0'..='9').collect();
        allowed.extend(['e']);
        let mut quick_exit_keys: Vec<char> = vec!['q', 't', ' '];
        quick_exit_keys.extend('0'..='9');
        cargo_e::e_prompts::prompt_line_with_poll_opts(cli.wait, &quick_exit_keys, Some(&allowed))?
    }
    .unwrap_or_default();
    println!("{}", &final_input);
    process_input(&final_input, &combined, cli)
}
pub fn append_run_history(target_name: &str) -> io::Result<()> {
    use std::io::Write;
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let history_path = format!("{}/run_history.txt", manifest_dir);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)?;
    writeln!(file, "{}", target_name)?;
    Ok(())
}
/// Processes the final input string and returns a LoopResult.
fn process_input(
    input: &str,
    combined: &[(&str, &Example)],
    cli: &Cli,
) -> Result<LoopResult, Box<dyn Error>> {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("q") {
        Ok(LoopResult::Quit)
    } else if trimmed.eq_ignore_ascii_case("t") {
        #[cfg(feature = "tui")]
        {
            let tui_examples: Vec<cargo_e::Example> =
                combined.iter().map(|&(_, ex)| ex.clone()).collect();
            cargo_e::e_tui::tui_interactive::launch_tui(cli, &tui_examples)?;
            std::process::exit(0);
        }
        #[cfg(not(feature = "tui"))]
        {
            Ok(LoopResult::Run(
                <std::process::ExitStatus as std::os::unix::process::ExitStatusExt>::from_raw(0),
            ))
        }
    } else if trimmed.to_lowercase().starts_with("e") {
        // Handle the "edit" command: e<num>
        let num_str = trimmed[1..].trim();
        if let Ok(index) = num_str.parse::<usize>() {
            if index == 0 || index > combined.len() {
                eprintln!("error: Invalid target number for edit: {}", trimmed);
                Ok(LoopResult::Quit)
            } else {
                let (target_type, target) = combined[index - 1];
                println!("editing {} \"{}\"...", target_type, target.name);
                // Call the appropriate function to open the target for editing.
                // For example, if using VSCode:
                use futures::executor::block_on;
                block_on(cargo_e::e_findmain::open_vscode_for_sample(target));
                // After editing, you might want to pause briefly or simply return to the menu.
                Ok(LoopResult::Run(
                    <std::process::ExitStatus as process::ExitStatusExt>::from_raw(0),
                ))
            }
        } else {
            eprintln!("error: Invalid edit command: {}", trimmed);
            Ok(LoopResult::Quit)
        }
    } else if let Ok(index) = trimmed.parse::<usize>() {
        if index == 0 || index > combined.len() {
            eprintln!("invalid number: {}", trimmed);
            Ok(LoopResult::Quit)
        } else {
            let (target_type, target) = combined[index - 1];
            println!("running {} \"{}\"...", target_type, target.name);
            let status = cargo_e::run_example(target, &cli.extra)?;
            let _ = append_run_history(&target.name.clone());
            println!(
                "Exitcode {:?}  Waiting for {} seconds...",
                status.code(),
                cli.wait
            );
            std::thread::sleep(std::time::Duration::from_secs(cli.wait));

            Ok(LoopResult::Run(status))
        }
    } else {
        Ok(LoopResult::Quit)
    }
}

/// The outer CLI loop. It repeatedly calls the selection loop.
/// If a target exits with an "interrupted" code (e.g. 130), it re‑displays the menu.
/// If the user quits (input "q"), it exits.
fn cli_loop(
    cli: &Cli,
    unique_examples: &[Example],
    builtin_examples: &[&Example],
    builtin_binaries: &[&Example],
) -> Result<(), Box<dyn Error>> {
    loop {
        match select_and_run_target_loop(cli, unique_examples, builtin_examples, builtin_binaries)?
        {
            LoopResult::Quit => {
                println!("quitting.");
                break;
            }
            LoopResult::Run(status) => {
                // Here, we treat exit code 130 as "interrupted".
                if status.code() == Some(130) {
                    println!("interrupted (Ctrl+C). returning to menu.");
                    continue;
                } else {
                    println!("finished with status: {:?}", status.code());
                    continue; //break;
                }
            }
        }
    }
    Ok(())
}
