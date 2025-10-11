// Test that mimics the original realtime_control.rs approach exactly

use e_window_api::{EWindow, WindowConfig, ControlCommand};
use tokio::time::{sleep, Duration};

const DEFAULT_CARD_TEMPLATE: &str = r#"--title "Demo: API Test" --width 400 --height 300 --x 200 --y 200
name | api_test | string
version | 1.0 | string
author | Test | string

Welcome to *API TEST* e_window!
This demo tests if sending raw card content works like the original.

Testing the exact approach from realtime_control.rs:
- Send card template first
- Then send control commands
- Should display content properly"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing original approach...");

    let config = WindowConfig::new()
        .title("API Test")
        .size(400, 300)
        .position(200, 200);

    println!("[DEBUG] Launching window...");
    let mut window = EWindow::launch(config).await?;
    println!("[DEBUG] Window launched successfully");

    // Send the initial card template exactly like the original does
    println!("[DEBUG] Sending initial card template...");
    window.send_document(DEFAULT_CARD_TEMPLATE).await?;
    println!("[DEBUG] Initial card template sent");
    
    // Wait a bit
    sleep(Duration::from_secs(5)).await;
    
    // Now send some control commands like the original
    println!("[DEBUG] Sending control commands...");
    let commands = vec![
        ControlCommand::Delay(2000),
        ControlCommand::SetTitle("Updated Title".to_string()),
        ControlCommand::Delay(3000),
    ];
    
    window.send_commands(commands).await?;
    println!("[DEBUG] Control commands sent");
    
    // Wait and finish
    sleep(Duration::from_secs(5)).await;
    println!("Test completed!");
    
    Ok(())
}