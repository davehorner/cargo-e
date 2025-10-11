// Example: Panic card simulation using the e_window_api
// This is the high-level API version of the panic_card_sim.rs example

use e_window_api::{EWindow, WindowConfig, Card};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simulating panic card with e_window_api...");

    // Simulated panic info
    let target_name = "my_binary";
    let cargo_e_version = env!("CARGO_PKG_VERSION");
    let panic_line = "thread 'main' panicked at 'something went wrong', src/main.rs:42:9";
    let prior_message = Some("Additional context: failed to connect to database.");

    // Configure the window for panic display
    let config = WindowConfig::new()
        .title(&format!("panic: {}", target_name))
        .size(400, 300)
        .position(100, 100);

    // Launch the window
    let mut window = EWindow::launch(config).await?;

    // Create the panic card
    let mut panic_card = Card::new()
        .window_title(&format!("panic: {}", target_name))
        .size(400, 300)
        .field("target", target_name)
        .field("cargo-e", cargo_e_version)
        .card_title(&format!("Panic detected in {}", target_name))
        .header("Runtime Error")
        .caption("Application panic occurred")
        .body_line(panic_line);

    // Add prior message if available
    if let Some(msg) = prior_message {
        panic_card = panic_card.body_line(msg);
    }

    // Show the panic card
    window.show_card(panic_card).await?;

    println!("Panic card displayed. Window will remain open until closed by user.");
    
    // Wait for the window to be closed by the user
    let exit_status = window.wait().await?;
    println!("Window closed with exit status: {:?}", exit_status);

    Ok(())
}