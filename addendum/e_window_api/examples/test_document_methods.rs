// Test to compare show_card vs manual document sending

use e_window_api::{Card, ControlCommand, EWindow, WindowConfig};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing document methods...");

    // Configure the window
    let config = WindowConfig::new()
        .title("Document Method Test")
        .size(400, 300)
        .position(200, 200);

    // Launch the window
    let mut window = EWindow::launch(config).await?;
    println!("Window launched.");

    // Test 1: show_card method
    println!("\n=== Test 1: Using show_card() ===");
    let card1 = Card::new()
        .window_title("Test Card 1")
        .card_title("This card was sent using show_card()")
        .header("Method Test 1")
        .caption("show_card() method verification")
        .body_line("Should display immediately")
        .body_line("Testing show_card method");

    println!("[DEBUG] Card 1 document content:");
    println!("=== CARD 1 CONTENT START ===");
    println!("{}", card1.to_document());
    println!("=== CARD 1 CONTENT END ===");

    println!("[DEBUG] Calling show_card()...");
    window.show_card(card1).await?;
    println!("[DEBUG] show_card() completed successfully");
    println!("show_card() called, waiting 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    // Test 2: manual document protocol
    println!("\n=== Test 2: Using manual document protocol ===");

    // Send BeginDocument
    println!("[DEBUG] Sending BeginDocument...");
    let begin_commands = vec![ControlCommand::BeginDocument];
    window.send_commands(begin_commands).await?;
    println!("[DEBUG] BeginDocument sent");

    // Send card content
    let card2 = Card::new()
        .window_title("Test Card 2")
        .card_title("This card was sent using manual protocol")
        .header("Method Test 2")
        .caption("Manual protocol verification")
        .body_line("BeginDocument -> send_document -> EndDocument")
        .body_line("Testing manual method");

    println!("[DEBUG] Card 2 document content:");
    println!("=== CARD 2 CONTENT START ===");
    println!("{}", card2.to_document());
    println!("=== CARD 2 CONTENT END ===");

    println!("[DEBUG] Sending document content...");
    window.send_document(&card2.to_document()).await?;
    println!("[DEBUG] Document content sent");

    // Send EndDocument
    println!("[DEBUG] Sending EndDocument...");
    let end_commands = vec![ControlCommand::EndDocument];
    window.send_commands(end_commands).await?;
    println!("[DEBUG] EndDocument sent");

    println!("Manual protocol completed, waiting 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    // Test 3: Back to show_card to verify it still works
    println!("\n=== Test 3: Back to show_card() ===");
    let card3 = Card::new()
        .window_title("Test Card 3")
        .card_title("Back to show_card() method")
        .header("Method Test 3")
        .caption("Final show_card() verification")
        .body_line("Final test")
        .body_line("Should work normally");

    println!("[DEBUG] Card 3 document content:");
    println!("=== CARD 3 CONTENT START ===");
    println!("{}", card3.to_document());
    println!("=== CARD 3 CONTENT END ===");

    println!("[DEBUG] Calling final show_card()...");
    window.show_card(card3).await?;
    println!("[DEBUG] Final show_card() completed successfully");
    println!("Final show_card() called, waiting 3 seconds...");
    sleep(Duration::from_secs(3)).await;

    println!("Test completed!");
    Ok(())
}
