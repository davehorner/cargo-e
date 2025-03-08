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


use cargo_e::{collect_workspace_members, locate_manifest, parse_available, Cli, Example, TargetKind};
use clap::Parser;
use crossterm::event::KeyEventKind;
use ctrlc;
use std::error::Error;
use std::process::Child;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{exit, Command},
    time::Instant,
};
#[cfg(feature = "concurrent")]
use threadpool::ThreadPool;



// /// Parses the stderr output to extract available items (e.g. binaries or examples)
// /// by looking for a marker of the form "Available {item}:".
// fn parse_available(stderr: &str, item: &str) -> Vec<String> {
//     let marker = format!("Available {}:", item);
//     let mut available = Vec::new();
//     let mut collecting = false;
//     for line in stderr.lines() {
//         if collecting {
//             let trimmed = line.trim();
//             if !trimmed.is_empty() {
//                 available.push(trimmed.to_string());
//             }
//         }
//         if line.contains(&marker) {
//             collecting = true;
//         }
//     }
//     available
// }

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();
    // If the first argument after the binary name is "e", remove it.
    // This prevents cargo's subcommand name from interfering with our explicit_example.
    if args.len() > 1 && args[1] == "e" {
        args.remove(1);
    }
    let cli = Cli::parse_from(args);

    println!("CLI options: {:?}", cli);

    let manifest_current = locate_manifest(false).unwrap_or_default();
    println!("Nearest        Cargo.toml: {}", manifest_current);

    let manifest_workspace = locate_manifest(true).unwrap_or_default();
    println!("Workspace root Cargo.toml: {}", manifest_workspace);

    let mut manifest_infos = Vec::new();
    let cwd = env::current_dir()?;
    let built_in_manifest = cwd.join("Cargo.toml");
    if built_in_manifest.exists() {
        // Cargo.toml exists in the current working directory.
        println!("Found Cargo.toml in current directory: {}", cwd.display());
    } else if let Ok(manifest_dir) = env::var("CARGO_MANIFEST") {
        let manifest_path = Path::new(&manifest_dir);
        if manifest_path.join("Cargo.toml").exists() {
            println!(
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
                    eprintln!("DEBUG: Manifest path {:?} does not exist", manifest_path);
                    continue;
                }
                manifest_infos.push((prefix, manifest_path, true));
            }
        }
    } else {
        eprintln!(
            "DEBUG: Extended samples directory {:?} does not exist.",
            extended_root
        );
    }

    eprintln!("DEBUG: manifest infos: {:?}", manifest_infos);

    // Control the maximum number of Cargo processes running concurrently.
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    // let samples = collect_samples_concurrently(manifest_infos, max_concurrency)?;
    let examples = collect_all_samples(cli.workspace, num_threads)?;
    // let examples = collect_samples(manifest_infos, num_threads)?;
    println!("Collected {} samples:", examples.len());
    for sample in &examples {
        println!("{:?}", sample);
    }

    let builtin_examples: Vec<&Example> = examples
        .iter()
        .filter(|e| !e.extended && matches!(e.kind, TargetKind::Example))
        .collect();
    if builtin_examples.is_empty() && !cli.tui {
        //  eprintln!("{}", stderr);
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
        run_example(&ex, &cli.extra)?;
    } else if builtin_examples.len() == 1 && !cli.tui {
        run_example(&builtin_examples[0], &Vec::new())?;
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
                if let Err(e) = tui_interactive::launch_tui(&cli, &examples) {
                    eprintln!("Error launching interactive TUI: {:?}", e);
                    exit(1);
                }
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("{}", stderr);
                eprintln!(
                    "TUI feature not enabled. Available examples: {:?}",
                    available_examples
                );
                exit(1);
            }
        } else {
            // eprintln!("{}", stderr);
            eprintln!("Multiple examples found: {:?}", examples);
            eprintln!("Please specify which example to run.");
            exit(1);
        }
    }
    Ok(())
}

#[cfg(feature = "tui")]
mod tui_interactive {
    use super::*;
    use cargo_e::{e_bacon, e_findmain, Example, TargetKind};
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
        execute,
        terminal::{
            disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
            LeaveAlternateScreen,
        },
    };
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout, Rect},
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem, ListState},
        Terminal,
    };
    use std::{collections::HashSet, thread, time::Duration};

    use crossterm::event::{poll, read};
    /// Flushes the input event queue, ignoring any stray Enter key events.
    pub fn flush_input() -> Result<(), Box<dyn std::error::Error>> {
        while poll(Duration::from_millis(0))? {
            if let Event::Key(key_event) = read()? {
                // Optionally, log or ignore specific keys.
                if key_event.code == KeyCode::Enter {
                    // Filtering out stray Return keys.
                    continue;
                }
                // You can also choose to ignore all events:
                // continue;
            }
        }
        Ok(())
    }

    /// Launches an interactive terminal UI for selecting an example.
    pub fn launch_tui(
        cli: &Cli,
        examples: &Vec<Example>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        flush_input()?; // Clear any buffered input (like stray Return keys)
        let mut exs = examples.clone();
        exs.sort();
        if exs.is_empty() {
            println!("No examples found!");
            return Ok(());
        }

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let history_path = format!("{}/run_history.txt", manifest_dir);
        let mut run_history: HashSet<String> = HashSet::new();
        if let Ok(contents) = fs::read_to_string(&history_path) {
            for line in contents.lines() {
                if !line.trim().is_empty() {
                    run_history.insert(line.trim().to_string());
                }
            }
        }
        println!("history");

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            Clear(ClearType::All)
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut exit_hover = false;

        'main_loop: loop {
            terminal.draw(|f| {
                let size = f.area();
                let area = Rect::new(0, 0, size.width, size.height);
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(2)
                    .constraints([Constraint::Min(0)].as_ref())
                    .split(area);
                let list_area = chunks[0];

                let left_text = format!("Select example ({} examples found)", exs.len());
                let separator = " ┃ ";
                let right_text = "Esc or q to EXIT";
                let title_line = if exit_hover {
                    Line::from(vec![
                        Span::raw(left_text),
                        Span::raw(separator),
                        Span::styled(right_text, Style::default().fg(Color::Yellow)),
                    ])
                } else {
                    Line::from(vec![
                        Span::raw(left_text),
                        Span::raw(separator),
                        Span::styled("Esc or q to ", Style::default().fg(Color::White)),
                        Span::styled("EXIT", Style::default().fg(Color::Red)),
                    ])
                };

                let block = Block::default().borders(Borders::ALL).title(title_line);
                // let items: Vec<ListItem> = exs.iter().map(|e| {
                //     let mut item = ListItem::new(e.as_str());
                //     if run_history.contains(e) {
                //         item = item.style(Style::default().fg(Color::Blue));
                //     }
                //     item
                // }).collect();
                let items: Vec<ListItem> = examples
                    .iter()
                    .map(|ex| {
                        let display_text = ex.display_name.clone();

                        let mut item = ListItem::new(display_text);
                        if run_history.contains(&ex.name) {
                            item = item.style(Style::default().fg(Color::Blue));
                        }
                        item
                    })
                    .collect();
                let list = List::new(items)
                    .block(block)
                    .highlight_style(Style::default().fg(Color::Yellow))
                    .highlight_symbol(">> ");
                f.render_stateful_widget(list, list_area, &mut list_state);
            })?;

            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key) => {
                        // Only process key-press events.
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break 'main_loop,
                            KeyCode::Down => {
                                let i = match list_state.selected() {
                                    Some(i) if i >= exs.len() - 1 => i,
                                    Some(i) => i + 1,
                                    None => 0,
                                };
                                list_state.select(Some(i));
                                // Debounce: wait a short while to avoid duplicate processing.
                                thread::sleep(Duration::from_millis(50));
                            }
                            KeyCode::Up => {
                                let i = match list_state.selected() {
                                    Some(0) | None => 0,
                                    Some(i) => i - 1,
                                };
                                list_state.select(Some(i));
                                // Debounce: wait a short while to avoid duplicate processing.
                                thread::sleep(Duration::from_millis(50));
                            }
                            KeyCode::PageDown => {
                                // Compute page size based on the terminal's current height.
                                let page = terminal
                                    .size()
                                    .map(|r| r.height.saturating_sub(4)) // subtract borders/margins; adjust as needed
                                    .unwrap_or(5)
                                    as usize;
                                let current = list_state.selected().unwrap_or(0);
                                let new = std::cmp::min(current + page, exs.len() - 1);
                                list_state.select(Some(new));
                            }
                            KeyCode::PageUp => {
                                let page = terminal
                                    .size()
                                    .map(|r| r.height.saturating_sub(4))
                                    .unwrap_or(5)
                                    as usize;
                                let current = list_state.selected().unwrap_or(0);
                                let new = if current < page { 0 } else { current - page };
                                list_state.select(Some(new));
                            }
                            KeyCode::Char('b') => {
                                if let Some(selected) = list_state.selected() {
                                    let sample = &examples[selected];
                                    // Run bacon in detached mode. Extra arguments can be added if needed.
                                    if let Err(e) = e_bacon::run_bacon(sample, &Vec::new()) {
                                        eprintln!("Error running bacon: {}", e);
                                    } else {
                                        println!("Bacon launched for sample: {}", sample.name);
                                    }
                                    reinit_terminal(&mut terminal)?;
                                }
                            }
                            KeyCode::Char('e') => {
                                if let Some(selected) = list_state.selected() {
                                    // Disable raw mode for debug printing.
                                    crossterm::terminal::disable_raw_mode()?;
                                    crossterm::execute!(
                                        std::io::stdout(),
                                        crossterm::terminal::LeaveAlternateScreen
                                    )?;
                                    // When 'e' is pressed, attempt to open the sample in VSCode.
                                    let sample = &examples[selected];
                                    println!("Opening VSCode for path: {}", sample.manifest_path);
                                    // Here we block on the asynchronous open_vscode call.
                                    // futures::executor::block_on(open_vscode(Path::new(&sample.manifest_path)));
                                    futures::executor::block_on(
                                        e_findmain::open_vscode_for_sample(sample),
                                    );
                                    std::thread::sleep(std::time::Duration::from_secs(5));
                                    reinit_terminal(&mut terminal)?;
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(selected) = list_state.selected() {
                                    run_piece(
                                        &examples,
                                        selected,
                                        &history_path,
                                        &mut run_history,
                                        &mut terminal,
                                        cli.wait,
                                    )?;
                                }
                            }
                            _ => {}
                        }
                    }
                    Event::Mouse(mouse_event) => {
                        let size = terminal.size()?;
                        let area = Rect::new(0, 0, size.width, size.height);
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .margin(2)
                            .constraints([Constraint::Min(0)].as_ref())
                            .split(area);
                        let list_area = chunks[0];
                        let title_row = list_area.y;
                        let title_start = list_area.x + 2;
                        let left_text = format!("Select example ({} examples found)", exs.len());
                        let separator = " ┃ ";
                        let right_text = "Esc or q to EXIT";
                        let offset = (left_text.len() + separator.len()) as u16;
                        let right_region_start = title_start + offset;
                        let right_region_end = right_region_start + (right_text.len() as u16);

                        match mouse_event.kind {
                            MouseEventKind::ScrollDown => {
                                let current = list_state.selected().unwrap_or(0);
                                let new = std::cmp::min(current + 1, exs.len() - 1);
                                list_state.select(Some(new));
                            }
                            MouseEventKind::ScrollUp => {
                                let current = list_state.selected().unwrap_or(0);
                                let new = if current == 0 { 0 } else { current - 1 };
                                list_state.select(Some(new));
                            }

                            MouseEventKind::Moved => {
                                if mouse_event.row == title_row {
                                    exit_hover = mouse_event.column >= right_region_start
                                        && mouse_event.column < right_region_end;
                                } else {
                                    exit_hover = false;
                                    let inner_y = list_area.y + 1;
                                    let inner_height = list_area.height.saturating_sub(2);
                                    if mouse_event.column >= list_area.x + 1
                                        && mouse_event.column < list_area.x + list_area.width - 1
                                        && mouse_event.row >= inner_y
                                        && mouse_event.row < inner_y + inner_height
                                    {
                                        let index = (mouse_event.row - inner_y) as usize;
                                        if index < exs.len() {
                                            list_state.select(Some(index));
                                        }
                                    }
                                }
                            }
                            MouseEventKind::Down(_) => {
                                if mouse_event.row == title_row
                                    && mouse_event.column >= right_region_start
                                    && mouse_event.column < right_region_end
                                {
                                    break 'main_loop;
                                }
                                let inner_y = list_area.y + 1;
                                let inner_height = list_area.height.saturating_sub(2);
                                if mouse_event.column >= list_area.x + 1
                                    && mouse_event.column < list_area.x + list_area.width - 1
                                    && mouse_event.row >= inner_y
                                    && mouse_event.row < inner_y + inner_height
                                {
                                    let index = (mouse_event.row - inner_y) as usize;
                                    if index < exs.len() {
                                        list_state.select(Some(index));
                                        run_piece(
                                            &exs,
                                            index,
                                            &history_path,
                                            &mut run_history,
                                            &mut terminal,
                                            cli.wait,
                                        )?;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        disable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            LeaveAlternateScreen,
            DisableMouseCapture,
            Clear(ClearType::All)
        )?;
        terminal.show_cursor()?;
        Ok(())
    }

    /// Reinitializes the terminal: enables raw mode, enters the alternate screen,
    /// enables mouse capture, clears the screen, and creates a new Terminal instance.
    /// This function updates the provided terminal reference.
    pub fn reinit_terminal(
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            Clear(ClearType::All)
        )?;
        *terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(())
    }


    /// Runs the given example (or binary) target. It leaves TUI mode, spawns a cargo process,
    /// installs a Ctrl+C handler to kill the process, waits for it to finish, updates history,
    /// flushes stray input, and then reinitializes the terminal.
    pub fn run_piece(
        examples: &Vec<Example>,
        index: usize,
        history_path: &str,
        run_history: &mut HashSet<String>,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        wait_secs: u64,
    ) -> Result<(), Box<dyn Error>> {
        let target = &examples[index];
        // Leave TUI mode before running the target.
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        let manifest_path = target.manifest_path.clone();

        let args: Vec<&str> = if target.kind == TargetKind::Example {
            if target.extended {
                println!("Running extended example with manifest: {}", manifest_path);
                // For workspace extended examples, assume the current directory is set correctly.
                vec!["run", "--manifest-path", &manifest_path]
            } else {
                println!(
                    "Running example: cargo run --release --example {}",
                    target.name
                );
                vec![
                    "run",
                    "--manifest-path",
                    &manifest_path,
                    "--release",
                    "--example",
                    &target.name,
                ]
            }
        } else {
            println!("Running binary: cargo run --release --bin {}", target.name);
            vec![
                "run",
                "--manifest-path",
                &manifest_path,
                "--release",
                "--bin",
                &target.name,
            ]
        };

        // If the target is extended, we want to run it from its directory.
        let current_dir = if target.extended {
            Path::new(&manifest_path).parent().map(|p| p.to_owned())
        } else {
            None
        };

        // Build the command.
        let mut cmd = Command::new("cargo");
        cmd.args(&args);
        if let Some(ref dir) = current_dir {
            cmd.current_dir(dir);
        }

        // Spawn the cargo process.
        let mut child = spawn_cargo_process(&args)?;
        println!("Process started. Press Ctrl+C to terminate or 'd' to detach...");
        let mut update_history = true;
        let status_code: i32;
        let mut detached = false;
        // Now we enter an event loop, periodically checking if the child has exited
        // and polling for keyboard input.
        loop {
            // Check if the child process has finished.
            if let Some(status) = child.try_wait()? {
                status_code = status.code().unwrap_or(1);
                println!("Process exited with status: {}", status_code);
                break;
            }
            // Poll for input events with a 100ms timeout.
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(event::KeyModifiers::CONTROL)
                    {
                        println!("Ctrl+C detected in event loop, killing process...");
                        child.kill()?;
                        update_history = false; // do not update history if cancelled
                                                // Optionally, you can also wait for the child after killing.
                        let status = child.wait()?;
                        status_code = status.code().unwrap_or(1);
                        break;
                    } else if key_event.code == KeyCode::Char('d') && key_event.modifiers.is_empty()
                    {
                        println!("'d' pressed; detaching process. Process will continue running.");
                        detached = true;
                        update_history = false;
                        // Do not kill or wait on the child.
                        // Break out of the loop immediately.
                        // We can optionally leave the process running.
                        status_code = 0;
                        break;
                    }
                }
            }
        }
        // Wrap the child process so that we can share it with our Ctrl+C handler.
        // let child_arc = Arc::new(Mutex::new(child));
        // let child_for_handler = Arc::clone(&child_arc);

        // Set up a Ctrl+C handler to kill the spawned process.
        // ctrlc::set_handler(move || {
        // eprintln!("Ctrl+C pressed, terminating process...");
        // if let Ok(mut child) = child_for_handler.lock() {
        // let _ = child.kill();
        // }
        // })?;

        // Wait for the process to finish.
        // let status = child_arc.lock().unwrap().wait()?;
        // println!("Process exited with status: {:?}", status.code());

        if !detached {
            // Only update run history if update_history is true and exit code is zero.
            if update_history && status_code == 0 {
                if run_history.insert(target.name.clone()) {
                    let history_data = run_history.iter().cloned().collect::<Vec<_>>().join("\n");
                    fs::write(history_path, history_data)?;
                }
            }
            println!(
                "Exitcode {}  Waiting for {} seconds...",
                status_code, wait_secs
            );
            std::thread::sleep(Duration::from_secs(wait_secs));
        }

        // Flush stray input events.
        while event::poll(std::time::Duration::from_millis(0))? {
            let _ = event::read()?;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Reinitialize the terminal.
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            crossterm::event::EnableMouseCapture,
            Clear(ClearType::All)
        )?;
        *terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(())
    }
}

#[cfg(not(feature = "tui"))]
mod tui_interactive {
    pub fn launch_tui(_available: &Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("TUI feature not enabled.");
        Ok(())
    }
}

/// Parses the workspace manifest (in TOML format) to return a vector of workspace member names and
/// their corresponding manifest paths. The workspace manifest is expected to have a [workspace]
/// table with a "members" array. Each member is joined with the workspace root directory.
// fn collect_workspace_members(
//     workspace_manifest: &str,
// ) -> Result<Vec<(String, PathBuf)>, Box<dyn Error>> {
//     let manifest_path = Path::new(workspace_manifest);
//     let workspace_root = manifest_path
//         .parent()
//         .ok_or("Cannot determine workspace root")?;
//     let manifest_contents = fs::read_to_string(workspace_manifest)?;
//     let value: Value = manifest_contents.parse::<Value>()?;
//     let mut members = Vec::new();
//     if let Some(ws) = value.get("workspace") {
//         if let Some(member_array) = ws.get("members").and_then(|v| v.as_array()) {
//             for member in member_array {
//                 if let Some(member_str) = member.as_str() {
//                     let member_path = workspace_root.join(member_str);
//                     let member_manifest = member_path.join("Cargo.toml");
//                     if member_manifest.exists() {
//                         // Use a prefix in the display name that shows the workspace member name.
//                         members.push((member_str.to_string(), member_manifest));
//                     }
//                 }
//             }
//         }
//     }
//     Ok(members)
// }

/// Runs `cargo run --bin` with the given manifest path and without specifying a binary name,
/// so that Cargo prints an error with a list of available binary targets.
/// Then parses that list to return a vector of Example instances, using the provided prefix.
pub fn collect_binaries(
    prefix: &str,
    manifest_path: &PathBuf,
    extended: bool,
) -> Result<Vec<Example>, Box<dyn Error>> {
    // Run `cargo run --bin --manifest-path <manifest_path>`.
    // Note: Cargo will return a non-zero exit code, but we only care about its stderr.
    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("--manifest-path")
        .arg(manifest_path)
        .output()?;

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let bin_names = parse_available(&stderr_str, "binaries");

    // Map each binary name into an Example instance.
    let binaries = bin_names
        .into_iter()
        .map(|name| {
            let display_name = if prefix.starts_with('$') {
                format!("{} > binary > {}", prefix, name)
            } else if extended {
                format!("{} {}", prefix, name)
            } else if prefix.starts_with("builtin") {
                format!("builtin binary: {}", name)
            } else {
                name.clone()
            };

            Example {
                name: name.clone(),
                display_name: display_name,
                manifest_path: manifest_path.to_string_lossy().to_string(),
                kind: TargetKind::Binary,
                extended: extended,
            }
        })
        .collect();

    Ok(binaries)
}

/// Runs `cargo run --example --manifest-path <manifest_path>` to trigger Cargo to
/// list available examples. Then it parses the stderr output using our generic parser.
pub fn collect_examples(
    prefix: &str,
    manifest_path: &PathBuf,
    extended: bool,
) -> Result<Vec<Example>, Box<dyn Error>> {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--example")
        .arg("--manifest-path")
        .arg(manifest_path)
        .output()?;

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    eprintln!("DEBUG: stderr (examples) = {:?}", stderr_str);

    let names = parse_available(&stderr_str, "examples");
    eprintln!("DEBUG: example names = {:?}", names);

    let examples = names
        .into_iter()
        .map(|name| {
            // If the prefix starts with '$', we assume this came from a workspace member.
            let display_name = if prefix.starts_with('$') {
                format!("{} > example > {}", prefix, name)
            } else if extended {
                format!("{} {}", prefix, name)
            } else if prefix.starts_with("builtin") {
                format!("builtin example: {}", name)
            } else {
                name.clone()
            };

            Example {
                name: name.clone(),
                display_name: display_name,
                manifest_path: manifest_path.to_string_lossy().to_string(),
                kind: TargetKind::Example,
                extended: extended,
            }
        })
        .collect();

    Ok(examples)
}

/// Given a vector of manifest infos (each a (prefix, manifest_path, extended) tuple),
/// this function concurrently collects samples (examples and binaries) from each manifest,
/// using up to `max_concurrency` concurrent Cargo commands.
// #[cfg(feature = "concurrent")]
// fn collect_samples_concurrently(
//     manifest_infos: Vec<(String, PathBuf, bool)>,
//     max_concurrency: usize,
// ) -> Result<Vec<Example>, Box<dyn Error>> {
//     let pool = ThreadPool::new(max_concurrency);
//     let (tx, rx) = std::sync::mpsc::channel();

//     for (prefix, manifest_path, extended) in manifest_infos {
//         let tx = tx.clone();
//         let prefix_clone = prefix.clone();
//         let manifest_clone = manifest_path.clone();
//         pool.execute(move || {
//             let mut results = Vec::new();
//             if let Ok(mut ex) = collect_examples(&prefix_clone, &manifest_clone, extended) {
//                 results.append(&mut ex);
//             }
//             if let Ok(mut bins) = collect_binaries(&prefix_clone, &manifest_clone, extended) {
//                 results.append(&mut bins);
//             }
//             tx.send(results).expect("Failed to send results");
//         });
//     }
//     drop(tx);
//     pool.join();

//     let mut all_samples = Vec::new();
//     for samples in rx {
//         all_samples.extend(samples);
//     }
//     Ok(all_samples)
// }

// Collects all sample targets (examples and binaries) from a list of manifests concurrently,
// but limits the number of concurrent processes to `max_concurrency`.
// fn collect_samples_concurrently(
//     manifest_infos: Vec<(String, String)>,
//     max_concurrency: usize,
// ) -> Result<Vec<Example>, Box<dyn Error>> {
//     let pool = ThreadPool::new(max_concurrency);
//     let (tx, rx) = std::sync::mpsc::channel();

//     // For each manifest info (prefix, manifest_path), spawn a task.
//     for (prefix, manifest_path) in manifest_infos {
//         let tx = tx.clone();
//         let prefix_clone = prefix.clone();
//         let manifest_clone = manifest_path.clone();
//         pool.execute(move || {
//             let mut results = Vec::new();
//             if let Ok(mut ex) = collect_examples(&prefix_clone, &manifest_clone) {
//                 results.append(&mut ex);
//             }
//             if let Ok(mut bins) = collect_binaries(&prefix_clone, &manifest_clone) {
//                 results.append(&mut bins);
//             }
//             tx.send(results).expect("Failed to send results");
//         });
//     }
//     drop(tx); // close channel so the receiver can finish
//     pool.join(); // wait for all tasks to finish

//     let mut all_samples = Vec::new();
//     for samples in rx {
//         all_samples.extend(samples);
//     }
//     Ok(all_samples)
// }

// --- Concurrent or sequential collection ---
pub fn collect_samples(
    manifest_infos: Vec<(String, PathBuf, bool)>,
    __max_concurrency: usize,
) -> Result<Vec<Example>, Box<dyn Error>> {
    let start_total = Instant::now();
    let mut all_samples = Vec::new();

    // "Before" message: starting collection
    println!("Timing: Starting sample collection...");

    #[cfg(feature = "concurrent")]
    {
        let pool = ThreadPool::new(__max_concurrency);
        let (tx, rx) = mpsc::channel();

        let start_concurrent = Instant::now();
        for (prefix, manifest_path, extended) in manifest_infos {
            let tx = tx.clone();
            let prefix_clone = prefix.clone();
            let manifest_clone = manifest_path.clone();
            pool.execute(move || {
                let mut results = Vec::new();
                if let Ok(mut ex) = collect_examples(&prefix_clone, &manifest_clone, extended) {
                    results.append(&mut ex);
                }
                if let Ok(mut bins) = collect_binaries(&prefix_clone, &manifest_clone, extended) {
                    results.append(&mut bins);
                }
                tx.send(results).expect("Failed to send results");
            });
        }
        drop(tx);
        pool.join(); // Wait for all tasks to finish.
        let duration_concurrent = start_concurrent.elapsed();
        println!(
            "Timing: Concurrent processing took {:?}",
            duration_concurrent
        );

        for samples in rx {
            all_samples.extend(samples);
        }
    }

    #[cfg(not(feature = "concurrent"))]
    {
        // Sequential fallback: process one manifest at a time.
        let start_seq = Instant::now();
        for (prefix, manifest_path) in manifest_infos {
            if let Ok(mut ex) = collect_examples(&prefix, &manifest_path) {
                all_samples.append(&mut ex);
            }
            if let Ok(mut bins) = collect_binaries(&prefix, &manifest_path) {
                all_samples.append(&mut bins);
            }
        }
        let duration_seq = start_seq.elapsed();
        println!("Timing: Sequential processing took {:?}", duration_seq);
    }

    let total_duration = start_total.elapsed();
    println!("Timing: Total collection time: {:?}", total_duration);
    Ok(all_samples)
}

pub fn run_example(example: &Example, extra_args: &[String]) -> Result<(), Box<dyn Error>> {
    // Build the base command.
    let mut cmd = Command::new("cargo");

    if example.extended {
        println!(
            "Running extended example in folder: examples/{}",
            example.name
        );
        // For extended samples, change directory to the sample's folder.
        cmd.arg("run")
            .current_dir(format!("examples/{}", example.name));
    } else {
        println!("Running: cargo run --release --example {}", example.name);
        cmd.args(&["run", "--release", "--example", &example.name]);
    }

    // If extra arguments are provided, append them after "--".
    if !extra_args.is_empty() {
        cmd.arg("--").args(extra_args);
    }

    // Spawn the process (instead of waiting with status())
    let child = cmd.spawn()?;

    // Wrap the child process so we can share it with our Ctrl+C handler.
    let child_arc = Arc::new(Mutex::new(child));
    let child_for_handler = Arc::clone(&child_arc);

    // Set up a Ctrl+C handler that kills the child process.
    ctrlc::set_handler(move || {
        eprintln!("Ctrl+C pressed, terminating process...");
        let mut child = child_for_handler.lock().unwrap();
        // Try to kill the process, ignore errors.
        let _ = child.kill();
    })?;

    // Wait for the process to complete.
    let status = child_arc.lock().unwrap().wait()?;
    println!("Process exited with status: {:?}", status.code());
    exit(status.code().unwrap_or(1));
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

/// Locate the Cargo.toml by invoking `cargo locate-project --message-format plain`.
/// If `workspace` is true, the `--workspace` flag is added so that the manifest
/// for the workspace root is returned.
// fn locate_manifest(workspace: bool) -> Result<String, Box<dyn Error>> {
//     // Build the arguments for cargo locate-project.
//     let mut args = vec!["locate-project", "--message-format", "plain"];
//     if workspace {
//         args.push("--workspace");
//     }

//     let output = Command::new("cargo").args(&args).output()?;

//     if !output.status.success() {
//         return Err("cargo locate-project failed".into());
//     }

//     let manifest = String::from_utf8_lossy(&output.stdout).trim().to_string();
//     if manifest.is_empty() {
//         return Err("No Cargo.toml found".into());
//     }
//     Ok(manifest)
// }

/// This function collects sample targets (examples and binaries) from both the current directory
/// and, if the --workspace flag is used, from each workspace member. The built–in samples (from
/// the current directory) are tagged with a "builtin" prefix, while workspace member samples are
/// tagged with "$member" so that the display name becomes "$member > example > sample_name" or
/// "$member > binary > sample_name".
pub fn collect_all_samples(
    use_workspace: bool,
    max_concurrency: usize,
) -> Result<Vec<Example>, Box<dyn Error>> {
    let mut manifest_infos: Vec<(String, PathBuf, bool)> = Vec::new();
    let cwd = env::current_dir()?;
    // Built-in samples: if there is a Cargo.toml in cwd, add it.
    let built_in_manifest = cwd.join("Cargo.toml");
    if built_in_manifest.exists() {
        println!(
            "Found built-in Cargo.toml in current directory: {}",
            cwd.display()
        );
        // For built-in samples, we use a fixed prefix.
        manifest_infos.push(("builtin".to_string(), built_in_manifest, false));
    } else {
        eprintln!("No Cargo.toml found in current directory for built-in samples.");
    }

    // If workspace flag is used, locate the workspace root and then collect all member manifests.
    if use_workspace {
        let ws_manifest = locate_manifest(true)?;
        println!("Workspace root manifest: {}", ws_manifest);
        let ws_members = collect_workspace_members(&ws_manifest)?;
        for (member_name, manifest_path) in ws_members {
            // The prefix for workspace samples is formatted as "$member_name"
            manifest_infos.push((format!("${}", member_name), manifest_path, false));
        }
    }

    // Also, extended samples: assume they live in an "examples" folder relative to cwd.
    let extended_root = cwd.join("examples");
    if extended_root.exists() {
        for entry in fs::read_dir(&extended_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("Cargo.toml").exists() {
                // Use the directory name as the prefix.
                let prefix = path.file_name().unwrap().to_string_lossy().to_string();
                let manifest_path = path.join("Cargo.toml");
                manifest_infos.push((prefix, manifest_path, true));
            }
        }
    } else {
        eprintln!(
            "Extended samples directory {:?} does not exist.",
            extended_root
        );
    }

    eprintln!("DEBUG: manifest infos: {:?}", manifest_infos);

    // Now, use either concurrent or sequential collection.
    // Here we assume a function similar to our earlier collect_samples_concurrently.
    // We reuse our previously defined collect_samples function, which now accepts a Vec<(String, PathBuf, bool)>.
    let samples = collect_samples(manifest_infos, max_concurrency)?;
    Ok(samples)
}
