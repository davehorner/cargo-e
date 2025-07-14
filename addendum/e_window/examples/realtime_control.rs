// Example: Send control commands to a running e_window instance via stdin
// Run e_window, then pipe commands to its stdin from this example

use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub const DEFAULT_CARD_TEMPLATE: &str = r#"--title \"Demo: realtime_control\" --width 1024 --height 768 --x 200 --y 200
name | realtime_control | string
version | 1.0 | string
author | Dave Horner | string

Welcome to *REALTIME CONTROL* e_window!
This demo shows how you can send live control commands to a running e_window instance via its stdin.

How this works:
- This Rust program launches e_window as a child process and pipes commands to its stdin.
- The initial card template is sent first, setting up the window and its fields.
- Then, a sequence of control commands is sent with timed delays, demonstrating dynamic updates:
    - Window title and position changes using `!control:` commands.
    - Animated movement and resizing of the window with `set_rect_eased`.
    - Swapping card content in real time by sending new document data.
    - Each card (Top Left, Top Right, Bottom Right, Bottom Left, Center) is shown in turn.
    - The window closes automatically at the end via `!control: exit`.

How to use:
- Run e_window, then run this demo to see the window respond to commands.
- You can modify the commands or card templates to experiment with different behaviors.
- The control protocol lets you automate window management, content updates, and more.

Try editing the card templates or control commands below, then re-run to see your changes in action!
anchor: Click me! | e_window --title "you clicked!" --width 800 --height 600 --x 100 --y 100
"#;
pub const CARD_TOP_LEFT: &str = "--title \"Top Left\" --width 400 --height 300 --x 0 --y 0\n\nTop Left Demo\nWindow animates to the top-left corner\nDemonstrates position control\nBody for Top Left";
pub const CARD_TOP_RIGHT: &str = "--title \"Top Right\" --width 400 --height 300 --x 1520 --y 0\n\nTop Right Demo\nWindow animates to the top-right corner\nShows dynamic movement\nBody for Top Right";
pub const CARD_BOTTOM_RIGHT: &str = "--title \"Bottom Right\" --width 400 --height 300 --x 1520 --y 780\n\nBottom Right Demo\nWindow animates to the bottom-right corner\nIllustrates resizing and placement\nBody for Bottom Right";
pub const CARD_BOTTOM_LEFT: &str = "--title \"Bottom Left\" --width 400 --height 300 --x 0 --y 780\n\nBottom Left Demo\nWindow animates to the bottom-left corner\nDemonstrates smooth transitions\nBody for Bottom Left";
pub const CARD_CENTER: &str = "--title \"Center\" --width 400 --height 300 --x 760 --y 390\n\nCenter Demo\nWindow moves to the center of the screen\nCombines movement and title update\nBody for Center";

fn main() {
    // Start e_window as a child process with piped stdin
    let mut child = Command::new("cargo")
        .args(&["run", "--bin", "e_window"])
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start e_window");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");

    // Send DEFAULT_CARD_TEMPLATE as the first command
    println!("Sending DEFAULT_CARD_TEMPLATE");
    stdin
        .write_all(DEFAULT_CARD_TEMPLATE.as_bytes())
        .expect("Failed to write");
    stdin.flush().expect("Failed to flush");
    println!("Sent DEFAULT_CARD_TEMPLATE");
    thread::sleep(Duration::from_millis(5000));

    // Send control commands with delays
    let cmds = [
        "!control: delay 3500\n", // 3.5 seconds delay before next command
        "!control: set_title Realtime Control Demo\n",
        "!control: set_rect_eased 300 300 600 400 1000 linear\n",
        "!control: delay 3000\n",
        // Top-left demo
        "!control: set_rect_eased 0 0 400 300 1000 linear\n", // Top-left animation
        "!control: delay 3000\n",
        "!control: begin_document\n",
        &format!("{}\n", CARD_TOP_LEFT),
        "this is the content of the document\n",
        "!control: content Another line.\n",
        "!control: end_document\n",
        "!control: delay 3000\n",
        // Top-right demo
        "!control: begin_document\n",
        &format!("{}\n", CARD_TOP_RIGHT),
        "!control: end_document\n",
        "!control: delay 1000\n",
        "!control: set_rect_eased 1520 0 400 300 1000 linear\n", // Top-right
        "!control: delay 3000\n",
        // Bottom-right demo
        "!control: begin_document\n",
        &format!("{}\n", CARD_BOTTOM_RIGHT),
        "!control: end_document\n",
        "!control: delay 1000\n",
        "!control: set_rect_eased 1520 780 400 300 1000 linear\n", // Bottom-right
        "!control: delay 3000\n",
        // Bottom-left demo
        "!control: begin_document\n",
        &format!("{}\n", CARD_BOTTOM_LEFT),
        "!control: end_document\n",
        "!control: delay 1000\n",
        "!control: set_rect_eased 0 780 400 300 1000 linear\n", // Bottom-left
        "!control: delay 3000\n",
        // Center demo
        "!control: begin_document\n",
        &format!("{}\n", CARD_CENTER),
        "!control: end_document\n",
        "!control: delay 1000\n",
        "!control: set_rect_eased 760 390 400 300 1000 linear\n", // Center (optional)
        "!control: set_title Updated Title\n",
        "!control: delay 3000\n", // Final 3 second delay
        "!control: exit\n",
    ];

    for cmd in cmds.iter() {
        println!("Sending: {}", cmd.trim());
        stdin.write_all(cmd.as_bytes()).expect("Failed to write");
        stdin.flush().expect("Failed to flush");
        println!("Sent: {}", cmd.trim()); // Debug print to confirm sent
        if cmd.trim() == "!control: exit" {
            break; // Stop sending further commands after exit
        }
        thread::sleep(Duration::from_millis(500));
    }
    child.wait().expect("Child process wasn't running");
    // No need for extra sleep; exit is now controlled by command
    // e_window will close when its window is closed or process exits
}
