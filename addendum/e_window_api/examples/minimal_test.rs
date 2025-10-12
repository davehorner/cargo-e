// Minimal test to debug why show_card works in notify_async but not other examples

use e_window_api::{Card, EWindow, WindowConfig};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing minimal show_card...");

    // Use exact same approach as notify_async
    let (screen_width, screen_height) = e_window_api::get_screen_size();
    let window_width = 400u32;
    let window_height = 150u32;
    let window_x = (screen_width.saturating_sub(window_width + 20)) / 2;
    let window_y = (screen_height.saturating_sub(window_height + 20)) / 2;

    println!(
        "[DEBUG] Window config: position=({}, {}), size=({}, {})",
        window_x, window_y, window_width, window_height
    );

    let config = WindowConfig::new()
        .title("Minimal Test")
        .size(window_width, window_height)
        .position(window_x as i32, window_y as i32);

    println!("[DEBUG] Launching window...");
    let mut window = EWindow::launch(config).await?;
    println!("[DEBUG] Window launched successfully");

    // Create card exactly like notify_async does
    let card = Card::new()
        .window_title("ℹ️ Test Card")
        .field("type", "Test")
        .field("duration", "10 seconds")
        .card_title("This is a test message")
        .header("Minimal Test Example")
        .caption("Testing show_card functionality")
        .body_line("")
        .body_line("Testing show_card functionality...")
        .body_line("⏱️ Will stay open for 10 seconds");

    println!("[DEBUG] Card created. Document content:");
    println!("=== CARD CONTENT START ===");
    println!("{}", card.to_document());
    println!("=== CARD CONTENT END ===");

    println!("[DEBUG] Sending card to window...");
    window.show_card(card).await?;
    println!("[DEBUG] Card sent successfully");

    println!("[DEBUG] Waiting 10 seconds...");
    sleep(Duration::from_secs(10)).await;

    println!("[DEBUG] Closing window...");
    // Window should close automatically when dropped

    println!("Test completed!");
    Ok(())
}
