// Test to manually send commands to e_window to debug the protocol

use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing direct e_window communication...");

    // Launch e_window directly
    let mut child = Command::new("..\\..\\target\\debug\\e_window.exe")
        .arg("--title")
        .arg("Direct Test")
        .arg("--width")
        .arg("400")
        .arg("--height")
        .arg("300")
        .arg("--x")
        .arg("100")
        .arg("--y")
        .arg("100")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        println!("Sending commands to e_window...");

        // Send a begin document command
        stdin.write_all(b"!control: begin_document\n")?;
        stdin.flush()?;

        thread::sleep(Duration::from_millis(500));

        // Send some content directly (not as control commands)
        stdin.write_all(b"title | Test Direct Communication | string\n")?;
        stdin.write_all(b"type | Information | string\n")?;
        stdin.write_all(b"\n")?;
        stdin.write_all(b"This is a test message sent directly to e_window.\n")?;
        stdin.write_all(b"\n")?;
        stdin.write_all(b"If you can see this, the communication is working!\n")?;
        stdin.flush()?;

        thread::sleep(Duration::from_millis(500));

        // Send end document command
        stdin.write_all(b"!control: end_document\n")?;
        stdin.flush()?;

        println!("Commands sent. Window should display content.");
        println!("Window will close in 10 seconds...");

        thread::sleep(Duration::from_secs(10));

        // Close the window
        stdin.write_all(b"!control: exit\n")?;
        stdin.flush()?;
    }

    // Wait for the process to finish
    let output = child.wait_with_output()?;

    println!("Exit status: {}", output.status);
    println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));

    Ok(())
}
