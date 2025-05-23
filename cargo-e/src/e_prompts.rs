#![allow(unused_variables)]
use anyhow::Result;
#[cfg(feature = "tui")]
use crossterm::event::{poll, read, Event, KeyCode};
use std::error::Error;
use std::time::Duration;

/// A RAII guard that enables raw mode and disables it when dropped.
#[allow(dead_code)]
struct RawModeGuard;

impl RawModeGuard {
    #[allow(dead_code)]
    fn new() -> Result<Self> {
        #[cfg(feature = "tui")]
        crossterm::terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        #[cfg(feature = "tui")]
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Prompts the user with the given message and waits up to `wait_secs` seconds
/// for a key press. Returns `Ok(Some(c))` if a key is pressed, or `Ok(None)`
/// if the timeout expires.
pub fn prompt(message: &str, wait_secs: u64) -> Result<Option<char>> {
    if !message.trim().is_empty() {
        println!("{}", message);
    }
    use std::io::IsTerminal;
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        // println!("Non-interactive mode detected; skipping prompt.");
        return Ok(None);
    }

    // When the "tui" feature is enabled, use raw mode.
    #[cfg(feature = "tui")]
    {
        let timeout = Duration::from_secs(wait_secs);
        drain_events().ok(); // Clear any pending events.
                             // Enable raw mode and ensure it will be disabled when the guard is dropped.
        let _raw_guard = RawModeGuard::new()?;
        let result = if poll(timeout)? {
            if let Event::Key(key_event) = read()? {
                if let KeyCode::Char(c) = key_event.code {
                    // Check if it's the Ctrl+C character, which is often '\x03'
                    if c == '\x03' {
                        // Ctrl+C
                        return Err(anyhow::anyhow!("Ctrl+C pressed").into()); // Propagate as error to handle
                    }
                    Some(c.to_ascii_lowercase())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        Ok(result)
    }

    // Otherwise, use normal line input.
    #[cfg(not(feature = "tui"))]
    {
        use std::io::{self, BufRead};
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let stdin = io::stdin();
            let mut line = String::new();
            // This call will block until input is received.
            let _ = stdin.lock().read_line(&mut line);
            let _ = tx.send(line);
        });

        match rx.recv_timeout(Duration::from_secs(wait_secs)) {
            Ok(line) => Ok(line.trim().chars().next().map(|c| c.to_ascii_lowercase())),
            Err(_) => Ok(None),
        }
    }
}

/// Reads an entire line from the user with a timeout of `wait_secs`.
/// Returns Ok(Some(String)) if input is received, or Ok(None) if the timeout expires.
pub fn prompt_line(message: &str, wait_secs: u64) -> Result<Option<String>, Box<dyn Error>> {
    if !message.trim().is_empty() {
        println!("{}", message);
    }
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        // println!("Non-interactive mode detected; skipping prompt.");
        return Ok(None);
    }

    #[cfg(not(feature = "tui"))]
    {
        use std::io::{self, BufRead};
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let stdin = io::stdin();
            let mut line = String::new();
            // This call will block until input is received.
            let _ = stdin.lock().read_line(&mut line);
            let _ = tx.send(line);
        });

        match rx.recv_timeout(Duration::from_secs(wait_secs)) {
            Ok(line) => Ok(Some(line.trim().to_string())),
            Err(_) => Ok(None),
        }
    }

    #[cfg(feature = "tui")]
    {
        // In TUI raw mode, we collect key events until Enter is pressed.
        use crossterm::event::{poll, read, Event, KeyCode};
        use std::io::{self, Write};
        // Enable raw mode and ensure it will be disabled when the guard is dropped.
        #[cfg(feature = "tui")]
        let _raw_guard = RawModeGuard::new()?;
        let mut input = String::new();
        let start = std::time::Instant::now();
        loop {
            let elapsed = start.elapsed().as_secs();
            if elapsed >= wait_secs {
                println!("Timeout reached; no input received.");
                return Ok(None);
            }
            let remaining = Duration::from_secs(wait_secs - elapsed);
            if poll(remaining)? {
                if let Event::Key(key_event) = read()? {
                    // Only process key press events
                    if key_event.kind != crossterm::event::KeyEventKind::Press {
                        continue;
                    }
                    match key_event.code {
                        KeyCode::Enter => break,
                        KeyCode::Char(c) => {
                            input.push(c);
                            print!("{}", c);
                            use std::io::{self, Write};
                            io::stdout().flush()?;
                        }
                        KeyCode::Backspace => {
                            input.pop();
                            print!("\r{} \r", input);
                            io::stdout().flush()?;
                        }
                        _ => {}
                    }
                }
            }
        }
        println!();
        Ok(Some(input.trim().to_string()))
    }
}

use std::io::{self, BufRead};
use std::sync::mpsc;
use std::thread;

/// Reads a line from standard input with a timeout.
/// Returns Ok(Some(String)) if a line is read before the timeout,
/// Ok(None) if the timeout expires, or an error.
pub fn read_line_with_timeout(wait_secs: u64) -> io::Result<Option<String>> {
    let timeout = Duration::from_secs(wait_secs);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut line = String::new();
        let _ = stdin.lock().read_line(&mut line);
        let _ = tx.send(line);
    });
    match rx.recv_timeout(timeout) {
        Ok(line) => Ok(Some(line.trim().to_string())),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
    }
}

/// Prompts the user for a full line of input using crossterm events with a timeout.
/// This works cross-platform and avoids leftover input from a lingering blocking thread.
pub fn prompt_line_with_poll(wait_secs: u64) -> Result<Option<String>, Box<dyn std::error::Error>> {
    #[cfg(feature = "tui")]
    {
        // Enable raw mode and ensure it will be disabled when the guard is dropped.
        #[cfg(feature = "tui")]
        let _raw_guard = RawModeGuard::new()?;
        let timeout = Duration::from_secs(wait_secs);
        let start = std::time::Instant::now();
        let mut input = String::new();
        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Ok(None);
            }
            // Poll for an event for the remaining time.
            let remaining = timeout - elapsed;
            if poll(remaining)? {
                if let Event::Key(crossterm::event::KeyEvent { code, kind, .. }) = read()? {
                    // Only process key press events
                    if kind != crossterm::event::KeyEventKind::Press {
                        continue;
                    }
                    match code {
                        KeyCode::Enter => break,
                        KeyCode::Char(c) => {
                            input.push(c);
                            // Optionally echo the character (if desired).
                            print!("{}", c);
                            io::Write::flush(&mut io::stdout())?;
                        }
                        KeyCode::Backspace => {
                            input.pop();
                            // Optionally update the display.
                            print!("\r{}\r", " ".repeat(input.len() + 1));
                            print!("{}", input);
                            io::Write::flush(&mut io::stdout())?;
                        }
                        _ => {} // Ignore other keys.
                    }
                }
            }
        }
        Ok(Some(input))
    }
    #[cfg(not(feature = "tui"))]
    {
        return Ok(read_line_with_timeout(wait_secs).unwrap_or_default());
    }
}

/// Prompts the user for a full line of input using crossterm events (raw mode) with a timeout.
///
/// - `wait_secs`: seconds to wait for input.
/// - `quick_exit`: a slice of characters that, if pressed, immediately cause the function to return that key (as a string).
/// - `allowed_chars`: an optional slice of allowed characters (case-insensitive). If provided, only these characters are accepted.
///
/// In this example, we use numeric digits as allowed characters.
pub fn prompt_line_with_poll_opts(
    wait_secs: u64,
    quick_exit: &[char],
    allowed_chars: Option<&[char]>,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    #[cfg(feature = "tui")]
    {
        let timeout = Duration::from_secs(wait_secs);
        let _raw_guard = RawModeGuard::new()?;
        let start = std::time::Instant::now();
        let mut input = String::new();

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Ok(None);
            }
            let remaining = timeout - elapsed;
            if poll(remaining)? {
                if let Event::Key(crossterm::event::KeyEvent { code, kind, .. }) = read()? {
                    // Only process key press events
                    if kind != crossterm::event::KeyEventKind::Press {
                        continue;
                    }
                    match code {
                        KeyCode::Enter => {
                            drain_events().ok();
                            break;
                        } // End input on Enter.
                        KeyCode::Char(c) => {
                            // Check quick exit keys (case-insensitive)
                            if quick_exit.iter().any(|&qe| qe.eq_ignore_ascii_case(&c)) {
                                input.push(c);
                                drain_events().ok(); // Clear any pending events.
                                return Ok(Some(input.to_string()));
                            }
                            // If allowed_chars is provided, only accept those.
                            if let Some(allowed) = allowed_chars {
                                if !allowed.iter().any(|&a| a.eq_ignore_ascii_case(&c)) {
                                    // Ignore characters that aren't allowed.
                                    continue;
                                }
                            }
                            input.push(c);
                            use crossterm::{
                                cursor::MoveToColumn,
                                execute,
                                terminal::{Clear, ClearType},
                            };
                            execute!(io::stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0))?;
                            print!("{}", input);
                            io::Write::flush(&mut io::stdout())?;
                        }
                        KeyCode::Backspace => {
                            if !input.is_empty() {
                                input.pop();
                                // Move cursor back one, overwrite with space, and move back again.
                                print!("\x08 \x08");
                                io::Write::flush(&mut io::stdout())?;
                            }
                        }
                        _ => {} // Ignore other keys.
                    }
                }
            }
        }
        // println!();
        Ok(Some(input.trim().to_string()))
    }

    #[cfg(not(feature = "tui"))]
    {
        return Ok(read_line_with_timeout(wait_secs).unwrap_or_default());
    }
}

/// Drain *all* pending input events so that the next `poll` really waits
#[allow(dead_code)]
fn drain_events() -> Result<()> {
    // keep pulling until there’s nothing left
    #[cfg(feature = "tui")]
    while poll(Duration::from_millis(0))? {
        let _ = read()?;
    }
    Ok(())
}

/// Prompts the user with a yes/no question and returns true if the user answers yes, false if no,
/// or None if the timeout expires or an error occurs.
pub fn yesno(prompt_message: &str, default: Option<bool>) -> Result<Option<bool>> {
    let prompt_with_default = match default {
        Some(true) => format!("{} (Y/n)? ", prompt_message),
        Some(false) => format!("{} (y/N)? ", prompt_message),
        None => format!("{} (y/n)? ", prompt_message),
    };

    let result = match prompt(&prompt_with_default, 10)? {
        Some(c) => match c {
            'y' => Some(true),
            'n' => Some(false),
            _ => {
                // Handle invalid input (e.g., by re-prompting or returning None)
                println!("Invalid input. Please enter 'y' or 'n'.");
                return Ok(None); // Or potentially re-prompt here.
            }
        },
        None => {
            // Timeout occurred
            match default {
                Some(value) => Some(value),
                None => None,
            }
        }
    };
    Ok(result)
}
