use clap::{Parser, Subcommand};

pub mod commands; // This will look for src/cli/commands/mod.rs
use self::commands::{genhere, genscript, summarize};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Summarize the Rust source code of a crate.
    Summarize(summarize::SummarizeArgs),
    /// Generate a complete recreate script (Python or Rust) for the crate.
    GenScript(genscript::GenScriptArgs),
    /// Generate heredoc output for each file in the crate.
    GenHere(genhere::GenHereArgs),
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Summarize(args)) => summarize::run(args),
        Some(Commands::GenScript(args)) => genscript::run(args),
        Some(Commands::GenHere(args)) => genhere::run(args),
        None => summarize::default_run(),
    }
}

pub async fn run_async() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Summarize(args)) => {
            // Call the asynchronous version of the summarize branch.
            // (You'll need to add a run_async function in your summarize module.)
            summarize::run_async(args).await?;
        }
        Some(Commands::GenScript(args)) => {
            // These functions remain synchronous. You can call them directly.
            genscript::run(args)?;
        }
        Some(Commands::GenHere(args)) => {
            genhere::run(args)?;
        }
        None => {
            // If no subcommand is provided, fallback to the default summarization run.
            summarize::run_async(summarize::SummarizeArgs::default()).await?;
        }
    }
    Ok(())
}
