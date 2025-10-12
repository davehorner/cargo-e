use e_window_api::msgbox::{message_box, MessageBoxIcon, MessageBoxResult, MessageBoxType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing button click communication...");

    // Simple yes/no test with timeout
    println!("Showing Yes/No dialog - please click a button (will timeout in 10 seconds)");

    let result = message_box(
        "Button Test",
        "Please click YES or NO to test button communication (5 sec timeout)",
        MessageBoxType::YesNo,
        MessageBoxIcon::Question,
        Some(5), // 5 second timeout
    )?;

    match result {
        MessageBoxResult::Yes => {
            println!("✅ SUCCESS: Button click for YES was properly received!");
            Ok(())
        }
        MessageBoxResult::No => {
            println!("✅ SUCCESS: Button click for NO was properly received!");
            Ok(())
        }
        MessageBoxResult::Timeout => {
            println!("⏰ TIMEOUT: No button was clicked within the timeout period");
            Ok(())
        }
        other => {
            println!("📝 INFO: Received result: {:?}", other);
            Ok(())
        }
    }
}
