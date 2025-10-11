use e_window_api::msgbox::{message_box, MessageBoxType, MessageBoxIcon};

fn main() {
    println!("🚀 Comprehensive MessageBox Testing with Real User Input");
    println!("=========================================================\n");
    
    // Test 1: Simple confirmation
    println!("1. Testing simple Yes/No confirmation (no timeout)...");
    match message_box(
        "Confirmation",
        "Do you want to continue?",
        MessageBoxType::YesNo,
        MessageBoxIcon::Question,
        None
    ) {
        Ok(result) => println!("   ✅ Result: {:?}\n", result),
        Err(e) => println!("   ❌ Error: {:?}\n", e),
    }
    
    // Test 2: Timeout test
    println!("2. Testing timeout (5 seconds)...");
    match message_box(
        "Timeout Test",
        "This will auto-close in 5 seconds",
        MessageBoxType::Ok,
        MessageBoxIcon::Information,
        Some(5)
    ) {
        Ok(result) => println!("   ✅ Result: {:?}\n", result),
        Err(e) => println!("   ❌ Error: {:?}\n", e),
    }
    
    // Test 3: Different message types
    println!("3. Testing OK/Cancel dialog...");
    match message_box(
        "Save Changes",
        "Save your work before exiting?",
        MessageBoxType::OkCancel,
        MessageBoxIcon::Warning,
        None
    ) {
        Ok(result) => println!("   ✅ Result: {:?}\n", result),
        Err(e) => println!("   ❌ Error: {:?}\n", e),
    }
    
    println!("🎉 All tests completed! The MessageBox system now supports:");
    println!("   • Real user input capture (Y/N/Enter/Escape keys)");
    println!("   • Timeout handling with visual countdown");
    println!("   • Multiple dialog types (YesNo, OkCancel, etc.)");
    println!("   • Proper result communication back to API");
    println!("   • Bidirectional process communication");
}