// Example: Realtime control using the e_window_api
// This is the high-level API version of the realtime_control.rs example

use e_window_api::{EWindow, WindowConfig, Card, AnimationEasing, ControlCommand, get_screen_size};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting realtime control demo with e_window_api...");

    // Get screen dimensions dynamically
    let (screen_width, screen_height) = get_screen_size();
    println!("Detected screen size: {}x{}", screen_width, screen_height);

    // Calculate positions based on screen size
    let margin = 50;
    let window_width = 400u32;
    let window_height = 300u32;
    
    // Calculate corner positions with margin
    let top_left_x = margin as i32;
    let top_left_y = margin as i32;
    let top_right_x = (screen_width - window_width - margin) as i32;
    let top_right_y = margin as i32;
    let bottom_right_x = (screen_width - window_width - margin) as i32;
    let bottom_right_y = (screen_height - window_height - margin) as i32;
    let bottom_left_x = margin as i32;
    let bottom_left_y = (screen_height - window_height - margin) as i32;
    let center_x = ((screen_width - window_width) / 2) as i32;
    let center_y = ((screen_height - window_height) / 2) as i32;

    // Configure the initial window
    let config = WindowConfig::new()
        .title("Demo: realtime_control")
        .size(window_width, window_height)
        .position(200, 200);

    // Launch the window
    let mut window = EWindow::launch(config).await?;

    // Create the initial card
    let initial_card = Card::new()
        .window_title("Demo: realtime_control")
        .size(window_width, window_height)
        .position(200, 200)
        .field("name", "realtime_control")
        .field("version", "1.0")
        .field("author", "Dave Horner")
        .field("screen_size", &format!("{}x{}", screen_width, screen_height))
        .card_title("Welcome to REALTIME CONTROL e_window!")
        .header("Interactive API Demonstration")
        .caption("Live control commands via Rust API")
        .body_line("This demo shows how you can send live control commands to a running e_window instance via the Rust API.")
        .body_line("")
        .body_line("How this works:")
        .body_line("- This Rust program launches e_window using the high-level API.")
        .body_line("- Detects your screen size and calculates appropriate positions.")
        .body_line("- The initial card is sent first, setting up the window and its fields.")
        .body_line("- Then, a sequence of control commands is sent with timed delays, demonstrating dynamic updates:")
        .body_line("    - Window title and position changes using API methods.")
        .body_line("    - Animated movement and resizing of the window with eased animations.")
        .body_line("    - Swapping card content in real time by sending new card data.")
        .body_line("    - Each card (Top Left, Top Right, Bottom Right, Bottom Left, Center) is shown in turn.")
        .body_line("    - The window closes automatically at the end.")
        .body_line("")
        .body_line(&format!("Your screen: {}x{}", screen_width, screen_height))
        .body_line("How to use:")
        .body_line("- Run this demo to see the window respond to API commands.")
        .body_line("- You can modify the cards or control commands below, then re-run to see your changes in action!")
        .body_line("- The API provides convenient methods for window management, content updates, and more.");

    // Show the initial card and wait (like the original)
    println!("Sending initial card...");
    println!("[DEBUG] Initial card content:");
    println!("=== INITIAL CARD START ===");
    println!("{}", initial_card.to_document());
    println!("=== INITIAL CARD END ===");
    
    window.show_card(initial_card).await?;
    println!("Initial card sent, waiting 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    // Now send all control commands in sequence (like the original does)
    println!("Sending command sequence...");
    let commands = vec![
        ControlCommand::Delay(7000), // 7 seconds delay before next command (increased from 3.5)
        ControlCommand::SetTitle("Realtime Control Demo".to_string()),
        ControlCommand::SetRectEased { 
            x: center_x, y: center_y, width: window_width, height: window_height, 
            duration_ms: 2000, easing: AnimationEasing::Linear  // Slower animation
        },
        ControlCommand::Delay(5000), // 5 seconds (increased from 3)
        // Top-left demo
        ControlCommand::SetRectEased { 
            x: top_left_x, y: top_left_y, width: window_width, height: window_height, 
            duration_ms: 2000, easing: AnimationEasing::Linear  // Slower animation
        },
        ControlCommand::Delay(7000), // 7 seconds (increased from 3)
    ];
    
    window.send_commands(commands).await?;

    // Top-left demo: move FIRST, then send document (like original)
    let top_left_move_commands = vec![
        // Top-left demo - move to top-left position FIRST
        ControlCommand::SetRectEased { 
            x: top_left_x, y: top_left_y, width: window_width, height: window_height, 
            duration_ms: 2000, easing: AnimationEasing::Linear  // Slower animation
        },
        ControlCommand::Delay(3000), // Wait for animation (like original)
        ControlCommand::BeginDocument,
    ];
    window.send_commands(top_left_move_commands).await?;
    
    let top_left_card = Card::new()
        .window_title("Top Left")
        .size(window_width, window_height)
        .position(top_left_x, top_left_y)
        .card_title("Top Left Demo")
        .header("Position Control")
        .caption("Top-left corner animation")
        .body_line("Window animates to the top-left corner")
        .body_line("Demonstrates position control")
        .body_line(&format!("Position: ({}, {})", top_left_x, top_left_y))
        .body_line(&format!("Screen: {}x{}", screen_width, screen_height))
        .body_line("Body for Top Left");
    
    println!("[DEBUG] Top-left card content:");
    println!("=== TOP-LEFT CARD START ===");
    println!("{}", top_left_card.to_document());
    println!("=== TOP-LEFT CARD END ===");
    
    window.send_document(&top_left_card.to_document()).await?;
    
    let top_left_end_commands = vec![
        ControlCommand::EndDocument,
        ControlCommand::Delay(3000), // Wait like original
    ];
    window.send_commands(top_left_end_commands).await?;
    
    // Top-right demo: document FIRST, then move (like original pattern for top-right)
    let more_commands = vec![
        // Top-right demo - send document first
        ControlCommand::BeginDocument,
    ];
    window.send_commands(more_commands).await?;

    // Top-right demo - use manual document protocol BEFORE moving
    let top_right_card = Card::new()
        .window_title("Top Right")
        .card_title("Top Right Demo")
        .header("Dynamic Movement")
        .caption("Top-right corner animation")
        .body_line("Window animates to the top-right corner")
        .body_line("Shows dynamic movement")
        .body_line(&format!("Position: ({}, {})", top_right_x, top_right_y))
        .body_line(&format!("Screen: {}x{}", screen_width, screen_height))
        .body_line("Body for Top Right");
    
    println!("[DEBUG] Top-right card content:");
    println!("=== TOP-RIGHT CARD START ===");
    println!("{}", top_right_card.to_document());
    println!("=== TOP-RIGHT CARD END ===");
    
    window.send_document(&top_right_card.to_document()).await?;
    
    let top_right_end_commands = vec![
        ControlCommand::EndDocument,
        ControlCommand::Delay(1000), // Short delay like original
        // Top-right demo - move to top-right position AFTER document
        ControlCommand::SetRectEased { 
            x: top_right_x, y: top_right_y, width: window_width, height: window_height, 
            duration_ms: 2000, easing: AnimationEasing::Linear 
        },
        ControlCommand::Delay(3000), // Wait for animation to complete
    ];
    window.send_commands(top_right_end_commands).await?;
    
    let final_commands = vec![
        // Bottom-right demo - send document first
        ControlCommand::BeginDocument,
    ];
    window.send_commands(final_commands).await?;
    
    // Bottom-right card - use manual document protocol
    let bottom_right_card = Card::new()
        .window_title("Bottom Right")
        .card_title("Bottom Right Demo")
        .header("Resizing & Placement")
        .caption("Bottom-right corner animation")
        .body_line("Window animates to the bottom-right corner")
        .body_line("Illustrates resizing and placement")
        .body_line(&format!("Position: ({}, {})", bottom_right_x, bottom_right_y))
        .body_line(&format!("Screen: {}x{}", screen_width, screen_height))
        .body_line("Body for Bottom Right");
    
    println!("[DEBUG] Bottom-right card content:");
    println!("=== BOTTOM-RIGHT CARD START ===");
    println!("{}", bottom_right_card.to_document());
    println!("=== BOTTOM-RIGHT CARD END ===");
    
    window.send_document(&bottom_right_card.to_document()).await?;
    
    let bottom_right_end_commands = vec![
        ControlCommand::EndDocument,
        ControlCommand::Delay(1000), // Wait before moving
        // Bottom-right demo - move to bottom-right position
        ControlCommand::SetRectEased { 
            x: bottom_right_x, y: bottom_right_y, width: window_width, height: window_height, 
            duration_ms: 2000, easing: AnimationEasing::Linear 
        },
        ControlCommand::Delay(3000), // Wait for animation to complete
    ];
    window.send_commands(bottom_right_end_commands).await?;
    
    let bottom_commands = vec![
        // Bottom-left demo - send document first
        ControlCommand::BeginDocument,
    ];
    window.send_commands(bottom_commands).await?;
    
    // Bottom-left card - use manual document protocol
    let bottom_left_card = Card::new()
        .window_title("Bottom Left")
        .card_title("Bottom Left Demo")
        .header("Smooth Transitions")
        .caption("Bottom-left corner animation")
        .body_line("Window animates to the bottom-left corner")
        .body_line("Demonstrates smooth transitions")
        .body_line(&format!("Position: ({}, {})", bottom_left_x, bottom_left_y))
        .body_line(&format!("Screen: {}x{}", screen_width, screen_height))
        .body_line("Body for Bottom Left");
    
    println!("[DEBUG] Bottom-left card content:");
    println!("=== BOTTOM-LEFT CARD START ===");
    println!("{}", bottom_left_card.to_document());
    println!("=== BOTTOM-LEFT CARD END ===");
    
    window.send_document(&bottom_left_card.to_document()).await?;
    
    let bottom_left_end_commands = vec![
        ControlCommand::EndDocument,
        ControlCommand::Delay(1000), // Wait before moving
        // Bottom-left demo - move to bottom-left position
        ControlCommand::SetRectEased { 
            x: bottom_left_x, y: bottom_left_y, width: window_width, height: window_height, 
            duration_ms: 2000, easing: AnimationEasing::Linear 
        },
        ControlCommand::Delay(3000), // Wait for animation to complete
    ];
    window.send_commands(bottom_left_end_commands).await?;
    
    let left_commands = vec![
        // Center demo - send document first
        ControlCommand::BeginDocument,
    ];
    window.send_commands(left_commands).await?;
    
    // Center card - use manual document protocol
    let center_card = Card::new()
        .window_title("Center")
        .card_title("Center Demo")
        .header("Animation Complete")
        .caption("Return to center position")
        .body_line("Window returns to the center")
        .body_line("Full animation sequence complete")
        .body_line(&format!("Position: ({}, {})", center_x, center_y))
        .body_line(&format!("Screen: {}x{}", screen_width, screen_height))
        .body_line("Body for Center");
    
    println!("[DEBUG] Center card content:");
    println!("=== CENTER CARD START ===");
    println!("{}", center_card.to_document());
    println!("=== CENTER CARD END ===");
    
    window.send_document(&center_card.to_document()).await?;
    
    let center_end_commands = vec![
        ControlCommand::EndDocument,
        ControlCommand::Delay(1000), // Wait before moving
        // Center demo - move back to center position
        ControlCommand::SetRectEased { 
            x: center_x, y: center_y, width: window_width, height: window_height, 
            duration_ms: 2000, easing: AnimationEasing::Linear 
        },
        ControlCommand::Delay(3000), // Wait for animation to complete
    ];
    window.send_commands(center_end_commands).await?;
    
    // Final commands to wait and close
    let final_final_commands = vec![
        ControlCommand::Delay(7000), // Wait at center
        ControlCommand::SetTitle("Animation Complete!".to_string()),
        ControlCommand::Delay(3000), // Final delay before closing
    ];
    window.send_commands(final_final_commands).await?;

    println!("Animation sequence complete!");
    Ok(())
}