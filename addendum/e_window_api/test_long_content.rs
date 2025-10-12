use e_window_api::msgbox::{message_box_async, MessageBoxIcon, MessageBoxType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing with very long content...");

    // Test with very long content to force different sizing
    let long_message =
        "This is a much longer message to test dynamic sizing with extensive content. ".repeat(10);

    let result = message_box_async(
        "Very Long Content Test",
        &long_message,
        MessageBoxType::YesNoCancel,
        MessageBoxIcon::Question,
        Some(30),
    )
    .await?;

    println!("Result: {:?}", result);

    Ok(())
}
