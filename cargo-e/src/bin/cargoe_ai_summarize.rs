// use e_ai_summarize::summarizer;
//
// #[tokio::main]
// async fn main() {
//     match summarizer::summarize_source().await {
//         Ok(summary) => println!("{}", summary),
//         Err(err) => eprintln!("Error during summarization: {}", err),
//     }
// }
//
//
//
use clap::Parser;
use e_ai_summarize::summarizer::{self};
use log::debug;
use rustyline::{Config, DefaultEditor};
use tokio;

/// Command-line arguments.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Optional file path to a Rust source file to summarize.
    file: Option<String>,

    /// Run interactive follow-up mode (read questions interactively until an empty line is entered)
    #[arg(long = "stdin", conflicts_with = "question")]
    interactive: bool,

    /// Provide a single follow-up question (reads until newline)
    #[arg(short = 'q', long, conflicts_with = "interactive")]
    question: Option<String>,

    /// Enable streaming mode (if not set, non-streaming mode is used)
    #[arg(long = "streaming", action = clap::ArgAction::SetTrue)]
    streaming: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (set RUST_LOG=debug to see debug! output).
    env_logger::Builder::new()
        .filter_module("rustyline", log::LevelFilter::Warn)
        .init();

    // Parse command-line arguments.
    let args = Args::parse();

    // Call summarize_source, which returns both the summary text and a ChatSession preloaded with that context.
    let (summary, mut session) =
        summarizer::summarize_source_session(args.file.as_deref(), args.streaming).await?;
    debug!("Summary:\n{}\n", summary);

    // If follow-up questions are desired, use the session.
    if args.interactive || args.question.is_some() {
        if args.interactive {
            let config = Config::builder().build();
            let mut rl = DefaultEditor::with_config(config)?;
            debug!("Interactive mode: enter follow-up questions (empty line to quit):");
            loop {
                let line = rl.readline("> ")?;
                let question = line.trim().to_string();
                if question.is_empty() {
                    break;
                }
                rl.add_history_entry(&question).ok();
                let answer = session.ask(&question).await?;
                debug!("Answer: {}\n", answer);
            }
        } else if let Some(q) = args.question {
            let answer = session.ask(&q).await?;
            debug!("Answer: {}\n", answer);
        }
    }

    Ok(())
}
