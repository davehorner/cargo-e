use e_window_api::Card;

#[test]
fn test_card_with_anchors() {
    let card = Card::new()
        .card_title("Test with Anchors")
        .body_line("This card demonstrates anchor support")
        .body_line("")
        .anchor("Click me!", "e_window --title \"you clicked!\" --width 400 --height 300")
        .anchor("Open Google", "https://www.google.com")
        .anchor("Open Calculator", "calc.exe");

    let document = card.to_document();
    
    println!("Generated document:\n{}", document);
    
    // Test that anchors are properly formatted
    assert!(document.contains("anchor: Click me! | e_window --title \"you clicked!\" --width 400 --height 300"));
    assert!(document.contains("anchor: Open Google | https://www.google.com"));
    assert!(document.contains("anchor: Open Calculator | calc.exe"));
}

#[test]
fn test_card_with_anchors_mixed_content() {
    let card = Card::new()
        .window_title("Mixed Content Test")
        .field("status", "testing")
        .card_title("Mixed Content with Anchors")
        .header("Test Header")
        .caption("Test Caption")
        .body_line("Some content here")
        .anchor("Launch another window", "e_window --title \"spawned\" --x 100 --y 100")
        .body_line("More content after anchor")
        .body_line("")
        .anchor("Calculator", "calc");

    let document = card.to_document();
    
    println!("Generated mixed document:\n{}", document);
    
    // Verify structure
    assert!(document.contains("--title \"Mixed Content Test\""));
    assert!(document.contains("status | testing"));
    assert!(document.contains("anchor: Launch another window | e_window --title \"spawned\" --x 100 --y 100"));
    assert!(document.contains("anchor: Calculator | calc"));
}

#[test]
fn test_anchor_only_text() {
    let card = Card::new()
        .card_title("Anchor Test")
        .anchor_text("Just display text");

    let document = card.to_document();
    
    println!("Generated anchor-only document:\n{}", document);
    
    assert!(document.contains("anchor: Just display text"));
    assert!(!document.contains(" | ")); // No href separator for text-only anchors
}