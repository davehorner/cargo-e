use e_window_api::{AnimationEasing, Card, ControlCommand, WindowConfig};

#[test]
fn test_window_config_creation() {
    let config = WindowConfig::new()
        .title("Test Window")
        .size(800, 600)
        .position(100, 100)
        .decode_debug(true);

    assert_eq!(config.title, "Test Window");
    assert_eq!(config.width, 800);
    assert_eq!(config.height, 600);
    assert_eq!(config.x, 100);
    assert_eq!(config.y, 100);
    assert_eq!(config.decode_debug, true);
}

#[test]
fn test_window_config_to_args() {
    let config = WindowConfig::new()
        .title("Test")
        .size(400, 300)
        .position(50, 50);

    let args = config.to_args();

    // Check that required args are present
    assert!(args.contains(&"--title".to_string()));
    assert!(args.contains(&"Test".to_string()));
    assert!(args.contains(&"--width".to_string()));
    assert!(args.contains(&"400".to_string()));
    assert!(args.contains(&"--height".to_string()));
    assert!(args.contains(&"300".to_string()));
    assert!(args.contains(&"--x".to_string()));
    assert!(args.contains(&"50".to_string()));
    assert!(args.contains(&"--y".to_string()));
    assert!(args.contains(&"50".to_string()));
}

#[test]
fn test_card_creation() {
    let card = Card::new()
        .window_title("Test Card")
        .size(400, 300)
        .position(100, 100)
        .field("name", "test")
        .field("version", "1.0")
        .card_title("Hello World")
        .header("Test Header")
        .caption("Test Caption")
        .body_line("Additional line");

    assert_eq!(card.window_title, Some("Test Card".to_string()));
    assert_eq!(card.width, Some(400));
    assert_eq!(card.height, Some(300));
    assert_eq!(card.x, Some(100));
    assert_eq!(card.y, Some(100));
    assert_eq!(card.title, Some("Hello World".to_string()));
    assert_eq!(card.header, Some("Test Header".to_string()));
    assert_eq!(card.caption, Some("Test Caption".to_string()));
    assert_eq!(card.body.len(), 1);
    assert_eq!(card.body[0], "Additional line");
    assert_eq!(card.fields.len(), 2);
}

#[test]
fn test_card_to_document() {
    let card = Card::new()
        .window_title("Test")
        .size(400, 300)
        .field("name", "test")
        .card_title("Hello")
        .body_line("World");

    let doc = card.to_document();

    // Should contain window config
    assert!(doc.contains("--title \"Test\""));
    assert!(doc.contains("--width 400"));
    assert!(doc.contains("--height 300"));

    // Should contain fields
    assert!(doc.contains("name | test | string"));

    // Should contain content
    assert!(doc.contains("Hello"));
    assert!(doc.contains("World"));
}

#[test]
fn test_control_command_formatting() {
    let cmd = ControlCommand::SetTitle("New Title".to_string());
    assert_eq!(cmd.to_control_string(), "!control: set_title New Title");

    let cmd = ControlCommand::SetRect {
        x: 100,
        y: 200,
        width: 300,
        height: 400,
    };
    assert_eq!(
        cmd.to_control_string(),
        "!control: set_rect 100 200 300 400"
    );

    let cmd = ControlCommand::SetRectEased {
        x: 0,
        y: 0,
        width: 800,
        height: 600,
        duration_ms: 1000,
        easing: AnimationEasing::Linear,
    };
    assert_eq!(
        cmd.to_control_string(),
        "!control: set_rect_eased 0 0 800 600 1000 linear"
    );

    let cmd = ControlCommand::Delay(500);
    assert_eq!(cmd.to_control_string(), "!control: delay 500");

    let cmd = ControlCommand::Exit;
    assert_eq!(cmd.to_control_string(), "!control: exit");
}

#[test]
fn test_animation_easing_display() {
    assert_eq!(format!("{}", AnimationEasing::Linear), "linear");
    assert_eq!(format!("{}", AnimationEasing::EaseIn), "ease-in");
    assert_eq!(format!("{}", AnimationEasing::EaseOut), "ease-out");
    assert_eq!(format!("{}", AnimationEasing::EaseInOut), "ease-in-out");
    assert_eq!(format!("{}", AnimationEasing::Bounce), "bounce");
    assert_eq!(format!("{}", AnimationEasing::Elastic), "elastic");
    assert_eq!(format!("{}", AnimationEasing::Back), "back");
    assert_eq!(
        format!("{}", AnimationEasing::Custom("custom".to_string())),
        "custom"
    );
}

#[test]
fn test_card_anchors() {
    let card = Card::new()
        .card_title("Test with Anchors")
        .body_line("This demonstrates anchor functionality")
        .anchor(
            "Click me!",
            "e_window --title \"Clicked!\" --width 400 --height 300",
        )
        .anchor("Open URL", "https://www.example.com")
        .anchor_text("Text only anchor");

    let document = card.to_document();

    // Verify anchors are in the document
    assert!(document
        .contains("anchor: Click me! | e_window --title \"Clicked!\" --width 400 --height 300"));
    assert!(document.contains("anchor: Open URL | https://www.example.com"));
    assert!(document.contains("anchor: Text only anchor"));
    assert!(!document.contains("anchor: Text only anchor |")); // No href for text-only
}

#[test]
fn test_card_with_anchors_and_fields() {
    let card = Card::new()
        .window_title("Anchor Test Window")
        .field("status", "active")
        .field("mode", "test")
        .card_title("Integration Test")
        .header("Testing anchors with other content")
        .body_line("This card has fields AND anchors")
        .anchor("Launch calc", "calc.exe")
        .body_line("More content after anchor");

    let document = card.to_document();

    // Verify structure
    assert!(document.contains("--title \"Anchor Test Window\""));
    assert!(document.contains("status | active"));
    assert!(document.contains("mode | test"));
    assert!(document.contains("Integration Test"));
    assert!(document.contains("Testing anchors with other content"));
    assert!(document.contains("This card has fields AND anchors"));
    assert!(document.contains("anchor: Launch calc | calc.exe"));
    assert!(document.contains("More content after anchor"));
}
