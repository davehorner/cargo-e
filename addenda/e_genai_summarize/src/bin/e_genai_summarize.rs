// src/bin/e_genai_summarize.rs

use std::env;
use std::fs;
use std::path::Path;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect command-line arguments.
    let args: Vec<String> = env::args().collect();

    // If no arguments are provided, use the program's own source code.
    // Otherwise, read the source content from the provided origin path.
    let (origin_path, content) = if args.len() < 2 {
        // Use a relative path to the current file.
        ("self", include_str!("e_genai_summarize.rs").to_string())
    } else {
        let origin_path = &args[1];
        let path = Path::new(origin_path);
        let content = fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("Error reading {}: {}", origin_path, err);
            std::process::exit(1);
        });
        (origin_path.as_str(), content)
    };

    // Create a prompt that asks for a summary focused on crates used, safety, and functionality.
    let prompt = format!(
        "Please analyze the following Rust source code and summarize:
- Which crates are used.
- Whether it appears safe to run.
- What its main functionality is.
---\n\n{}",
        content
    );

    // Prepare the chat request.
    let client = Client::default();
    let mut chat_req = ChatRequest::default().with_system("You are a Rust code analyst.");
    chat_req = chat_req.append_message(ChatMessage::user(prompt));

    // Use your preferred model (for example, "gpt-4o-mini").
    let model = "gpt-4o-mini";
    let response = client.exec_chat(model, chat_req, None).await?;


    if let Some(ref content) = response.content {
    // First, convert the content to a string (using Debug for now)
    let debug_str = format!("{:?}", content);
    // Attempt to parse the debug string as JSON to unescape it
    let unescaped: String = serde_json::from_str(&debug_str).unwrap_or_else(|_| {
        // If parsing fails, fall back to a manual replacement.
        debug_str.replace("\\n", "\n")
    });
    println!("Summary:\n{}", unescaped);
} else {
    println!("No summary text received.");
}

    Ok(())
}
