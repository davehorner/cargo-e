use e_window_api::{EWindow, WindowConfig, Card};

/// Demonstrates anchor link functionality in e_window_api
/// 
/// This example shows how to create cards with clickable anchor links
/// that can:
/// - Launch other e_window instances
/// - Open URLs in the default browser  
/// - Execute system commands
/// - Display text-only anchors (non-clickable)
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting anchor links demo...");

    // Create a window to demonstrate anchor functionality
    let config = WindowConfig::new()
        .title("Anchor Links Demo")
        .size(800, 600)
        .position(200, 200);

    let mut window = EWindow::launch(config).await?;

    // Create a card with various types of anchors
    let main_card = Card::new()
        .window_title("Anchor Links Demo")
        .field("demo", "anchor_links")
        .field("version", "1.0")
        .card_title("🔗 e_window Anchor Links Demo")
        .header("Click the buttons below to test different anchor types")
        .caption("This demonstrates the anchor link functionality")
        .body_line("")
        .body_line("📋 Available anchor types:")
        .body_line("")
        .anchor("🪟 Launch another e_window", "e_window --title \"Spawned Window!\" --width 400 --height 300 --x 100 --y 100")
        .body_line("")
        .anchor("🌐 Open Google in browser", "https://www.google.com")
        .body_line("")
        .anchor("🧮 Open Calculator", "calc.exe")
        .body_line("")
        .anchor("📂 Open File Explorer", "explorer.exe")
        .body_line("")
        .anchor_text("📝 This is a text-only anchor (not clickable)")
        .body_line("")
        .body_line("💡 Tips:")
        .body_line("- URL anchors (http/https) open in your default browser")
        .body_line("- Command anchors execute system commands")
        .body_line("- e_window anchors can launch new windows with specific configs")
        .body_line("- Text-only anchors are for display purposes");

    window.show_card_with_auto_close(main_card, 30).await?;

    println!("Demo window launched! Try clicking the anchor buttons.");
    println!("Window displays countdown timer automatically!");

    // Wait for auto-close (30 seconds)
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    println!("Window auto-closed successfully!");

    window.close().await?;
    Ok(())
}