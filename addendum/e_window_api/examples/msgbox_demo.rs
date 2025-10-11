// Example: Message Box API demo - both sync and async versions
// This demonstrates the WinAPI ShowMessageBox-style interface

use e_window_api::msgbox::{
    message_box, notify, info, warn, error, ask_yes_no, confirm,
    message_box_async, notify_async,
    MessageBoxType, MessageBoxIcon, MessageBoxResult
};

// Synchronous example - no async/await needed
fn sync_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Synchronous Message Box Demo ===");
    
    // Simple notification
    println!("Calling info()...");
    info("Welcome", "Welcome to the e_window_api message box demo!")?;
    println!("info() completed.");
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Warning message
    println!("Calling warn()...");
    warn("Warning", "This is a warning message that will auto-close.")?;
    println!("warn() completed.");
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Error message
    println!("Calling error()...");
    error("Error Demo", "This simulates an error message with longer display time.")?;
    println!("error() completed.");
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Yes/No question
    println!("Calling ask_yes_no()...");
    let likes_api = ask_yes_no("Question", "Do you like this message box API?")?;
    println!("ask_yes_no() completed. User likes API: {}", likes_api);
    
    // Confirmation dialog
    println!("Calling confirm()...");
    let confirmed = confirm("Confirmation", "Do you want to continue with the demo?")?;
    println!("confirm() completed. User confirmed: {}", confirmed);
    
    if confirmed {
        // Custom message box with timeout
        println!("Calling message_box() with timeout...");
        let result = message_box(
            "Timed Dialog",
            "This dialog will auto-close in 5 seconds if you don't respond.\n\nIn a real implementation, you could press Y for Yes or N for No.",
            MessageBoxType::YesNo,
            MessageBoxIcon::Question,
            Some(5)
        )?;
        println!("message_box() completed.");
        
        match result {
            MessageBoxResult::Yes => println!("User chose Yes"),
            MessageBoxResult::No => println!("User chose No"),
            MessageBoxResult::Timeout => println!("Dialog timed out"),
            _ => println!("Other result: {:?}", result),
        }
    }
    
    // Final notification
    println!("Calling final notify()...");
    notify("Demo Complete", "Synchronous message box demo completed!", MessageBoxIcon::Information, Some(3))?;
    println!("Final notify() completed.");
    
    Ok(())
}

// Asynchronous example - for use in async contexts
async fn async_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Asynchronous Message Box Demo ===");
    
    // You can use the async versions in async contexts
    notify_async("Async Demo", "This is running in an async context!", MessageBoxIcon::Information, Some(3)).await?;
    
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    
    let result = message_box_async(
        "Async Question",
        "This is an async message box call.\n\nIt doesn't block the async runtime.",
        MessageBoxType::OkCancel,
        MessageBoxIcon::Question,
        Some(4)
    ).await?;
    
    match result {
        MessageBoxResult::Ok => println!("Async: User chose OK"),
        MessageBoxResult::Cancel => println!("Async: User chose Cancel"),
        MessageBoxResult::Timeout => println!("Async: Dialog timed out"),
        _ => println!("Async: Other result: {:?}", result),
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Message Box API Demo...\n");
    
    // Run sync demo (this works even in an async main)
    sync_demo()?;
    
    println!("\nWaiting 2 seconds before async demo...\n");
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    
    // Run async demo
    async_demo().await?;
    
    println!("\nAll demos completed!");
    Ok(())
}

// Example of using in a non-async context
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test] 
    fn test_sync_api_in_regular_code() {
        // This is how you'd use it in regular, non-async Rust code
        let result = info("Test", "This is a test notification");
        assert!(result.is_ok());
        
        // Simple confirmation
        let _confirmed = confirm("Test Confirm", "This is a test confirmation");
        // In real usage, user would interact with the dialog
    }
}