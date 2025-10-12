// Test positioning fix - simple test with explicit positioning
use e_window_api::{Card, EWindow, WindowConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing positioning fix with a single positioned window...");

    let config = WindowConfig::new()
        .title("Test Position Window")
        .size(400, 300)
        .position(100, 100); // This should disable auto-centering

    println!("Config auto_center flag: {}", config.auto_center);
    println!("Config position: ({}, {})", config.x, config.y);
    println!("Generated args: {:?}", config.to_args());

    let mut window = EWindow::launch(config).await?;

    let card = Card::new()
        .window_title("Position Test")
        .card_title("This window should be positioned at (100, 100)")
        .header("Position Verification")
        .caption("Testing coordinate placement")
        .body_line("If it appears at (100, 100), the fix is working!")
        .body_line("If it's auto-centered, there's still an issue.");

    window.show_card(card).await?;

    // Give time to see the position
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    window.close().await?;

    Ok(())
}
