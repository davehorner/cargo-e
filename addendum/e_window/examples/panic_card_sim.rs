// Example: Simulate sending a panic card to e_window via stdin
// This demonstrates how to send a custom card and NOT include the default card

use std::io::Write;
use std::process::{Command, Stdio};

fn main() {
    // Simulated panic info
    let target_name = "my_binary";
    let cargo_e_version = env!("CARGO_PKG_VERSION");
    let line = "thread 'main' panicked at 'something went wrong', src/main.rs:42:9";
    let prior_message = Some("Additional context: failed to connect to database.");

    // Compose the panic card
    let mut card = format!(
        "--title \"panic: {target}\" --width 400 --height 300\n\
        target | {target} | string\n\
        cargo-e | {version} | string\n\
        \n\
        Panic detected in {target}\n{line}",
        target = target_name,
        version = cargo_e_version,
        line = line
    );
    if let Some(msg) = prior_message {
        card = format!("{}\n{}", card, msg);
    }

    // Find e_window in PATH
    let e_window_path = if let Ok(path) = which::which("e_window") {
        path
    } else {
        eprintln!("e_window not found in PATH");
        return;
    };

    // Start e_window and send the card
    let mut child = Command::new(e_window_path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start e_window");

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(card.as_bytes())
            .expect("Failed to write card");
        stdin.flush().expect("Failed to flush");
    }

    child.wait().expect("Child process wasn't running");
}
