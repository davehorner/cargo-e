// Example: Keyboard handling test for message boxes
// This demonstrates keyboard shortcuts for all message box types

use e_window_api::msgbox::{
    message_box_ex, MessageBoxConfig, MessageBoxIcon, MessageBoxResult, MessageBoxType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Keyboard Shortcuts Test ===");
    println!("This demo tests keyboard shortcuts for all message box types.");
    println!("Try these keys during each dialog:");
    println!("- Enter: Default button");
    println!("- Escape: Cancel/No");
    println!("- Y: Yes (in Yes/No dialogs)");
    println!("- N: No (in Yes/No dialogs)");
    println!("- R: Retry (in RetryCancel dialogs)");
    println!();

    // 1. YesNo with default Yes
    println!("1. YesNo dialog (default Yes) - try Y, N, or Enter keys...");
    let config = MessageBoxConfig::new(
        "Test 1",
        "Yes/No with default Yes\n\nPress Y for Yes, N for No, Enter for default (Yes)",
    )
    .with_type(MessageBoxType::YesNo)
    .with_icon(MessageBoxIcon::Question)
    .with_default_button(MessageBoxResult::Yes)
    .with_timeout(15);

    let response = message_box_ex(config)?;
    println!("Result: {:?}\n", response.result);

    // 2. YesNo with default No
    println!("2. YesNo dialog (default No) - try Y, N, or Enter keys...");
    let config = MessageBoxConfig::new(
        "Test 2",
        "Yes/No with default No\n\nPress Y for Yes, N for No, Enter for default (No)",
    )
    .with_type(MessageBoxType::YesNo)
    .with_icon(MessageBoxIcon::Question)
    .with_default_button(MessageBoxResult::No)
    .with_timeout(15);

    let response = message_box_ex(config)?;
    println!("Result: {:?}\n", response.result);

    // 3. YesNoCancel
    println!("3. YesNoCancel dialog - try Y, N, Enter, or Escape keys...");
    let config = MessageBoxConfig::new("Test 3", "Yes/No/Cancel dialog\n\nPress Y for Yes, N for No, Enter for default (Cancel), Escape for Cancel")
        .with_type(MessageBoxType::YesNoCancel)
        .with_icon(MessageBoxIcon::Question)
        .with_default_button(MessageBoxResult::Cancel)
        .with_timeout(15);

    let response = message_box_ex(config)?;
    println!("Result: {:?}\n", response.result);

    // 4. OkCancel
    println!("4. OkCancel dialog - try Enter for OK, Escape for Cancel...");
    let config = MessageBoxConfig::new(
        "Test 4",
        "OK/Cancel dialog\n\nPress Enter for OK, Escape for Cancel",
    )
    .with_type(MessageBoxType::OkCancel)
    .with_icon(MessageBoxIcon::Information)
    .with_timeout(15);

    let response = message_box_ex(config)?;
    println!("Result: {:?}\n", response.result);

    // 5. RetryCancel
    println!("5. RetryCancel dialog - try R for Retry, Escape for Cancel...");
    let config = MessageBoxConfig::new(
        "Test 5",
        "Retry/Cancel dialog\n\nPress R for Retry, Escape for Cancel, Enter for default (Retry)",
    )
    .with_type(MessageBoxType::RetryCancel)
    .with_icon(MessageBoxIcon::Warning)
    .with_timeout(15);

    let response = message_box_ex(config)?;
    println!("Result: {:?}\n", response.result);

    // 6. TextInput
    println!("6. TextInput dialog - try typing and Enter to confirm, Escape to cancel...");
    let config = MessageBoxConfig::new(
        "Test 6",
        "Text Input dialog\n\nType some text and press Enter to confirm, Escape to cancel",
    )
    .with_type(MessageBoxType::TextInput)
    .with_icon(MessageBoxIcon::Question)
    .with_initial_text("Type here...")
    .with_timeout(20);

    let response = message_box_ex(config)?;
    println!("Result: {:?}", response.result);
    if let Some(text) = response.text_result {
        println!("Text entered: {}\n", text);
    }

    // 7. FileSelection
    println!("7. FileSelection dialog - try editing path and Enter to select, Escape to cancel...");
    let config = MessageBoxConfig::new(
        "Test 7",
        "File Selection dialog\n\nEdit the path and press Enter to select, Escape to cancel",
    )
    .with_type(MessageBoxType::FileSelection)
    .with_icon(MessageBoxIcon::Question)
    .with_initial_directory("C:\\temp\\example.txt")
    .with_file_filter("*.txt;*.log")
    .with_timeout(20);

    let response = message_box_ex(config)?;
    println!("Result: {:?}", response.result);
    if let Some(file) = response.text_result {
        println!("File selected: {}\n", file);
    }

    println!("=== Keyboard Test Complete ===");
    println!("All keyboard shortcuts have been tested!");

    Ok(())
}
