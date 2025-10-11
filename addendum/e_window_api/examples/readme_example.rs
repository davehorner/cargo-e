use e_window_api::Card;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== README Example Demo ===");
    println!("This example demonstrates the exact format described in the README.\n");

    // Create the card exactly as shown in the README documentation
    let card = Card::new()
        // Window configuration
        .window_title("Task Manager")
        .size(500, 400)
        .position(200, 150)
        
        // Fields section
        .field("project", "e_window")
        .field("language", "Rust")
        .typed_field("tasks_completed", "15", "number")
        .typed_field("progress", "75%", "percentage")
        
        // Content section
        .card_title("Daily Progress Report")
        .header("Development Status")
        .caption("Status: On Track")
        .body_line("Today's accomplishments:")
        .body_line("✅ Fixed positioning bugs")
        .body_line("✅ Implemented structured API")
        .body_line("✅ Updated documentation")
        .body_line("")
        .body_line("Tomorrow's goals:")
        .body_line("🎯 Add animation features")
        .body_line("🎯 Improve error messages")
        .body_line("🎯 Write more examples");

    println!("Generated document format:");
    println!("---");
    println!("{}", card.to_document());
    println!("---");
    
    println!("\nShowing the card (press OK to close)...");
    
    // Display the card
    card.show_blocking()?;
    
    println!("Demo completed!");
    
    Ok(())
}