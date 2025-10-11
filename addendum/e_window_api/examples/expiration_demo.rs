use e_window_api::{EWindow, WindowConfig, Card};
use tokio::time::{sleep, Duration};

/// Demonstrates expiration field functionality in e_window_api
/// 
/// This example shows how to create cards with expiration fields that should
/// display countdown timers showing when the window will auto-close.
/// The expiration field type is designed to work with e_window's timer display.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting expiration field demo...");

    // Test 1: Simple expiration demo
    println!("\n=== Test 1: Simple Expiration Demo ===");
    
    let config = WindowConfig::new()
        .title("Auto-Close Demo")
        .size(600, 400)
        .position(100, 100);

    let mut window = EWindow::launch(config).await?;

    let card = Card::new()
        .window_title("Auto-Close Demo")
        .field("demo", "expiration_test")
        .expiration("auto_close", 15)  // 15 seconds
        .card_title("⏰ Auto-Close Demo")
        .header("This window will auto-close automatically")
        .caption("Watch the countdown timer")
        .body_line("")
        .body_line("🔢 The 'auto_close' field shows countdown")
        .body_line("📊 Field type: expiration")
        .body_line("⚡ Duration: 15 seconds")
        .body_line("")
        .body_line("The expiration field should display a countdown timer")
        .body_line("showing time remaining until auto-close.");

    window.show_card(card).await?;

    println!("Window 1 launched with 15-second expiration timer");
    
    // Wait 8 seconds, then show another demo
    sleep(Duration::from_secs(8)).await;

    // Test 2: Multiple expiration timers
    println!("\n=== Test 2: Multiple Expiration Timers ===");
    
    let config2 = WindowConfig::new()
        .title("Multiple Timers")
        .size(700, 500)
        .position(200, 200);

    let mut window2 = EWindow::launch(config2).await?;

    let card2 = Card::new()
        .window_title("Multiple Timers Demo")
        .field("status", "active")
        .expiration("warning", 10)      // Warning at 10 seconds
        .expiration("shutdown", 25)     // Final close at 25 seconds
        .duration_field("uptime", "00:00:08.000")  // 8 seconds elapsed
        .datetime_field("started", &std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs().to_string())
        .card_title("🕐 Multiple Timer Demo")
        .header("Demonstrating multiple expiration fields")
        .caption("Different timers for different purposes")
        .body_line("")
        .body_line("⚠️  Warning timer: 10 seconds")
        .body_line("🛑 Shutdown timer: 25 seconds")
        .body_line("⏱️  Uptime: Shows elapsed time")
        .body_line("📅 Started: Shows start timestamp")
        .body_line("")
        .body_line("This demonstrates how multiple time-based")
        .body_line("fields can be used together for complex")
        .body_line("timing scenarios.");

    window2.show_card(card2).await?;

    println!("Window 2 launched with multiple expiration timers");

    // Wait a bit more to see the timers in action
    sleep(Duration::from_secs(10)).await;

    // Test 3: Real auto-close functionality  
    println!("\n=== Test 3: Actual Auto-Close Implementation ===");
    
    let config3 = WindowConfig::new()
        .title("Real Auto-Close")
        .size(500, 350)
        .position(300, 300);

    let mut window3 = EWindow::launch(config3).await?;

    let close_time = 10; // seconds
    let card3 = Card::new()
        .window_title("Real Auto-Close")
        .field("demo", "auto_close_implementation")
        .expiration("closing_in", close_time)
        .card_title("🚀 Real Auto-Close")
        .header("This window will actually close automatically")
        .body_line("")
        .body_line(&format!("⏰ Closing in {} seconds", close_time))
        .body_line("👀 Watch the expiration field countdown")
        .body_line("🔄 Window will close when timer reaches zero");

    window3.show_card(card3).await?;

    println!("Window 3 launched - will auto-close in {} seconds", close_time);

    // Actually close the window after the specified time
    sleep(Duration::from_secs(close_time as u64)).await;
    println!("Auto-closing window 3...");
    window3.close().await?;

    // Clean up remaining windows after demo
    sleep(Duration::from_secs(5)).await;
    println!("\nCleaning up remaining demo windows...");
    
    window.close().await?;
    window2.close().await?;

    println!("Demo completed!");
    println!("\nExpiration field features demonstrated:");
    println!("✅ Single expiration timer");
    println!("✅ Multiple expiration timers");
    println!("✅ Mixed field types (expiration, duration, datetime)");
    println!("✅ Actual auto-close implementation");
    
    Ok(())
}