// Example: Pure synchronous usage (no async/await required)
// This shows how to use e_window_api in traditional, non-async Rust projects

use e_window_api::msgbox::{info, warn, error, ask_yes_no, confirm, MessageBoxIcon, notify};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Pure Synchronous e_window_api Demo");
    println!("===================================\n");
    
    // This is just like WinAPI MessageBox - simple and blocking
    info("Application Started", "The application has started successfully!")?;
    
    // Simulate some work
    println!("Doing some work...");
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Warning about something
    warn("Low Memory", "System memory is running low. Consider closing other applications.")?;
    
    // Simulate more work
    println!("Continuing work...");
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Ask user a question
    let save_work = ask_yes_no("Save Work", "Do you want to save your work before continuing?")?;
    
    if save_work {
        println!("User chose to save work");
        
        // Simulate saving
        notify("Saving", "Saving your work...", MessageBoxIcon::Information, Some(2))?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        info("Save Complete", "Your work has been saved successfully!")?;
    } else {
        println!("User chose not to save");
        
        // Double-check with confirmation
        let really_no_save = confirm("Confirm", "Are you sure you don't want to save? All changes will be lost.")?;
        
        if really_no_save {
            warn("Not Saved", "Continuing without saving. Changes will be lost.")?;
        } else {
            // User changed their mind
            notify("Saving", "Changed your mind? Saving now...", MessageBoxIcon::Information, Some(2))?;
            info("Save Complete", "Your work has been saved successfully!")?;
        }
    }
    
    // Simulate potential error condition
    println!("Checking system status...");
    std::thread::sleep(std::time::Duration::from_millis(300));
    
    let has_error = false; // Simulate no error for demo
    if has_error {
        error("Critical Error", "A critical error has occurred. The application will now exit.")?;
        return Err("Critical error occurred".into());
    }
    
    // Success completion
    info("Application Complete", "All operations completed successfully!")?;
    
    println!("\nDemo completed!");
    Ok(())
}

// This shows how you might use it in different contexts:

/// Example: Error handling in a regular function
fn process_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Processing file: {}", filename);
    
    // Simulate file processing
    if filename.is_empty() {
        error("Invalid File", "Filename cannot be empty!")?;
        return Err("Invalid filename".into());
    }
    
    if filename.ends_with(".tmp") {
        let continue_anyway = ask_yes_no(
            "Temporary File",
            "This appears to be a temporary file. Do you want to process it anyway?"
        )?;
        
        if !continue_anyway {
            info("Cancelled", "File processing cancelled by user.")?;
            return Ok(());
        }
    }
    
    // Simulate processing
    notify("Processing", "Processing file, please wait...", MessageBoxIcon::Information, Some(3))?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    info("Success", &format!("File '{}' processed successfully!", filename))?;
    Ok(())
}

/// Example: User workflow with multiple decisions
fn user_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // Welcome message
    info("Workflow Started", "Starting the user workflow process...")?;
    
    // Step 1: Get user preference
    let use_advanced = ask_yes_no(
        "Workflow Options",
        "Do you want to use advanced options? (No = use simple mode)"
    )?;
    
    if use_advanced {
        notify("Advanced Mode", "Advanced mode selected. More options available.", MessageBoxIcon::Information, Some(2))?;
        
        // Advanced workflow
        let backup_first = confirm("Backup", "Create a backup before proceeding?")?;
        if backup_first {
            info("Backup Created", "Backup created successfully.")?;
        }
        
    } else {
        notify("Simple Mode", "Simple mode selected. Using default settings.", MessageBoxIcon::Information, Some(2))?;
    }
    
    // Final confirmation
    let proceed = confirm("Final Confirmation", "Ready to proceed with the workflow?")?;
    if proceed {
        info("Workflow Complete", "Workflow completed successfully!")?;
    } else {
        warn("Workflow Cancelled", "Workflow was cancelled by user.")?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_processing() {
        // In a real test, you might mock the user interactions
        let result = process_file("test.txt");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_empty_filename() {
        let result = process_file("");
        // This would show an error dialog and return an error
        assert!(result.is_err());
    }
}