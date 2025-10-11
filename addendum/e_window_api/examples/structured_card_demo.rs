use e_window_api::Card;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Structured Card API Demo ===");
    
    // Create a card using the new structured API
    let card = Card::new()
        .window_title("Structured Card Demo")
        // .size(600, 400)
        // .position(200, 150)
        .field("version", "1.0")
        .field("author", "GitHub Copilot")
        .typed_field("rating", "5", "number")
        .card_title("Welcome to the Structured API!")
        .header("This is the header section")
        .caption("This is the caption section")
        .body_line("This demonstrates the new structured content API.")
        .body_line("You can now set:")
        .body_line("- Card title (first line after blank separator)")
        .body_line("- Header (second line)")
        .body_line("- Caption (third line)")
        .body_line("- Body content (remaining lines)")
        .body_line("")
        .body_line("This follows the e_window document protocol structure!");

    println!("Generated document:");
    println!("{}", card.to_document());
    
    // Show the card
    card.show().await?;

    Ok(())
}