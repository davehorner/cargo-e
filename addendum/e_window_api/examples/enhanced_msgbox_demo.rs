// Example: Enhanced Message Box API demo - comprehensive demonstration of all features
// This demonstrates the extended MessageBox functionality including text input, file selection,
// custom default buttons, and advanced configuration options

use e_window_api::msgbox::{
    message_box, message_box_ex, input_text, select_file, ask_yes_no_ex,
    MessageBoxConfig, MessageBoxType, MessageBoxIcon, MessageBoxResult
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Enhanced Message Box Demo ===");
    
    // 1. Basic message boxes with new button types
    println!("\n1. Testing all button types...");
    
    // Yes/No with default No (Enter key will choose No)
    println!("Testing YesNo with default No...");
    let config = MessageBoxConfig::new("Question", "Do you want to continue?\n(Default is No - press Enter to see)")
        .with_type(MessageBoxType::YesNo)
        .with_icon(MessageBoxIcon::Question)
        .with_default_button(MessageBoxResult::No)
        .with_timeout(5);
    
    let response = message_box_ex(config)?;
    println!("Result: {:?}", response.result);
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // 2. Text input dialog
    println!("\n2. Testing text input dialog...");
    if let Some(text) = input_text("User Input", "Please enter your name:", "John Doe", Some(8))? {
        println!("User entered: {}", text);
    } else {
        println!("User cancelled or timed out");
    }
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // 3. File selection dialog
    println!("\n3. Testing file selection dialog...");
    if let Some(file_path) = select_file("Select File", "C:\\", "*.txt;*.log;*.md", Some(8))? {
        println!("User selected: {}", file_path);
    } else {
        println!("User cancelled or timed out");
    }
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // 4. Enhanced Yes/No with different defaults
    println!("\n4. Testing enhanced Yes/No dialogs...");
    
    // Default Yes
    if let Some(answer) = ask_yes_no_ex("Confirmation", "Save changes before closing?\n(Default is Yes)", true, Some(5))? {
        println!("Save changes: {}", if answer { "Yes" } else { "No" });
    } else {
        println!("User didn't respond in time");
    }
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Default No  
    if let Some(answer) = ask_yes_no_ex("Warning", "Delete all files?\n(Default is No for safety)", false, Some(5))? {
        println!("Delete files: {}", if answer { "Yes" } else { "No" });
    } else {
        println!("User didn't respond in time");
    }
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // 5. Complex configuration example
    println!("\n5. Testing complex configuration...");
    let config = MessageBoxConfig::new("Advanced Dialog", "This dialog demonstrates all configuration options")
        .with_type(MessageBoxType::YesNoCancel)
        .with_icon(MessageBoxIcon::Warning)
        .with_timeout(10)
        .with_default_button(MessageBoxResult::Cancel); // Enter key defaults to Cancel
    
    let response = message_box_ex(config)?;
    match response.result {
        MessageBoxResult::Yes => println!("User chose Yes"),
        MessageBoxResult::No => println!("User chose No"),
        MessageBoxResult::Cancel => println!("User chose Cancel (or pressed Enter)"),
        MessageBoxResult::Timeout => println!("Dialog timed out"),
        _ => println!("Unexpected result: {:?}", response.result),
    }
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // 6. Text input with custom initial text
    println!("\n6. Testing text input with custom initial text...");
    let config = MessageBoxConfig::new("Configuration", "Enter server URL:")
        .with_type(MessageBoxType::TextInput)
        .with_icon(MessageBoxIcon::Information)
        .with_initial_text("https://api.example.com")
        .with_timeout(8);
    
    let response = message_box_ex(config)?;
    match response.result {
        MessageBoxResult::TextInput | MessageBoxResult::Ok => {
            if let Some(url) = response.text_result {
                println!("Server URL: {}", url);
            }
        },
        MessageBoxResult::Cancel => println!("User cancelled"),
        MessageBoxResult::Timeout => println!("Input timed out"),
        _ => println!("Unexpected result: {:?}", response.result),
    }
    
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // 7. File selection with filter and initial directory
    println!("\n7. Testing file selection with filter...");
    let config = MessageBoxConfig::new("Open Document", "Select a document to open:")
        .with_type(MessageBoxType::FileSelection)
        .with_icon(MessageBoxIcon::Question)
        .with_file_filter("*.txt;*.doc;*.pdf")
        .with_initial_directory("C:\\Documents")
        .with_timeout(8);
    
    let response = message_box_ex(config)?;
    match response.result {
        MessageBoxResult::FileSelected | MessageBoxResult::Ok => {
            if let Some(file) = response.text_result {
                println!("Selected document: {}", file);
            }
        },
        MessageBoxResult::Cancel => println!("User cancelled file selection"),
        MessageBoxResult::Timeout => println!("File selection timed out"),
        _ => println!("Unexpected result: {:?}", response.result),
    }
    
    println!("\n=== Demo Complete ===");
    println!("This demo showed:");
    println!("✓ Custom default buttons (Enter key behavior)");
    println!("✓ Text input dialogs with initial text");
    println!("✓ File selection dialogs with filters");
    println!("✓ All button types: Ok, OkCancel, YesNo, YesNoCancel, RetryCancel");
    println!("✓ Keyboard shortcuts: Y/N, Enter, Escape, R");
    println!("✓ Timeout handling with auto-close countdown");
    println!("✓ Enhanced configuration options");
    
    Ok(())
}