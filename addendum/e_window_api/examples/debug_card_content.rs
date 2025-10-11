// Debug example to see what content is being sent to e_window

use e_window_api::{EWindow, WindowConfig, Card};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Debug Message Box Content ===");
    
    // Create a simple card like the msgbox API does
    let card = Card::new()
        .window_title("ℹ️ Debug Test")
        .field("type", "Information")
        .card_title("This is a test message to debug the content display.")
        .header("Debug Test Card")
        .caption("Content display verification")
        .body_line("")
        .body_line("Additional content line 1")
        .body_line("Additional content line 2")
        .body_line("")
        .body_line("This should be visible in the window!");

    // Print what will be sent
    println!("Card document content:");
    println!("======================");
    println!("{}", card.to_document());
    println!("======================");
    
    // Configure window (centered)
    let (screen_width, screen_height) = e_window_api::get_screen_size();
    let window_width = 450u32;
    let window_height = 250u32;
    let centered_x = (screen_width.saturating_sub(window_width) / 2) as i32;
    let centered_y = (screen_height.saturating_sub(window_height) / 2) as i32;
    
    let config = WindowConfig::new()
        .title("Debug Test")
        .size(window_width, window_height)
        .position(centered_x, centered_y);

    // Launch window and show card
    println!("Launching window...");
    let mut window = EWindow::launch(config).await?;
    
    println!("Showing card...");
    window.show_card(card).await?;
    
    println!("Card displayed. Window will close in 10 seconds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    
    window.close().await?;
    println!("Window closed.");
    
    Ok(())
}