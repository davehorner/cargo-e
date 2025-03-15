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
    println!("{}", message);
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        println!("Non-interactive mode detected; skipping prompt.");
        return Ok(None);
    }

    // When the "tui" feature is enabled, use raw mode.
    #[cfg(feature = "tui")]
    {
        let timeout = Duration::from_secs(wait_secs);
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
        use std::error::Error;
        use std::io::{self, BufRead, IsTerminal, Write};
        use std::sync::mpsc;
        use std::thread;
        print!("Enter choice: ");
        io::stdout().flush()?;

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
