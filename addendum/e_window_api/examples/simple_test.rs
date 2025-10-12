use e_window_api::msgbox::{message_box, MessageBoxIcon, MessageBoxResult, MessageBoxType};

fn main() {
    println!("Testing simple MessageBox with real user input...");

    // Test a simple Yes/No dialog
    match message_box(
        "Test Dialog",
        "Press Y for Yes or N for No (no timeout)",
        MessageBoxType::YesNo,
        MessageBoxIcon::Question,
        None, // No timeout - wait for user input
    ) {
        Ok(result) => {
            println!("User choice: {:?}", result);
            match result {
                MessageBoxResult::Yes => println!("✅ User chose Yes!"),
                MessageBoxResult::No => println!("❌ User chose No!"),
                MessageBoxResult::Timeout => println!("⏰ Timed out (shouldn't happen)"),
                other => println!("🤔 Unexpected result: {:?}", other),
            }
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
        }
    }

    println!("\nTest completed!");
}
