use e_window_api::msgbox::{message_box_async, MessageBoxIcon, MessageBoxType};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let message_box_type = args
        .iter()
        .position(|arg| arg == "--type")
        .and_then(|pos| args.get(pos + 1))
        .map(|t| MessageBoxType::from_str(t).unwrap_or(MessageBoxType::Ok))
        .unwrap_or(MessageBoxType::Ok);

    let title = args
        .iter()
        .position(|arg| arg == "--title")
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.to_string())
        .unwrap_or("Dynamic Sizing Test".to_string());

    let text = args.iter()
        .position(|arg| arg == "--text")
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.to_string())
        .unwrap_or(
            "This is a test of the dynamic sizing functionality. The window should automatically size itself based on content and center properly on the screen, regardless of screen resolution.".to_string());

    println!("Testing dynamic sizing...");

    let result = message_box_async(
        &title,
        &text,
        message_box_type,
        MessageBoxIcon::Information,
        None,
    )
    .await?;

    println!("Result: {:?}", result);

    Ok(())
}
