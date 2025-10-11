// Example: Simple Message Box Demo
// This demonstrates the WinAPI ShowMessageBox-style interface in a pure sync context

use e_window_api::msgbox::{info, warn, error, ask_yes_no, confirm, notify, MessageBoxIcon};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== e_window_api Message Box Demo ===\n");
    
    // This is just like calling MessageBox() in WinAPI - simple and synchronous!
    info("Welcome", "Welcome to the e_window_api message box demo!")?;
    
    // Different message types
    notify("Processing", "Starting demo sequence...", MessageBoxIcon::Information, Some(2))?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    warn("Warning", "This is a warning message demonstration.")?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    error("Error Demo", "This demonstrates an error message (auto-closes after 7 seconds).")?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Interactive dialogs
    let likes_api = ask_yes_no("Question", "Do you like this MessageBox-style API?")?;
    println!("User response to 'Do you like this API?': {}", if likes_api { "Yes" } else { "No" });
    
    if likes_api {
        let continue_demo = confirm("Continue", "Great! Would you like to see more features?")?;
        println!("User wants to continue: {}", continue_demo);
        
        if continue_demo {
            notify("Feature Demo", "Here are some key features:\n\n• WinAPI MessageBox-style interface\n• Works in sync code (no async/await needed)\n• Auto-sizing and positioning\n• Multiple dialog types\n• Cross-platform", MessageBoxIcon::Information, Some(5))?;
        }
    } else {
        let give_feedback = confirm("Feedback", "No problem! Would you like to provide feedback?")?;
        if give_feedback {
            notify("Feedback", "Thank you for your feedback!\n\nPlease let us know how we can improve the API.", MessageBoxIcon::Information, Some(4))?;
        }
    }
    
    // Final message
    info("Demo Complete", "Thank you for trying the e_window_api message box interface!")?;
    
    println!("\nDemo completed successfully!");
    println!("Key features demonstrated:");
    println!("✅ Synchronous API (no async/await required)");
    println!("✅ WinAPI MessageBox-style interface");
    println!("✅ Multiple dialog types (info, warn, error, confirm, ask)");
    println!("✅ Auto-positioning and sizing");
    println!("✅ Cross-platform compatibility");
    
    Ok(())
}