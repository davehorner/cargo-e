// use clap::Parser;
// use std::path::Path;
// use tokio;
// use crate::summarizer;

// #[derive(Parser, Debug, Default)]
// pub struct SummarizeArgs {
//     /// Crate location to start the upward search for Cargo.toml.
//     #[arg(short, long, value_hint = clap::ValueHint::DirPath, default_value = ".")]
//     pub crate_location: String,
//     /// Run interactive follow-up mode.
//     #[arg(long = "stdin", conflicts_with = "question")]
//     pub interactive: bool,
//     /// Provide a single follow-up question.
//     #[arg(short = 'q', long, conflicts_with = "interactive")]
//     pub question: Option<String>,
// }

// /// Run the summarize subcommand.
// pub fn run(args: SummarizeArgs) -> anyhow::Result<()> {
//     let rt = tokio::runtime::Runtime::new()?;
//     rt.block_on(async {
//         // Assumes your summarizer module exposes summarize_a_crate that accepts a crate location.
//         let summary =
//             summarizer::summarize_a_crate(&args.crate_location).await?;
//         println!("Summary:\n{}\n", summary);
//         if args.interactive || args.question.is_some() {
//             if args.interactive {
//                 let mut rl = rustyline::DefaultEditor::new()?;
//                 println!("Interactive mode: enter follow-up questions (empty line to quit):");
//                 loop {
//                     let line = rl.readline("> ")?;
//                     let q = line.trim().to_string();
//                     if q.is_empty() {
//                         break;
//                     }
//                     rl.add_history_entry(&q).ok();
//                     let answer = session.ask(&q).await?;
//                     println!("Answer: {}\n", answer);
//                 }
//             } else if let Some(q) = args.question {
//                 let answer = session.ask(&q).await?;
//                 println!("Answer: {}\n", answer);
//             }
//         }
//         Ok(())
//     })
// }

// /// Default run if no subcommand is provided.
// pub fn default_run() -> anyhow::Result<()> {
//     run(SummarizeArgs::default())
// }

use clap::Parser;
use tokio;
#[derive(Parser, Debug, Default)]
pub struct SummarizeArgs {
    /// Crate location to begin the upward search for Cargo.toml.
    #[arg(short, long, default_value = ".")]
    pub crate_location: String,
    /// Run interactive follow-up mode.
    #[arg(long = "stdin", conflicts_with = "question")]
    pub interactive: bool,
    /// Provide a single follow-up question.
    #[arg(short = 'q', long, conflicts_with = "interactive")]
    pub question: Option<String>,
    /// Enable streaming mode for the summarization session.
    #[arg(short = 's', long = "streaming")]
    pub streaming: bool,
    /// Model to use for the summarization (e.g. "gpt-4o-mini").
    #[arg(short = 'm', long = "model", default_value = "gpt-4o-mini")]
    pub model: String,
    /// System prompt to initialize the chat session.
    #[arg(
        short = 'S',
        long = "system",
        default_value = "You are a Rust code analyst."
    )]
    pub system: String,
}

pub fn run(args: SummarizeArgs) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Create a ChatSession using the provided system prompt, model, and streaming flag.
        let mut session =
            crate::summarizer::ChatSession::new(&args.system, &args.model, args.streaming);

        // Call summarize_a_crate with the provided crate location and mutable session.
        let summary =
            crate::summarizer::summarize_a_crate(&args.crate_location, &mut session).await?;
        println!("Summary:\n{}\n", summary);

        // If interactive mode or a follow-up question is requested, use the session.
        if args.interactive || args.question.is_some() {
            if args.interactive {
                let mut rl = rustyline::DefaultEditor::new()?;
                println!("Interactive mode: enter follow-up questions (empty line to quit):");
                loop {
                    let line = rl.readline("> ")?;
                    let q = line.trim().to_string();
                    if q.is_empty() {
                        break;
                    }
                    rl.add_history_entry(&q).ok();
                    let answer = session.ask(&q).await?;
                    println!("Answer: {}\n", answer);
                }
            } else if let Some(q) = args.question {
                let answer = session.ask(&q).await?;
                println!("Answer: {}\n", answer);
            }
        }
        Ok(())
    })
}

pub fn default_run() -> anyhow::Result<()> {
    run(SummarizeArgs::default())
}
