#![allow(unused_variables)]
#[cfg(feature = "tui")]
use crossterm::{
    event::{poll, read, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::error::Error;
use std::time::Duration;

/// Prompts the user with the given message and waits up to `wait_secs` seconds
/// for a key press. Returns `Ok(Some(c))` if a key is pressed, or `Ok(None)`
/// if the timeout expires.
pub fn prompt(message: &str, wait_secs: u64) -> Result<Option<char>, Box<dyn Error>> {
    if !message.trim().is_empty() {
        println!("{}", message);
    }
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        println!("Non-interactive mode detected; skipping prompt.");
        return Ok(None);
    }

    // When the "tui" feature is enabled, use raw mode.
    #[cfg(feature = "tui")]
    {
        let timeout = Duration::from_secs(wait_secs);
        // Clear any pending events.
        while poll(Duration::from_millis(0))? {
            let _ = read()?;
        }
        enable_raw_mode()?;
        let result = if poll(timeout)? {
            if let Event::Key(key_event) = read()? {
                if let KeyCode::Char(c) = key_event.code {
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
        disable_raw_mode()?;
        Ok(result)
    }

    // Otherwise, use normal line input.
    #[cfg(not(feature = "tui"))]
    {
        use std::io::{self, BufRead, Write};
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
        println!("Non-interactive mode detected; skipping prompt.");
        return Ok(None);
    }

    #[cfg(not(feature = "tui"))]
    {
        use std::io::{self, BufRead, Write};
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
        use crossterm::{
            event::{poll, read, Event, KeyCode},
            terminal::{disable_raw_mode, enable_raw_mode},
        };
        use std::io::{self, Write};
        enable_raw_mode()?;
        let mut input = String::new();
        let start = std::time::Instant::now();
        loop {
            let elapsed = start.elapsed().as_secs();
            if elapsed >= wait_secs {
                disable_raw_mode()?;
                return Ok(None);
            }
            let remaining = Duration::from_secs(wait_secs - elapsed);
            if poll(remaining)? {
                if let Event::Key(key_event) = read()? {
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
        disable_raw_mode()?;
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
        enable_raw_mode()?;
        let timeout = Duration::from_secs(wait_secs);
        let start = std::time::Instant::now();
        let mut input = String::new();
        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                #[cfg(feature = "tui")]
                disable_raw_mode()?;
                return Ok(None);
            }
            // Poll for an event for the remaining time.
            let remaining = timeout - elapsed;
            if poll(remaining)? {
                if let Event::Key(crossterm::event::KeyEvent { code, .. }) = read()? {
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
        disable_raw_mode()?;
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
) -> Result<Option<String>, Box<dyn Error>> {
    #[cfg(feature = "tui")]
    {
        let timeout = Duration::from_secs(wait_secs);
        enable_raw_mode()?;
        let start = std::time::Instant::now();
        let mut input = String::new();

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                disable_raw_mode()?;
                return Ok(None);
            }
            let remaining = timeout - elapsed;
            if poll(remaining)? {
                if let Event::Key(crossterm::event::KeyEvent { code, .. }) = read()? {
                    match code {
                        KeyCode::Enter => break, // End input on Enter.
                        KeyCode::Char(c) => {
                            // Check quick exit keys (case-insensitive)
                            if quick_exit.iter().any(|&qe| qe.eq_ignore_ascii_case(&c)) {
                                disable_raw_mode()?;
                                input.push(c);
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
                            print!("{}", c);
                            io::Write::flush(&mut io::stdout())?;
                        }
                        KeyCode::Backspace => {
                            input.pop();
                            // Clear and reprint the input (a simple approach).
                            print!("\r{}{}\r", " ".repeat(input.len() + 2), input);
                            io::Write::flush(&mut io::stdout())?;
                        }
                        _ => {} // Ignore other keys.
                    }
                }
            }
        }
        disable_raw_mode()?;
        println!();
        Ok(Some(input.trim().to_string()))
    }

    #[cfg(not(feature = "tui"))]
    {
        return Ok(read_line_with_timeout(wait_secs).unwrap_or_default());
    }
}
