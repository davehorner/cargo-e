use e_window_api::msgbox::{MessageBoxType, MessageBoxIcon, message_box_async};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Esc key behavior...");
    
    // Test with OkCancel - Esc should return "Cancel"
    let result = message_box_async(
        "Esc Key Test",
        "Press Esc to test Cancel behavior, or click OK/press Enter for OK",
        MessageBoxType::OkCancel,
        MessageBoxIcon::Question,
        None
    ).await?;
    
    println!("Result: {:?}", result);
    
    Ok(())
}