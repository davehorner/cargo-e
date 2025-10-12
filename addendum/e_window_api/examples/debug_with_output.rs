// Test with e_window stdout/stderr visible

use e_window_api::{Card, EWindow, WindowConfig};
use std::process::{Command, Stdio};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing with e_window output visible...");

    // Manually launch e_window to see its output
    let config = WindowConfig::new()
        .title("Debug Test")
        .size(400, 300)
        .position(200, 200);

    println!("[DEBUG] Config args: {:?}", config.to_args());

    // Launch e_window manually with visible output
    println!("[DEBUG] Starting e_window with visible output...");

    let e_window_project = std::path::Path::new("../e_window");
    if !e_window_project.exists() {
        return Err("e_window project not found at ../e_window".into());
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--bin").arg("e_window").arg("--");
    cmd.args(config.to_args());
    cmd.current_dir(&e_window_project);
    cmd.stdin(Stdio::piped());
    // Keep stdout and stderr visible!
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    println!("[DEBUG] Command: {:?}", cmd);

    let mut process = cmd.spawn()?;
    let stdin = process.stdin.take().unwrap();
    let mut stdin = std::io::BufWriter::new(stdin);

    println!("[DEBUG] e_window process started, waiting 30 seconds for compilation and startup...");
    sleep(Duration::from_secs(30)).await;

    // Now try to send a card
    use std::io::Write;
    let card = Card::new()
        .window_title("Debug Card")
        .card_title("This should appear in the window")
        .header("Debug Test")
        .caption("Output verification")
        .body_line("If you see this, the content is working")
        .body_line("Debug test successful");

    let document = card.to_document();
    println!("[DEBUG] Sending card document:");
    println!("=== DOCUMENT START ===");
    println!("{}", document);
    println!("=== DOCUMENT END ===");

    // Send BeginDocument
    let begin_cmd = "!control: begin_document\n";
    stdin.write_all(begin_cmd.as_bytes())?;
    stdin.flush()?;
    println!("[DEBUG] Sent begin_document");

    // Send document content
    for line in document.lines() {
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
    }
    stdin.flush()?;
    println!("[DEBUG] Sent document content");

    // Send EndDocument
    let end_cmd = "!control: end_document\n";
    stdin.write_all(end_cmd.as_bytes())?;
    stdin.flush()?;
    println!("[DEBUG] Sent end_document");

    println!("[DEBUG] Waiting 10 seconds to see the window...");
    sleep(Duration::from_secs(10)).await;

    println!("[DEBUG] Terminating process...");
    let _ = process.kill();
    println!("Test completed!");

    Ok(())
}
