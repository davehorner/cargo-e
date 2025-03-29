#[cfg(feature = "tui")]
pub mod tui_interactive {
    use crate::e_command_builder::CargoCommandBuilder;
    use crate::e_manifest::maybe_patch_manifest_for_run;
    use crate::e_prompts::prompt_line;
    use crate::e_target::CargoTarget;
    use crate::prelude::*;
    use crate::{e_bacon, e_findmain, Cli};
    use crossterm::event::KeyEventKind;
    use crossterm::event::{poll, read};
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
                continue;
            }
        }
        Ok(())
    }

    /// Try to collect an escape sequence if the first event is Esc.
    /// Returns Some(arrow) if the sequence matches an arrow key, otherwise None.
    fn try_collect_arrow_sequence() -> Result<Option<KeyCode>, Box<dyn std::error::Error>> {
        // Buffer to hold the sequence. We already know the first event is Esc.
        let mut sequence = vec![];
        let start = Instant::now();
        // Give a short window (e.g. 50 ms) to collect additional events.
        while start.elapsed() < Duration::from_millis(50) {
            if poll(Duration::from_millis(0))? {
                if let Event::Key(key) = read()? {
                    // Only consider Press events.
                    if key.kind == KeyEventKind::Press {
                        sequence.push(key);
                    }
                }
            }
        }
        // Now, an arrow key should have a sequence like: Esc, '[' and then 'A' (or 'B', 'C', 'D').
        if sequence.len() >= 2 {
            if sequence[0].code == KeyCode::Char('[') {
                // Check the third element if available.
                if let Some(third) = sequence.get(1) {
                    // Compare the character case-insensitively (to handle unexpected modifiers).
                    if let KeyCode::Char(ch) = third.code {
                        let ch = ch.to_ascii_uppercase();
                        return Ok(match ch {
                            'A' => Some(KeyCode::Up),
                            'B' => Some(KeyCode::Down),
                            'C' => Some(KeyCode::Right),
                            'D' => Some(KeyCode::Left),
                            _ => None,
                        });
                    }
                }
            }
        }
        Ok(None)
    }

    /// Launches an interactive terminal UI for selecting an example.
    pub fn launch_tui(
        cli: &Cli,
        examples: &[CargoTarget],
    ) -> Result<(), Box<dyn std::error::Error>> {
        flush_input()?; // Clear any buffered input (like stray Return keys)
        let mut exs = examples.to_vec();
        if exs.is_empty() {
            println!("No examples found!");
            return Ok(());
        }
        exs.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        // Determine the directory containing the Cargo.toml at runtime.
        let manifest_dir = crate::e_manifest::find_manifest_dir()?;
        let history_path = manifest_dir.join("run_history.txt");
        let mut run_history: HashSet<String> = HashSet::new();
        if let Ok(contents) = fs::read_to_string(&history_path) {
            for line in contents.lines() {
                if !line.trim().is_empty() {
                    run_history.insert(line.trim().to_string());
                }
            }
        }

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
                let right_text = "q to EXIT";
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
                        Span::styled("q to ", Style::default().fg(Color::White)),
                        Span::styled("EXIT", Style::default().fg(Color::Red)),
                    ])
                };

                let block = Block::default().borders(Borders::ALL).title(title_line);
                let items: Vec<ListItem> = exs
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
                        if key.kind == KeyEventKind::Press {
                            // Check if we might be starting an escape sequence for an arrow key.
                            if key.code == KeyCode::Esc {
                                // Try to collect the rest of the sequence.
                                if let Some(arrow_code) = try_collect_arrow_sequence()? {
                                    match arrow_code {
                                        KeyCode::Up => {
                                            let new_index = match list_state.selected() {
                                                Some(0) | None => 0,
                                                Some(i) => i.saturating_sub(1),
                                            };
                                            list_state.select(Some(new_index));
                                        }
                                        KeyCode::Down => {
                                            let new_index = match list_state.selected() {
                                                Some(i) if i >= exs.len() - 1 => i,
                                                Some(i) => i + 1,
                                                None => 0,
                                            };
                                            list_state.select(Some(new_index));
                                        }
                                        KeyCode::Left => {
                                            // Handle left arrow if needed.
                                        }
                                        KeyCode::Right => {
                                            // Handle right arrow if needed.
                                        }
                                        _ => {}
                                    }
                                    // We've handled the arrow, so skip further processing.
                                    continue;
                                } else {
                                    // No follow-up sequence—treat it as a standalone Esc if needed.
                                    // For example, you might decide not to exit on Esc now.
                                    // println!("Standalone Esc detected (ignoring).");
                                    continue;
                                }
                            }
                            match key.code {
                                KeyCode::Char('q') => {
                                    // Exit the TUI mode when 'q' is pressed.
                                    println!("Exiting TUI mode...");
                                    break 'main_loop;
                                }
                                KeyCode::Down => {
                                    let i = match list_state.selected() {
                                        Some(i) if i >= exs.len() - 1 => i,
                                        Some(i) => i + 1,
                                        None => 0,
                                    };
                                    list_state.select(Some(i));
                                    thread::sleep(Duration::from_millis(50));
                                }
                                KeyCode::Up => {
                                    let i = match list_state.selected() {
                                        Some(0) | None => 0,
                                        Some(i) => i - 1,
                                    };
                                    list_state.select(Some(i));
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
                                    let new = current.saturating_sub(page);
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
                                        let sample = &exs[selected];
                                        println!(
                                            "Opening VSCode for path: {}",
                                            sample
                                                .manifest_path
                                                .to_str()
                                                .unwrap_or_default()
                                                .to_owned()
                                        );
                                        // Here we block on the asynchronous open_vscode call.
                                        // futures::executor::block_on(open_vscode(Path::new(&sample.manifest_path)));
                                        futures::executor::block_on(
                                            e_findmain::open_vscode_for_sample(sample),
                                        );
                                        std::thread::sleep(std::time::Duration::from_secs(5));
                                        reinit_terminal(&mut terminal)?;
                                    }
                                }
                                KeyCode::Char('i') => {
                                    if let Some(selected) = list_state.selected() {
                                        // Disable raw mode for debug printing.
                                        crossterm::terminal::disable_raw_mode()?;
                                        crossterm::execute!(
                                            std::io::stdout(),
                                            crossterm::terminal::LeaveAlternateScreen
                                        )?;
                                        let target = &exs[selected];
                                        println!("Target: {:?}", target);
                                        futures::executor::block_on(
                                            crate::e_runner::open_ai_summarize_for_target(target),
                                        );
                                        prompt_line("", 120).ok();
                                        reinit_terminal(&mut terminal)?;
                                    }
                                }
                                // KeyCode::Char('v') => {
                                //     if let Some(selected) = list_state.selected() {
                                //         // Disable raw mode for debug printing.
                                //         crossterm::terminal::disable_raw_mode()?;
                                //         crossterm::execute!(
                                //             std::io::stdout(),
                                //             crossterm::terminal::LeaveAlternateScreen
                                //         )?;
                                //         // When 'e' is pressed, attempt to open the sample in VSCode.
                                //         let sample = &examples[selected];
                                //         println!("Opening VIM for path: {}", sample.manifest_path);
                                //         // Here we block on the asynchronous open_vscode call.
                                //         // futures::executor::block_on(open_vscode(Path::new(&sample.manifest_path)));
                                //         e_findmain::open_vim_for_sample(sample);
                                //         std::thread::sleep(std::time::Duration::from_secs(5));
                                //         reinit_terminal(&mut terminal)?;
                                //     }
                                // }
                                KeyCode::Enter => {
                                    if let Some(selected) = list_state.selected() {
                                        run_piece(
                                            &exs,
                                            selected,
                                            &history_path,
                                            &mut run_history,
                                            &mut terminal,
                                            cli,
                                        )?;
                                        reinit_terminal(&mut terminal)?;
                                    }
                                }
                                _ => {
                                    //println!("Unhandled key event: {:?}", key.code);
                                }
                            }
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
                        let right_text = "q to EXIT";
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
                                    if mouse_event.column > list_area.x + 1
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
                                    println!("Exiting TUI mode...");
                                    break 'main_loop;
                                }
                                let inner_y = list_area.y + 1;
                                let inner_height = list_area.height.saturating_sub(2);
                                if mouse_event.column > list_area.x + 1
                                    && mouse_event.column < list_area.x + list_area.width - 1
                                    && mouse_event.row >= inner_y
                                    && mouse_event.row < inner_y + inner_height
                                {
                                    let index = (mouse_event.row - inner_y) as usize;
                                    if index < exs.len() {
                                        list_state.select(Some(index));
                                        run_piece(
                                            &exs.clone(),
                                            index,
                                            &history_path,
                                            &mut run_history,
                                            &mut terminal,
                                            cli,
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
        flush_input()?; // Clear any buffered input after reinitializing the terminal.
        Ok(())
    }

    /// Runs the given example (or binary) target. It leaves TUI mode, spawns a cargo process,
    /// installs a Ctrl+C handler to kill the process, waits for it to finish, updates history,
    /// flushes stray input, and then reinitializes the terminal.
    pub fn run_piece(
        examples: &[CargoTarget],
        index: usize,
        history_path: &Path,
        run_history: &mut HashSet<String>,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        cli: &Cli,
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

        //let manifest_path = target.manifest_path.clone();
        let manifest_path = PathBuf::from(target.manifest_path.clone());

        // let mut args: Vec<String> = if target.kind == TargetKind::Example {
        //     if target.extended {
        //         if cli.print_program_name {
        //             println!("Running extended example with manifest: {}", manifest_path);
        //         }
        //         // For workspace extended examples, assume the current directory is set correctly.
        //         vec![
        //             "run".to_string(),
        //             "--manifest-path".to_string(),
        //             manifest_path.to_owned(),
        //         ]
        //     } else {
        //         if cli.print_program_name {
        //             println!(
        //                 "Running example: cargo run --release --example {}",
        //                 target.name
        //             );
        //         }
        //         vec![
        //             "run".to_string(),
        //             "--manifest-path".to_string(),
        //             manifest_path.to_owned(),
        //             "--release".to_string(),
        //             "--example".to_string(),
        //             format!("{}", target.name),
        //         ]
        //     }
        // } else {
        //     if cli.print_program_name {
        //         println!("Running binary: cargo run --release --bin {}", target.name);
        //     }
        //     vec![
        //         "run".to_string(),
        //         "--manifest-path".to_string(),
        //         manifest_path.to_owned(),
        //         "--release".to_string(),
        //         "--bin".to_string(),
        //         format!("{}", target.name),
        //     ]
        // };

        let builder = CargoCommandBuilder::new()
            .with_target(target)
            .with_required_features(&manifest_path, target)
            .with_cli(cli);
        let mut cmd = builder.build_command();

        // Set current directory appropriately.
        // if target.kind == TargetKind::ManifestTauri {
        //     let manifest_dir = manifest_path.parent().expect("Expected parent directory");
        //     cmd.current_dir(manifest_dir);
        // } else if target.extended {
        //     if let Some(dir) = manifest_path.parent() {
        //         cmd.current_dir(dir);
        //     }
        // }

        println!("Running command: {:?}", cmd);
        // If the target is extended, we want to run it from its directory.
        if target.extended {
            Path::new(&manifest_path).parent().map(|p| p.to_owned())
        } else {
            None
        };

        // Before spawning, patch the manifest if needed.
        let manifest_path_obj = Path::new(&manifest_path);
        let backup = maybe_patch_manifest_for_run(manifest_path_obj)?;

        // // // Build the command.
        // // let mut cmd = Command::new("cargo");
        // // cmd.args(&args);
        // // if let Some(ref dir) = current_dir {
        // //     cmd.current_dir(dir);
        // // }
        // // Convert command args into &str slices for spawn_cargo_process.
        // // (Assuming spawn_cargo_process accepts a slice of &str.)
        // let owned_args: Vec<String> = cmd
        //     .get_args()
        //     .map(|arg| arg.to_string_lossy().to_string())
        //     .collect();
        // // Now create a vector of &str references valid as long as `owned_args` is in scope:
        // let args_ref: Vec<&str> = owned_args.iter().map(|s| s.as_str()).collect();

        // // let args_ref: Vec<&str> = args.iter().map(|s| &**s).collect();
        // let mut child = crate::e_runner::spawn_cargo_process(&args_ref)?;

        let mut child = cmd.spawn()?;
        if cli.print_instruction {
            println!("Process started. Press Ctrl+C to terminate or 'd' to detach...");
        }
        let mut update_history = true;
        let status_code: i32;
        let mut detached = false;
        // Now we enter an event loop, periodically checking if the child has exited
        // and polling for keyboard input.
        loop {
            // // Check if the child process has finished.
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
                        if cli.print_instruction {
                            println!("Ctrl+C detected in event loop, killing process...");
                        }
                        child.kill()?;
                        update_history = false; // do not update history if cancelled
                                                // Optionally, you can also wait for the child after killing.
                        let status = child.wait()?;
                        status_code = status.code().unwrap_or(1);
                        break;
                    } else if key_event.code == KeyCode::Char('d') && key_event.modifiers.is_empty()
                    {
                        if cli.print_instruction {
                            println!(
                                "'d' pressed; detaching process. Process will continue running."
                            );
                        }
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
        // Restore the manifest if it was patched.
        if let Some(original) = backup {
            fs::write(manifest_path_obj, original)?;
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
            if update_history && status_code == 0 && run_history.insert(target.name.clone()) {
                let history_data = run_history.iter().cloned().collect::<Vec<_>>().join("\n");
                fs::write(history_path, history_data)?;
            }
            let message = if cli.print_exit_code {
                format!("Exitcode {:?}. Press any key to continue...", status_code)
            } else {
                "".to_string()
            };
            let _ = crate::e_prompts::prompt(&message, cli.wait)?;
        }

        reinit_terminal(terminal)?; // Reinitialize the terminal after running the target.

        // // Flush stray input events.
        // while event::poll(std::time::Duration::from_millis(0))? {
        //     let _ = event::read()?;
        // }
        // std::thread::sleep(std::time::Duration::from_millis(50));

        // // // Reinitialize the terminal.
        // enable_raw_mode()?;
        // let mut stdout = io::stdout();
        // execute!(
        //     stdout,
        //     EnterAlternateScreen,
        //     crossterm::event::EnableMouseCapture,
        //     Clear(ClearType::All)
        // )?;
        // *terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(())
    }
}
