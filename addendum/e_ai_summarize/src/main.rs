use clap::Parser;
use e_ai_summarize::summarizer::summarize_source_session;
use log::debug;
use rustyline::DefaultEditor;
use std::path::Path;
use tokio;

// Our CLI now includes a flag for generating a Rust script to recreate the crate.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Optional file path: for summarization this is a Rust source file;
    /// for crate recreation this is the source folder to process.
    file: Option<String>,

    /// Run interactive follow-up mode (for summarization).
    #[arg(long = "stdin", conflicts_with = "question")]
    interactive: bool,

    /// Provide a single follow-up question (for summarization).
    #[arg(short = 'q', long, conflicts_with = "interactive")]
    question: Option<String>,

    /// Enable streaming mode (for summarization).
    #[arg(long = "streaming", action = clap::ArgAction::SetTrue)]
    streaming: bool,

    /// Generate a Python script that recreates the crate.
    #[arg(long = "recreate-crate-py")]
    recreate_crate_py: bool,

    /// Generate a Rust script that recreates the crate.
    #[arg(long = "recreate-crate-rs")]
    recreate_crate_rs: bool,

    /// Process only the 'src' subfolder (for crate recreation).
    #[arg(long = "src-only")]
    src_only: bool,
}

// mod cli;

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     cli::run()
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new()
        .filter_module("rustyline", log::LevelFilter::Warn)
        .init();

    // Parse command-line arguments.
    let args = Args::parse();

    if args.recreate_crate_rs {
        // For Rust script recreation mode, use the `file` argument as the source folder.
        let source_folder = args.file.unwrap_or_else(|| ".".to_string());
        e_ai_summarize::crate_recreator_rs::recreate_crate_rs(
            Path::new(&source_folder),
            args.src_only,
        )?;
    } else if args.recreate_crate_py {
        // For Python script recreation mode, use the existing module.
        let source_folder = args.file.unwrap_or_else(|| ".".to_string());
        e_ai_summarize::crate_recreator_py::recreate_crate_py(
            Path::new(&source_folder),
            args.src_only,
        )?;
    } else {
        // Summarization mode (existing functionality).
        let (summary, mut session) =
            summarize_source_session(args.file.as_deref(), args.streaming).await?;
        debug!("Summary:\n{}\n", summary);

        if args.interactive || args.question.is_some() {
            if args.interactive {
                let mut rl: DefaultEditor = DefaultEditor::new()?;
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
    }

    Ok(())
}
