use e_window_api::Card;

#[test]
fn test_expiration_field() {
    let card = Card::new()
        .card_title("Auto-Close Test")
        .expiration("auto_close", 30)  // 30 seconds
        .body_line("This window will auto-close in 30 seconds");

    let document = card.to_document();
    
    println!("Generated document with expiration:\n{}", document);
    
    // Verify expiration field is correctly formatted
    assert!(document.contains("auto_close | 30 | expiration"));
}

#[test]
fn test_duration_and_datetime_fields() {
    let card = Card::new()
        .card_title("Time Fields Test")
        .duration_field("elapsed", "00:05:30.123")
        .datetime_field("started", "2024-10-11 15:30:45.123")
        .body_line("Testing time-related field types");

    let document = card.to_document();
    
    println!("Generated document with time fields:\n{}", document);
    
    // Verify field types are correctly set
    assert!(document.contains("elapsed | 00:05:30.123 | duration"));
    assert!(document.contains("started | 2024-10-11 15:30:45.123 | datetime"));
}

#[test]
fn test_mixed_field_types_with_expiration() {
    let card = Card::new()
        .window_title("Mixed Fields Test")
        .field("status", "running")
        .expiration("timeout", 60)  // 1 minute
        .duration_field("uptime", "00:02:15.456")
        .datetime_field("created", "2024-10-11 15:28:30.000")
        .card_title("Comprehensive Field Types Test")
        .header("Testing all field types together")
        .body_line("String field: status")
        .body_line("Expiration field: timeout (countdown)")
        .body_line("Duration field: uptime (elapsed)")
        .body_line("Datetime field: created (timestamp)");

    let document = card.to_document();
    
    println!("Generated comprehensive document:\n{}", document);
    
    // Verify all field types
    assert!(document.contains("status | running | string"));
    assert!(document.contains("timeout | 60 | expiration"));
    assert!(document.contains("uptime | 00:02:15.456 | duration"));
    assert!(document.contains("created | 2024-10-11 15:28:30.000 | datetime"));
    assert!(document.contains("--title \"Mixed Fields Test\""));
}

#[test]
fn test_multiple_expiration_fields() {
    let card = Card::new()
        .card_title("Multiple Expiration Test")
        .expiration("warning_timeout", 10)    // 10 seconds warning
        .expiration("final_timeout", 30)      // 30 seconds final close
        .body_line("Warning in 10s, close in 30s");

    let document = card.to_document();
    
    println!("Generated document with multiple expirations:\n{}", document);
    
    // Both expiration fields should be present
    assert!(document.contains("warning_timeout | 10 | expiration"));
    assert!(document.contains("final_timeout | 30 | expiration"));
}