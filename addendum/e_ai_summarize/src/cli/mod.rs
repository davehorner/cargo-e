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
