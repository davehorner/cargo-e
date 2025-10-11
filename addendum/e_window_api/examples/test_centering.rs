use e_window_api::{message_box, MessageBoxType, MessageBoxIcon, MessageBoxResult};

fn main() {
    println!("Testing auto-centering behavior...");
    
    // Test a simple message box that should be auto-centered
    let response = message_box(
        "Centered Test",
        "This message should be properly centered on screen!",
        MessageBoxType::Ok,
        MessageBoxIcon::Information,
        None
    ).expect("Failed to show message box");
    
    println!("User clicked: {:?}", response);
}