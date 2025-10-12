use e_window_api::{Card, EWindow, WindowConfig};
use tokio::time::{sleep, Duration};

/// Demonstrates automatic expiration field injection for auto-close functionality
///
/// This example shows how the API can automatically add expiration fields when
/// auto-close is needed, without requiring manual field management.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting auto-close demo with automatic expiration fields...");

    // Demo 1: Simple auto-close with automatic expiration field
    println!("\n=== Demo 1: Automatic Expiration Field ===");

    let config = WindowConfig::new()
        .title("Auto-Close with Auto-Expiration")
        .size(600, 400)
        .position(100, 100);

    let mut window = EWindow::launch(config).await?;

    let card = Card::new()
        .window_title("Auto-Close Demo")
        .field("demo", "auto_expiration")
        .card_title("🚀 Automatic Expiration Field")
        .header("This window will auto-close in 12 seconds")
        .body_line("")
        .body_line("✨ The expiration field was added automatically!")
        .body_line("⏱️  No manual .expiration() call needed")
        .body_line("🔄 The API handles countdown timer injection")
        .body_line("")
        .body_line("Watch the 'auto_close' field show the countdown");

    // This automatically adds an expiration field called "auto_close"
    window.show_card_with_auto_close(card, 12).await?;

    println!("Window with automatic expiration field launched");

    // Wait for it to auto-close
    sleep(Duration::from_secs(12)).await;
    window.close().await?;

    // Demo 2: Launch and auto-close in one call
    println!("\n=== Demo 2: One-Shot Auto-Close ===");

    let config2 = WindowConfig::new()
        .title("One-Shot Auto-Close")
        .size(500, 300)
        .position(200, 200);

    let card2 = Card::new()
        .window_title("One-Shot Demo")
        .field("type", "one_shot")
        .card_title("⚡ One-Shot Auto-Close")
        .header("This window is fully managed by the API")
        .body_line("")
        .body_line("🎯 Launched with auto-close built-in")
        .body_line("⏰ Will close automatically in 8 seconds")
        .body_line("📦 No manual window management needed");

    // This launches, shows, waits, and closes all in one call
    println!("Launching one-shot window (will auto-close in 8 seconds)...");
    EWindow::launch_with_auto_close(config2, card2, 8).await?;
    println!("One-shot window completed its lifecycle");

    // Demo 3: Multiple auto-close windows with different timers
    println!("\n=== Demo 3: Multiple Auto-Close Windows ===");

    let mut windows = Vec::new();
    let timers = vec![5, 7, 10];

    for (i, &timer) in timers.iter().enumerate() {
        let config = WindowConfig::new()
            .title(&format!("Auto-Close #{}", i + 1))
            .size(400, 250)
            .position(300 + i as i32 * 50, 300 + i as i32 * 50);

        let mut window = EWindow::launch(config).await?;

        let card = Card::new()
            .window_title(&format!("Auto-Close #{}", i + 1))
            .field("window_id", &format!("window_{}", i + 1))
            .field("sequence", &(i + 1).to_string())
            .card_title(&format!("🎪 Window #{}", i + 1))
            .header(&format!("Closing in {} seconds", timer))
            .body_line("")
            .body_line(&format!("⏱️  Timer: {} seconds", timer))
            .body_line(&format!("🎯 This is window #{} of 3", i + 1))
            .body_line("📊 Multiple windows demonstration");

        window.show_card_with_auto_close(card, timer).await?;
        windows.push((window, timer));

        println!("Launched window {} with {}-second timer", i + 1, timer);

        // Small delay between launches
        sleep(Duration::from_millis(500)).await;
    }

    // Wait for all windows to auto-close
    println!("Waiting for all windows to auto-close...");
    for (window, timer) in windows {
        sleep(Duration::from_secs(timer as u64)).await;
        window.close().await?;
        println!("Window with {}-second timer closed", timer);
    }

    println!("\n🎉 Auto-close demo completed!");
    println!("\nFeatures demonstrated:");
    println!("✅ Automatic expiration field injection");
    println!("✅ show_card_with_auto_close() method");
    println!("✅ launch_with_auto_close() one-shot method");
    println!("✅ Multiple windows with different auto-close timers");
    println!("✅ Improved countdown visibility (orange instead of yellow)");

    Ok(())
}
