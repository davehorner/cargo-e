// Example: Demonstrating shared msgbox types between e_window and e_window_api
// This shows how the shared types work across both crates

use e_window_api::{MessageBoxType, MessageBoxIcon, MessageBoxResult, message_box};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Shared Message Box Types Demo ===");
    
    // These types are now shared between e_window and e_window_api
    // They're defined in e_window::msgbox_types and re-exported by e_window_api
    
    println!("Available MessageBoxType variants:");
    println!("  - MessageBoxType::Ok");
    println!("  - MessageBoxType::OkCancel");
    println!("  - MessageBoxType::YesNo");
    println!("  - MessageBoxType::YesNoCancel");
    println!("  - MessageBoxType::RetryCancel");
    
    println!("\nAvailable MessageBoxIcon variants:");
    println!("  - MessageBoxIcon::None");
    println!("  - MessageBoxIcon::Information (ℹ️)");
    println!("  - MessageBoxIcon::Warning (⚠️)");
    println!("  - MessageBoxIcon::Error (❌)");
    println!("  - MessageBoxIcon::Question (❓)");
    
    println!("\nAvailable MessageBoxResult variants:");
    println!("  - MessageBoxResult::Ok");
    println!("  - MessageBoxResult::Cancel");
    println!("  - MessageBoxResult::Yes");
    println!("  - MessageBoxResult::No");
    println!("  - MessageBoxResult::Retry");
    println!("  - MessageBoxResult::Timeout");
    
    // Demonstrate using the types
    let result = message_box(
        "Shared Types Demo",
        "These types are shared between e_window and e_window_api!\n\nBoth crates use the same enum definitions.",
        MessageBoxType::OkCancel,
        MessageBoxIcon::Information,
        Some(5)
    )?;
    
    match result {
        MessageBoxResult::Ok => println!("\n✅ User clicked OK"),
        MessageBoxResult::Cancel => println!("\n❌ User clicked Cancel"),
        MessageBoxResult::Timeout => println!("\n⏱️ Dialog timed out"),
        _ => println!("\n🤔 Unexpected result: {:?}", result),
    }
    
    println!("\n🎉 Shared types demo completed!");
    println!("These same types can be used by e_window itself when");
    println!("implementing msgbox functionality in the future.");
    
    Ok(())
}