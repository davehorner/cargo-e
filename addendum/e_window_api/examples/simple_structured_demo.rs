use e_window_api::Card;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Structured Content Demo ===");
    
    // Example 1: Basic structured content
    let card1 = Card::new()
        .window_title("Example 1: Basic Structure")
        .card_title("My Card Title")
        .header("Important Information")
        .caption("Status: Active")
        .body("This is the main body content.\nIt can span multiple lines.\n\nHere's a second paragraph.");

    println!("Card 1 document:");
    println!("{}", card1.to_document());
    println!("---");

    // Example 2: Using individual body lines
    let card2 = Card::new()
        .window_title("Example 2: Individual Lines")
        .card_title("Task List")
        .header("Daily Tasks")
        .caption("Priority: High")
        .body_line("1. Review code")
        .body_line("2. Write documentation")
        .body_line("3. Fix bugs")
        .body_line("4. Update tests");

    println!("Card 2 document:");
    println!("{}", card2.to_document());
    println!("---");

    // Example 3: With fields
    let card3 = Card::new()
        .window_title("Example 3: With Fields")
        .field("project", "e_window")
        .field("language", "Rust")
        .typed_field("lines_of_code", "2500", "number")
        .card_title("Project Status")
        .header("Development Progress")
        .caption("Phase: Beta Testing")
        .body_lines(vec![
            "The project is progressing well.",
            "Key features implemented:",
            "- Structured card API",
            "- Window positioning",
            "- Document protocol",
        ]);

    println!("Card 3 document:");
    println!("{}", card3.to_document());

    // Show one of the cards
    println!("\nShowing Card 1...");
    card1.show_blocking()?;

    Ok(())
}