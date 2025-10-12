//! Test the new msgbox API with emoji prefixes and optimal sizing

use e_window_api::msgbox::{message_box, MessageBoxIcon, MessageBoxResult, MessageBoxType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing new msgbox API with emoji prefixes...\n");

    // Test OK button with automatic emoji
    println!("1. Testing OK message (should show ✅ emoji)...");
    let result = message_box(
        "Success",
        "Operation completed successfully!",
        MessageBoxType::Ok,
        MessageBoxIcon::None, // Let it use automatic emoji
        None,
    )?;
    println!("Result: {:?}\n", result);

    // Test Yes/No with automatic emoji
    println!("2. Testing Yes/No question (should show ❓ emoji)...");
    let result = message_box(
        "Question",
        "Do you want to continue?",
        MessageBoxType::YesNo,
        MessageBoxIcon::None, // Let it use automatic emoji
        None,
    )?;
    println!("Result: {:?}\n", result);

    // Test Retry with automatic emoji
    println!("3. Testing Retry/Cancel (should show 🔄 emoji)...");
    let result = message_box(
        "Retry",
        "The operation failed. Would you like to retry?",
        MessageBoxType::RetryCancel,
        MessageBoxIcon::None, // Let it use automatic emoji
        None,
    )?;
    println!("Result: {:?}\n", result);

    // Test Text Input with automatic emoji
    println!("4. Testing Text Input (should show 📝 emoji)...");
    let result = message_box(
        "Input Required",
        "Please enter your name:",
        MessageBoxType::TextInput,
        MessageBoxIcon::None, // Let it use automatic emoji
        None,
    )?;
    println!("Result: {:?}\n", result);

    // Test File Selection with automatic emoji
    println!("5. Testing File Selection (should show 📁 emoji)...");
    let result = message_box(
        "File Selection",
        "Please choose a file:",
        MessageBoxType::FileSelection,
        MessageBoxIcon::None, // Let it use automatic emoji
        None,
    )?;
    println!("Result: {:?}\n", result);

    println!("All tests completed!");
    Ok(())
}
