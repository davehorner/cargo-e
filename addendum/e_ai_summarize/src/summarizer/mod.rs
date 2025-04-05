//! e_ai_summarize: A GenAI-powered Rust source code summarizer.
//!
//! This crate analyzes Rust source code to summarize its main functionality,
//! used crates, safety (including file operations), and any notable issues.

use anyhow::Context;

use genai::Client;
use genai::chat::printer::PrintChatStreamOptions;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use include_dir::{Dir, include_dir};
use path_slash::PathBufExt;
use std::env;
use std::fs;
use std::path::Path;

use crate::cargo_utils::find_cargo_toml;
use crate::cargo_utils::get_crate_name_and_version;
use crate::sanitize;
static SRC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src");

pub struct ChatSession {
    client: Client,
    model: String,
    chat_req: ChatRequest,
    streaming: bool,
    print_options: PrintChatStreamOptions,
}

impl Default for ChatSession {
    fn default() -> Self {
        Self {
            client: Client::default(),
            model: "gpt-4o-mini".to_string(),
            chat_req: ChatRequest::new(vec![ChatMessage::system("You are a Rust code analyst.")]),
            streaming: false,
            print_options: PrintChatStreamOptions::from_print_events(false),
        }
    }
}

impl ChatSession {
    /// Creates a new chat session with the given system prompt, model, and streaming flag.
    pub fn new(system_prompt: &str, model: &str, streaming: bool) -> Self {
        Self {
            client: Client::default(),
            model: model.to_string(),
            chat_req: ChatRequest::new(vec![ChatMessage::system(system_prompt)]),
            streaming,
            print_options: PrintChatStreamOptions::from_print_events(false),
        }
    }

    /// Sets the streaming mode.
    pub fn set_streaming(&mut self, streaming: bool) {
        self.streaming = streaming;
    }

    /// Appends a user message, retrieves the assistant response using streaming or non-streaming,
    /// and appends the assistant answer to the conversation.
    pub async fn ask(&mut self, question: &str) -> anyhow::Result<String> {
        self.chat_req = self
            .chat_req
            .clone()
            .append_message(ChatMessage::user(question));

        let answer = if self.streaming {
            // Streaming call: uses print_chat_stream to display as stream and returns a String.
            let chat_res = self
                .client
                .exec_chat_stream(&self.model, self.chat_req.clone(), None)
                .await?;
            print_chat_stream(chat_res, Some(&self.print_options)).await?
        } else {
            // Non-streaming call: waits for complete answer.
            let chat_res = self
                .client
                .exec_chat(&self.model, self.chat_req.clone(), None)
                .await?;
            let resp = chat_res
                .content_text_as_str()
                .unwrap_or("NO ANSWER")
                .to_string();
            println!("{}", resp);
            resp
        };

        self.chat_req = self
            .chat_req
            .clone()
            .append_message(ChatMessage::assistant(&answer));
        Ok(answer)
    }
}

/// Analyzes and summarizes Rust source code.
///
/// If no file path is provided via command-line arguments, the function
/// defaults to using the source code of this file. Otherwise, it attempts
/// to read the file specified by the first argument.
///
/// # Returns
///
/// A `Result` containing the summarization as a `String` on success or an error.
pub async fn summarize_source() -> Result<String, Box<dyn std::error::Error>> {
    // Collect command-line arguments.
    let args: Vec<String> = env::args().collect();

    // Retrieve source content either from a provided file or the current file.
    let content = if args.len() < 2 {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).to_string()
    } else {
        let origin_path = &args[1];
        let path = Path::new(origin_path);
        fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("Error reading {}: {}", origin_path, err);
            std::process::exit(1);
        })
    };

    // Construct a concise prompt for summarization.
    let prompt = format!(
            "analyze the following Rust source code and summarize, be concise:
- What its main functionality is.
- Which crates are used.
- Whether it appears safe to run.  This should include a SAFE_TO_RUN: YES/NO answer to start and a brief explanation.
- Whether it performs any deletes or file modifications.  This should include a FILE_OPERATIONS: YES/NO answer to start and a brief explanation.
- If there are any notable limitations or issues.
- Any other relevant insights.
---
{}",
            content
        );

    // Prepare the chat request with a system prompt.
    let client = Client::default();
    let mut chat_req = ChatRequest::default().with_system("You are a Rust code analyst.");
    chat_req = chat_req.append_message(ChatMessage::user(prompt));

    // Use the chosen model to execute the chat request.
    let model = "gpt-4o-mini";
    // let response = client.exec_chat(model, chat_req, None).await?;
    let response = client
        .exec_chat_stream(model, chat_req.clone(), None)
        .await?;

    let assistant_answer = print_chat_stream(response, None).await?;
    Ok(assistant_answer)
}

/// Summarizes a Rust source file by reading its content, sending a summarization prompt
/// using a ChatSession, and then returning both the summary and the session (with context).
pub async fn summarize_source_session(
    file_path: Option<&str>,
    streaming: bool,
) -> Result<(String, ChatSession), Box<dyn std::error::Error>> {
    let mut crate_name = env!("CARGO_PKG_NAME").to_string();
    let mut crate_version = env!("CARGO_PKG_VERSION").to_string();

    let exe_path = env::current_exe().expect("Failed to get current exe path");
    // Read the content from the provided file or fallback to this file's own source.
    let content = if let Some(fp) = file_path {
        let possible_toml = find_cargo_toml(Path::new(fp));
        if let Some(crate_toml_path) = possible_toml {
            let (name, version) =
                get_crate_name_and_version(&crate_toml_path.to_path_buf()).unwrap_or_default();
            crate_name = name;
            crate_version = version;
        }
        let path = Path::new(fp);
        if path.is_dir() {
            let files = crate::cargo_utils::gather_files_from_crate(&path.to_string_lossy(), false)
                .with_context(|| {
                    format!(
                        "Failed to gather files from crate at {}",
                        &path.to_string_lossy()
                    )
                })?;
            generate_heredoc_output(&crate_name, &crate_version, &files)
        } else if path.is_file() {
            fs::read_to_string(path).unwrap_or_else(|err| {
                eprintln!("Error reading {}: {}", fp, err);
                std::process::exit(1);
            })
        } else {
            eprintln!("Error reading {}", fp);
            std::process::exit(1);
        }
    } else {
        let mut combined_source = String::new();
        combined_source.push_str(&format!("The following is a file listing from {} v{}, the primary executable is {}, demonstrate using short options"
                                          ,crate_name,crate_version,exe_path.file_name().unwrap_or_default().to_string_lossy()));
        println!(
            "Not arguments supplied, {} v{} is self summarizing {}.",
            crate_name,
            crate_version,
            exe_path.file_name().unwrap_or_default().to_string_lossy()
        );

        // Read Cargo.toml content using the include_str! macro.
        let cargo_toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        combined_source.push_str(&format!(
            "\n//- ----- [Cargo.toml] -----\n{}\n//- ----- [Cargo.toml] -----\n\n",
            cargo_toml
        ));
        for entry in SRC_DIR.find("**/*.rs").unwrap() {
            if let Some(file) = entry.as_file() {
                let rel_path = entry.path().display();
                let header = format!("//- ----- [{}]::{} -----\n", crate_name, rel_path);
                combined_source.push_str(&header);

                // Attempt to get UTF-8 contents; if unavailable, use a lossily converted version.
                let file_content = if let Some(contents) = file.contents_utf8() {
                    contents.to_string()
                } else {
                    String::from_utf8_lossy(file.contents()).into_owned()
                };
                combined_source.push_str(&file_content);

                let footer = format!("\n//- ----- [{}]::{} -----\n\n", crate_name, rel_path);
                combined_source.push_str(&footer);
            }
        }
        combined_source

        // // Use a relative path to the binary source as fallback.
        // // include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).to_string()
        // // This will embed the contents at compile time.
        // let lib_source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
        // let main_source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"));
        // format!(
        //     "{}\n\n// ===== Begin src/main.rs =====\n\n{}\n// ===== End src/main.rs =====",
        //     lib_source, main_source
        // )
    };
    println!("{}", content);
    // Create a ChatSession with a system prompt for summarization.
    let mut session = ChatSession::new("You are a Rust code analyst.", "gpt-4o-mini", streaming);

    // If no file path is provided, generate a help/usage summary first.
    if file_path.is_none() {
        // Combine all .rs files from src into one source string.
        // let combined_source = get_combined_source(args.detailed_headers)?;
        let usage_prompt = format!(
            "Based on the following source code, please generate a concise help and usage summary:\n\n{}",
            content
        );
        // Ask for the usage summary using the existing session.
        let usage_summary = session.ask(&usage_prompt).await?;
        println!("Help and Usage Summary:\n{}\n", usage_summary);
        println!(
            "> {} v{} {} is performing self summarization report which includes the YES / NO answers to important questions.",
            crate_name,
            crate_version,
            exe_path.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    // Build the summarization prompt.
    let prompt = format!(
            "analyze the following Rust source code and summarize, be concise:
- What its main functionality is.
- Which crates are used.
- Whether it appears safe to run. This should include a SAFE_TO_RUN: YES/NO answer to start and a brief explanation.
- Whether it performs any deletes or file modifications. This should include a FILE_OPERATIONS: YES/NO answer to start and a brief explanation.
- If there are any notable limitations or issues.
- Any other relevant insights.
- Provide a tree view of the files if possible.  If you can provide a - after the name and short sentence what it is, do so, leaving it blank is acceptable too.
---
{}
",
            content
        );

    // Ask for the summary using the ChatSession.
    let summary = session.ask(&prompt).await?;
    // Now the ChatSession has the summary context appended for follow-up questions.
    Ok((summary, session))
}

pub async fn summarize_a_crate(
    crate_location: &str,
    session: &mut ChatSession,
) -> anyhow::Result<String> {
    // Gather files from the entire crate (set src_only to false).
    let files = crate::cargo_utils::gather_files_from_crate(crate_location, false)
        .with_context(|| format!("Failed to gather files from crate at {}", crate_location))?;

    // Combine file contents into one large source string.

    let mut combined_source = String::new();

    for (path, content) in &files {
        let path_str = path.to_string_lossy();

        combined_source.push_str(&format!("// File: {}\n", path_str));

        combined_source.push_str(content);

        combined_source.push_str("\n\n");
    }

    // Create a summarization prompt.

    let prompt = format!(
        "Analyze the following Rust crate source code and summarize its main functionality, safety (including file operations), and any notable issues:\n\n{}",
        combined_source
    );

    // Use the provided ChatSession to get the summary.

    let answer = session.ask(&prompt).await?;

    Ok(answer)
}

/// Generates heredoc output for each file.
pub fn generate_heredoc_output(
    crate_name: &str,
    crate_version: &str,
    files: &std::collections::HashMap<std::path::PathBuf, String>,
) -> String {
    let mut out = String::new();

    let ver = if !crate_version.is_empty() {
        format!("v{}", &crate_version)
    } else {
        String::new()
    };
    let reference = if !crate_name.is_empty() && !ver.is_empty() {
        format!("[{} {}]", &crate_name, &ver)
    } else if !crate_name.is_empty() {
        format!("[{}]", &crate_name)
    } else {
        String::new()
    };
    for (path, content) in files {
        let rel_path = path.to_slash_lossy();
        out.push_str(&format!("//- ----- {}::{} -----\n", reference, rel_path));
        out.push_str(&sanitize(content));
        out.push_str(&format!("\n//- ----- {}::{} -----\n", reference, rel_path));
    }
    out
}

// Synchronous wrappers (_blocking versions)
pub fn summarize_source_blocking() -> anyhow::Result<String> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(summarize_source())
        .map_err(|e| anyhow::Error::msg(e.to_string()))
}

pub fn summarize_source_session_blocking(
    file_path: Option<&str>,
    streaming: bool,
) -> anyhow::Result<(String, ChatSession)> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(summarize_source_session(file_path, streaming))
        .map_err(|e| anyhow::Error::msg(e.to_string()))
}

pub fn summarize_a_crate_blocking(
    crate_location: &str,
    session: &mut ChatSession,
) -> anyhow::Result<String> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(summarize_a_crate(crate_location, session))
        .map_err(|e| anyhow::Error::msg(e.to_string()))
}
