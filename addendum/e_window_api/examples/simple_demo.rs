// Example: Simple usage demonstration
// Shows the most basic usage of the e_window_api

use e_window_api::{EWindow, WindowConfig, Card};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple e_window_api demonstration...");

    // Quick way: launch with default config
    let config = WindowConfig::new()
        .title("Simple Demo")
        .size(400, 300)
        .position(300, 300);

    let card = Card::new()
        .field("app", "simple_demo")
        .field("status", "running")
        .card_title("Simple e_window_api Demonstration")
        .header("Welcome to the API")
        .caption("Basic usage example")
        .body_line("This is a simple demonstration of e_window_api")
        .body_line("")
        .body_line("The API makes it easy to:")
        .body_line("- Launch windows with custom configuration")
        .body_line("- Display structured content")
        .body_line("- Control window properties")
        .body_line("")
        .body_line("Window will auto-close with countdown timer in 5 seconds...");

    // Launch window with auto-close (one-shot method)
    EWindow::launch_with_auto_close(config, card, 5).await?;

    println!("Demo completed! Window auto-closed with countdown timer.");
    Ok(())
}