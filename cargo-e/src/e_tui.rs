// src/e_tui.rs
#![allow(dead_code)]
//! This module implements the interactive terminal UI for cargo‑e.
//! The UI drawing logic is separated into its own submodule (`ui`).

#[cfg(feature = "tui")]
pub mod tui_interactive {
    use crate::{Cli, Example, TargetKind};
    use crate::e_bacon;
    use crate::e_findmain;
    use crate::e_manifest::{find_manifest_dir, maybe_patch_manifest_for_run};
    use crate::e_runner;
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind, poll, read},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Rect},
        Terminal,
    };
    use std::{collections::HashSet, fs, io, process::Command, mem, thread, time::Duration};

    /// The `ui` module contains functions for drawing the TUI.
    pub mod ui {
        use ratatui::{
            layout::{Constraint, Direction, Layout, Rect},
            style::{Color, Style},
            text::{Line, Span},
            widgets::{Block, Borders, List, ListItem, ListState},
            Frame,
        };
        use crate::Example;
        use std::collections::HashSet;

        /// Draws the entire UI on the provided frame.
        ///
        /// # Arguments
        ///
        /// * `f` - The frame to draw on.
        /// * `area` - The rectangular area available for drawing.
        /// * `examples` - A slice of examples to display.
        /// * `run_history` - A set of example names that have been run.
        /// * `list_state` - The state for the list widget.
        /// * `exit_hover` - Indicates whether the exit region is hovered.
        pub fn draw_ui(
            f: &mut Frame,
            area: Rect,
            examples: &[Example],
            run_history: &HashSet<String>,
            list_state: &mut ListState,
            exit_hover: bool,
        ) {

        // println!("Drawing UI...");
            // Use the layout API from ratatui.
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Min(0)].as_ref())
                .split(area);
            let list_area = chunks[0];

            let left_text = format!("Select example ({} examples found)", examples.len());
            let separator = " ┃ ";
            let right_text = "Esc or q to EXIT";
            // Use dereferencing (&*right_text) so that the string slice converts correctly.
            let title_line = if exit_hover {
                Line::from(vec![
                    Span::raw(&left_text),
                    Span::raw(separator),
                    Span::styled(&*right_text, Style::default().fg(Color::Yellow)),
                ])
            } else {
                Line::from(vec![
                    Span::raw(&left_text),
                    Span::raw(separator),
                    Span::styled("Esc or q to ", Style::default().fg(Color::White)),
                    Span::styled("EXIT", Style::default().fg(Color::Red)),
                ])
            };

            let block = Block::default().borders(Borders::ALL) .border_style(Style::default().fg(Color::White)).title(title_line);
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
            f.render_stateful_widget(list, list_area, list_state);
        }
    }

    /// Flush stray input events (e.g. stray Enter key presses).
    pub fn flush_input() -> Result<(), Box<dyn std::error::Error>> {
        while poll(Duration::from_millis(0))? {
            if let Event::Key(key_event) = read()? {
                if key_event.code == KeyCode::Enter {
                    continue;
                }
            }
        }
        Ok(())
    }

    /// Reinitializes the terminal by enabling raw mode, entering the alternate screen,
    /// enabling mouse capture, clearing the screen, and recreating the Terminal instance.
    pub fn reinit_terminal(
        terminal: &mut Terminal<CrosstermBackend<io::Stderr>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stderr();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            Clear(ClearType::All)
        )?;
        *terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(())
    }

    /// Launches the interactive TUI.
    pub fn launch_tui(cli: &Cli, examples: &[Example]) -> Result<(), Box<dyn std::error::Error>> {
        flush_input()?;
        let mut exs = examples.to_vec();
        if exs.is_empty() {
            println!("No examples found!");
            return Ok(());
        }
        exs.sort();

        // Load run history from the Cargo.toml directory.
        let manifest_dir = find_manifest_dir()?;
        let history_path = manifest_dir.join("run_history.txt");
        let mut run_history: HashSet<String> = HashSet::new();
        if let Ok(contents) = fs::read_to_string(&history_path) {
            for line in contents.lines() {
                if !line.trim().is_empty() {
                    run_history.insert(line.trim().to_string());
                }
            }
        }

        //  enable_raw_mode()?;
        //  let mut stdout = io::stdout();
        //  execute!(
        //      stdout,
        //      EnterAlternateScreen,
        //      EnableMouseCapture,
        //      Clear(ClearType::All)
        //  )?;
        // let backend = CrosstermBackend::new(stdout);
        // let mut terminal = Terminal::new(backend)?;
    'tui_loop: loop {
print!("\x1B[2J");
use std::io::{self, Write};
io::stdout().flush().ok();
        let mut terminal = Terminal::new(CrosstermBackend::new(io::BufWriter::new(std::io::stderr())))?;
        // let mut  terminal = ratatui::init();
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));
        let mut exit_hover = false;
        let mut run_glow = false;
        let mut terminal_area = terminal.size()?;

        'main_loop: loop {

            if run_glow {
                // Exit TUI mode to run `glow -p`
                break 'main_loop;
            }
            terminal.draw(|f| {
                let area = f.area();
                ui::draw_ui(f, area, examples, &run_history, &mut list_state, exit_hover);
            })?;

            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break 'tui_loop,
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
                                let page = terminal.size().map(|r| r.height.saturating_sub(4)).unwrap_or(5) as usize;
                                let current = list_state.selected().unwrap_or(0);
                                let new = std::cmp::min(current + page, exs.len() - 1);
                                list_state.select(Some(new));
                            }
                            KeyCode::PageUp => {
                                let page = terminal.size().map(|r| r.height.saturating_sub(4)).unwrap_or(5) as usize;
                                let current = list_state.selected().unwrap_or(0);
                                let new = current.saturating_sub(page);
                                list_state.select(Some(new));
                            }
                            KeyCode::Char('b') => {
                                if let Some(selected) = list_state.selected() {
                                    let sample = &examples[selected];
                                    if let Err(e) = e_bacon::run_bacon(sample, &Vec::new()) {
                                        eprintln!("Error running bacon: {}", e);
                                    } else {
                                        println!("Bacon launched for sample: {}", sample.name);
                                    }
                                    // reinit_terminal(&mut terminal)?;
                                }
                            }
                            KeyCode::Char('r') => {
                                // Exit TUI mode to run `glow -p`
                                // disable_raw_mode()?;
                                // execute!(
                                //     stdout,
                                //     LeaveAlternateScreen,
                                //     DisableMouseCapture,
                                //     Clear(ClearType::All)
                                // )?;
                                // terminal.show_cursor()?;
                                
                                    // reinit_terminal(&mut terminal)?;
                                // terminal.clear()?;
                                // drop(terminal);
        run_glow=true;
                break 'main_loop;
       // ratatui::restore();
        // disable_raw_mode()?;
        // disable_raw_mode()?;
        // execute!(
        //     terminal.backend_mut(),
        //     LeaveAlternateScreen,
        //     crossterm::event::DisableMouseCapture,
        //     Clear(ClearType::All)
        // )?;
        // terminal.show_cursor()?;
                                if let Err(e) = e_runner::run_glow(cli.workspace) {
                                    eprintln!("Failed to run glow: {}", e);
                                    thread::sleep(Duration::from_secs(5));
                                }
 //       terminal = ratatui::init();
        // thread::sleep(Duration::from_millis(50));
        // disable_raw_mode()?;
        // enable_raw_mode()?;
        // let mut stdout = io::stdout();
        // execute!(
        //     stdout,
        //     EnterAlternateScreen,
        //     EnableMouseCapture,
        //     Clear(ClearType::All)
        // )?;
        // let new_terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        // mem::replace(&mut terminal, new_terminal);
                                    // reinit_terminal(&mut terminal)?;
                                // enable_raw_mode()?;
                                // let mut new_stdout = io::stdout(); // declare as mutable
                                // execute!(
                                //     new_stdout,
                                //     EnterAlternateScreen,
                                //     EnableMouseCapture,
                                //     Clear(ClearType::All)
                                // )?;
                                // let new_terminal = Terminal::new(CrosstermBackend::new(new_stdout))?;
                                // mem::replace(&mut terminal, new_terminal);
                                // // Force an immediate redraw.
                                terminal.draw(|f| {
                                    let area = f.area();
                                    ui::draw_ui(f, area, examples, &run_history, &mut list_state, exit_hover);
                                })?;
                                terminal.flush()?;
                            }
                            KeyCode::Char('e') => {
                                if let Some(selected) = list_state.selected() {
                                    disable_raw_mode()?;
                                    execute!(
                                        io::stdout(),
                                        LeaveAlternateScreen,
                                        DisableMouseCapture,
                                        Clear(ClearType::All)
                                    )?;
                                    terminal.show_cursor()?;
                                    let sample = &examples[selected];
                                    println!("Opening VSCode for path: {}", sample.manifest_path);
                                    futures::executor::block_on(e_findmain::open_vscode_for_sample(sample));
                                    thread::sleep(Duration::from_secs(5));
                                    // reinit_terminal(&mut terminal)?;
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(selected) = list_state.selected() {
                                    run_piece(
                                        examples,
                                        selected,
                                        &history_path,
                                        &mut run_history,
                                        &mut terminal,
                                        cli.wait,
                                        cli.print_exit_code,
                                        cli.print_program_name,
                                        cli.print_instruction,
                                    )?;
                                }
                            }
                            _ => {}
                        }
                    }
                    Event::Mouse(mouse_event) => {
                        let size = terminal.size()?;
                        let area = Rect::new(0, 0, size.width, size.height);
                        let chunks = ratatui::layout::Layout::default()
                            .direction(ratatui::layout::Direction::Vertical)
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
                                    exit_hover = mouse_event.column >= right_region_start &&
                                                 mouse_event.column < right_region_end;
                                } else {
                                    exit_hover = false;
                                    let inner_y = list_area.y + 1;
                                    let inner_height = list_area.height.saturating_sub(2);
                                    if mouse_event.column > list_area.x + 1 &&
                                       mouse_event.column < list_area.x + list_area.width - 1 &&
                                       mouse_event.row >= inner_y &&
                                       mouse_event.row < inner_y + inner_height
                                    {
                                        let index = (mouse_event.row - inner_y) as usize;
                                        if index < exs.len() {
                                            list_state.select(Some(index));
                                        }
                                    }
                                }
                            }
                            MouseEventKind::Down(_) => {
                                if mouse_event.row == title_row &&
                                   mouse_event.column >= right_region_start &&
                                   mouse_event.column < right_region_end
                                {
                                    break 'main_loop;
                                }
                                let inner_y = list_area.y + 1;
                                let inner_height = list_area.height.saturating_sub(2);
                                if mouse_event.column > list_area.x + 1 &&
                                   mouse_event.column < list_area.x + list_area.width - 1 &&
                                   mouse_event.row >= inner_y &&
                                   mouse_event.row < inner_y + inner_height
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
                                            cli.wait,
                                            cli.print_exit_code,
                                            cli.print_program_name,
                                            cli.print_instruction,
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


        if run_glow {
            // ratatui::restore();
            if let Err(e) = e_runner::run_glow(cli.workspace) {
                eprintln!("Failed to run glow: {}", e);
                    #[cfg(target_os = "windows")]
    {
        eprintln!("install glow using Chocolatey: choco install glow");
    }
    #[cfg(target_os = "macos")]
    {
        eprintln!("install glow using Homebrew: brew install glow");
    }
                thread::sleep(Duration::from_secs(5));

            }
            // use std::io::{stderr, stdout};
        // let new_terminal = Terminal::new(CrosstermBackend::new(stderr()))?;
        // mem::replace(terminal, new_terminal);
            // terminal = ratatui::init();
            terminal = Terminal::new(CrosstermBackend::new(io::BufWriter::new(std::io::stderr())))?;

            terminal.clear()?;
            let rect_area = Rect::new(0, 0, terminal_area.width-1, terminal_area.height-1);
            terminal.resize(rect_area).ok();
            terminal.autoresize().ok();
            terminal.size().ok();
            terminal.draw(|f| {
                   let area = f.area();
                    f.render_widget(ratatui::widgets::Clear, area);
                   ui::draw_ui(f, area, examples, &run_history, &mut list_state, exit_hover);
             })?;
             terminal.flush()?;
            continue 'tui_loop; // "goto"-like behavior
        }
       // break 'tui_loop; // Exit outer loop if not run_glow
    }

        // disable_raw_mode()?;
        // let mut stdout = io::stdout();
        // execute!(
        //     stdout,
        //     LeaveAlternateScreen,
        //     DisableMouseCapture,
        //     Clear(ClearType::All)
        // )?;
        // terminal.show_cursor()?;
        Ok(())
    }

    /// Runs the specified example (or binary) target.
    pub fn run_piece(
        examples: &[Example],
        index: usize,
        history_path: &std::path::Path,
        run_history: &mut HashSet<String>,
        terminal: &mut Terminal<CrosstermBackend<io::BufWriter<io::Stderr>>>,
        wait_secs: u64,
        print_exit_code: bool,
        print_program_name: bool,
        print_instruction: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let target = &examples[index];
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
                if print_program_name {
                    println!("Running extended example with manifest: {}", manifest_path);
                }
                vec!["run", "--manifest-path", &manifest_path]
            } else {
                if print_program_name {
                    println!("Running example: cargo run --release --example {}", target.name);
                }
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
            if print_program_name {
                println!("Running binary: cargo run --release --bin {}", target.name);
            }
            vec![
                "run",
                "--manifest-path",
                &manifest_path,
                "--release",
                "--bin",
                &target.name,
            ]
        };

        let current_dir = if target.extended {
            std::path::Path::new(&manifest_path).parent().map(|p| p.to_owned())
        } else {
            None
        };

        let manifest_path_obj = std::path::Path::new(&manifest_path);
        let backup = maybe_patch_manifest_for_run(manifest_path_obj)?;

        let mut cmd = Command::new("cargo");
        cmd.args(&args);
        if let Some(ref dir) = current_dir {
            cmd.current_dir(dir);
        }

        let mut child = crate::e_runner::spawn_cargo_process(&args)?;
        if print_instruction {
            println!("Process started. Press Ctrl+C to terminate or 'd' to detach...");
        }
        let mut update_history = true;
        let status_code: i32;
        let mut detached = false;
        loop {
            if let Some(status) = child.try_wait()? {
                status_code = status.code().unwrap_or(1);
                break;
            }
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(event::KeyModifiers::CONTROL)
                    {
                        if print_instruction {
                            println!("Ctrl+C detected, killing process...");
                        }
                        child.kill()?;
                        update_history = false;
                        let status = child.wait()?;
                        status_code = status.code().unwrap_or(1);
                        break;
                    } else if key_event.code == KeyCode::Char('d') && key_event.modifiers.is_empty() {
                        if print_instruction {
                            println!("'d' pressed; detaching process. Process will continue running.");
                        }
                        detached = true;
                        update_history = false;
                        status_code = 0;
                        break;
                    }
                }
            }
        }
        if let Some(original) = backup {
            fs::write(manifest_path_obj, original)?;
        }
        if !detached {
            if update_history && status_code == 0 && run_history.insert(target.name.clone()) {
                let history_data = run_history.iter().cloned().collect::<Vec<_>>().join("\n");
                fs::write(history_path, history_data)?;
            }
            if !print_exit_code {
                println!("Exitcode {}  Waiting for {} seconds...", status_code, wait_secs);
            }
            thread::sleep(Duration::from_secs(wait_secs));
        }
        while event::poll(Duration::from_millis(0))? {
            let _ = event::read()?;
        }
        thread::sleep(Duration::from_millis(50));
        enable_raw_mode()?;
        let mut stdout = io::stderr();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            Clear(ClearType::All)
        )?;
        let new_terminal = Terminal::new(CrosstermBackend::new(io::BufWriter::new(stdout)))?;
        mem::replace(terminal, new_terminal);
        Ok(())
    }
}

