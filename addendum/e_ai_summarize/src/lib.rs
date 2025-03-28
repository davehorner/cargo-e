//! e_ai_summarize: A GenAI-powered Rust source code summarizer.
//!
//! This crate analyzes Rust source code to summarize its main functionality,
//! used crates, safety (including file operations), and any notable issues.

pub mod summarizer {
    use genai::Client;
    use genai::chat::printer::PrintChatStreamOptions;
    use genai::chat::printer::print_chat_stream;
    use genai::chat::{ChatMessage, ChatRequest};
    use std::env;
    use std::fs;
    use std::path::Path;

    pub struct ChatSession {
        client: Client,
        model: String,
        chat_req: ChatRequest,
        streaming: bool,
        print_options: PrintChatStreamOptions,
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
        pub async fn ask(&mut self, question: &str) -> Result<String, Box<dyn std::error::Error>> {
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
        // Read the content from the provided file or fallback to this file's own source.
        let content = if let Some(fp) = file_path {
            let path = Path::new(fp);
            fs::read_to_string(path).unwrap_or_else(|err| {
                eprintln!("Error reading {}: {}", fp, err);
                std::process::exit(1);
            })
        } else {
            // Use a relative path to the binary source as fallback.
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).to_string()
        };

        // Build the summarization prompt.
        let prompt = format!(
            "analyze the following Rust source code and summarize, be concise:
- What its main functionality is.
- Which crates are used.
- Whether it appears safe to run. This should include a SAFE_TO_RUN: YES/NO answer to start and a brief explanation.
- Whether it performs any deletes or file modifications. This should include a FILE_OPERATIONS: YES/NO answer to start and a brief explanation.
- If there are any notable limitations or issues.
- Any other relevant insights.
---
{}
",
            content
        );

        // Create a ChatSession with a system prompt for summarization.
        let mut session =
            ChatSession::new("You are a Rust code analyst.", "gpt-4o-mini", streaming);

        // Ask for the summary using the ChatSession.
        let summary = session.ask(&prompt).await?;
        // Now the ChatSession has the summary context appended for follow-up questions.
        Ok((summary, session))
    }
}
