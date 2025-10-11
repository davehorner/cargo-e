// Debug version of realtime_control_api to see what's happening

use e_window_api::{EWindow, WindowConfig, Card, AnimationEasing, ControlCommand, get_screen_size};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DEBUG Realtime Control Demo ===");

    // Get screen dimensions dynamically
    let (screen_width, screen_height) = get_screen_size();
    println!("Detected screen size: {}x{}", screen_width, screen_height);

    // Calculate positions based on screen size
    let window_width = 400u32;
    let window_height = 300u32;
    let center_x = ((screen_width - window_width) / 2) as i32;
    let center_y = ((screen_height - window_height) / 2) as i32;

    // Configure the initial window
    let config = WindowConfig::new()
        .title("Debug: realtime_control")
        .size(window_width, window_height)
        .position(center_x, center_y);

    // Launch the window
    println!("[DEBUG] Launching window...");
    let mut window = EWindow::launch(config).await?;
    println!("[DEBUG] Window launched successfully");

    // Create the initial card
    let initial_card = Card::new()
        .window_title("Debug: Initial Card")
        .field("test", "initial")
        .card_title("This is the initial card content.")
        .header("Debug Test Initial")
        .caption("First card verification")
        .body_line("You should see this text clearly.")
        .body_line("If this appears, the initial card works!");

    println!("[DEBUG] Initial card document:");
    println!("=== INITIAL CARD START ===");
    println!("{}", initial_card.to_document());
    println!("=== INITIAL CARD END ===");

    // Show the initial card and wait
    println!("[DEBUG] Sending initial card...");
    window.show_card(initial_card).await?;
    println!("[DEBUG] Initial card sent, waiting 3 seconds...");
    sleep(Duration::from_secs(3)).await;

    // Test simple control commands
    println!("[DEBUG] Sending title change command...");
    window.send_command(ControlCommand::SetTitle("Title Changed!".to_string())).await?;
    println!("[DEBUG] Title change sent, waiting 2 seconds...");
    sleep(Duration::from_secs(2)).await;

    // Test a new card with show_card method
    let second_card = Card::new()
        .window_title("Debug: Second Card")
        .field("test", "second")
        .card_title("This is the second card content.")
        .header("Debug Test Second")
        .caption("Title change verification")
        .body_line("The title should have changed.")
        .body_line("If you can see this, show_card() works!");

    println!("[DEBUG] Second card document:");
    println!("=== SECOND CARD START ===");
    println!("{}", second_card.to_document());
    println!("=== SECOND CARD END ===");

    println!("[DEBUG] Sending second card...");
    window.show_card(second_card).await?;
    println!("[DEBUG] Second card sent, waiting 3 seconds...");
    sleep(Duration::from_secs(3)).await;

    // Test manual document sending (like the original does)
    let third_card = Card::new()
        .window_title("Debug: Third Card (Manual)")
        .field("test", "manual")
        .card_title("This is manually sent content.")
        .header("Debug Test Manual")
        .caption("Manual document protocol verification")
        .body_line("This uses BeginDocument/EndDocument commands.")
        .body_line("If you see this, manual sending works!");

    println!("[DEBUG] Third card document:");
    println!("=== THIRD CARD START ===");
    println!("{}", third_card.to_document());
    println!("=== THIRD CARD END ===");

    println!("[DEBUG] Sending BeginDocument command...");
    window.send_command(ControlCommand::BeginDocument).await?;
    
    println!("[DEBUG] Sending document content manually...");
    window.send_document(&third_card.to_document()).await?;
    
    println!("[DEBUG] Sending EndDocument command...");
    window.send_command(ControlCommand::EndDocument).await?;
    
    println!("[DEBUG] Manual document sent, waiting 3 seconds...");
    sleep(Duration::from_secs(3)).await;

    // Close window
    println!("[DEBUG] Closing window...");
    window.close().await?;
    println!("[DEBUG] Demo completed successfully!");

    Ok(())
}