use clap::Parser;
use std::path::Path;
use tokio;
#[derive(Parser, Debug)]
pub struct GenScriptArgs {
    /// Crate location to start the upward search for Cargo.toml.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    pub crate_location: String,
    /// Language for the generated script ("py", "rs", or "summarize").
    #[arg(short = 'l', long = "language", default_value = "rs")]
    pub language: String,
    /// Process only the 'src' subfolder.
    #[arg(long = "src-only")]
    pub src_only: bool,
    /// Model to use when summarizing (only used if language is "summarize").
    #[arg(short = 'm', long = "model", default_value = "gpt-4o-mini")]
    pub model: String,
    /// Enable streaming mode (only used if language is "summarize").
    #[arg(short = 's', long = "streaming")]
    pub streaming: bool,
    /// System prompt to use when summarizing (only used if language is "summarize").
    #[arg(
        short = 'S',
        long = "system",
        default_value = "You are a Rust code analyst."
    )]
    pub system: String,
}

pub fn run(args: GenScriptArgs) -> anyhow::Result<()> {
    match args.language.to_lowercase().as_str() {
        "py" => {
            crate::crate_recreator_py::recreate_crate_py(
                Path::new(&args.crate_location),
                args.src_only,
            )?;
        }
        "rs" => {
            crate::crate_recreator_rs::recreate_crate_rs(
                Path::new(&args.crate_location),
                args.src_only,
            )?;
        }
        "summarize" => {
            // For summarization, create a ChatSession with the provided parameters.
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let mut session =
                    crate::summarizer::ChatSession::new(&args.system, &args.model, args.streaming);
                let summary =
                    crate::summarizer::summarize_a_crate(&args.crate_location, &mut session)
                        .await?;
                println!("Summary:\n{}\n", summary);
                anyhow::Ok(())
            })?;
        }
        other => {
            eprintln!(
                "Unknown language option: {}. Supported options: py, rs, summarize",
                other
            );
        }
    }
    Ok(())
}
pub async fn run_async(args: GenScriptArgs) -> anyhow::Result<()> {
    match args.language.to_lowercase().as_str() {
        "py" => {
            // These functions are synchronous, so we call them directly.
            crate::crate_recreator_py::recreate_crate_py(
                Path::new(&args.crate_location),
                args.src_only,
            )?;
            Ok(())
        }
        "rs" => {
            crate::crate_recreator_rs::recreate_crate_rs(
                Path::new(&args.crate_location),
                args.src_only,
            )?;
            Ok(())
        }
        "summarize" => {
            // Run the async summarization directly.
            let mut session =
                crate::summarizer::ChatSession::new(&args.system, &args.model, args.streaming);
            let summary =
                crate::summarizer::summarize_a_crate(&args.crate_location, &mut session).await?;
            println!("Summary:\n{}\n", summary);
            Ok(())
        }
        other => {
            eprintln!(
                "Unknown language option: {}. Supported options: py, rs, summarize",
                other
            );
            Ok(())
        }
    }
}

// use clap::Parser;
// use std::path::Path;
// use crate::summarizer::summarize_a_crate;

// #[derive(Parser, Debug)]
// pub struct GenScriptArgs {
//     /// Crate location to start the upward search for Cargo.toml.
//     #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
//     pub crate_location: String,
//     /// Language for the generated script ("py" or "rs" or "summarize").
//     #[arg(short = 'l', long = "language", default_value = "rs")]
//     pub language: String,
//     /// Process only the 'src' subfolder.
//     #[arg(long = "src-only")]
//     pub src_only: bool,
// }

// pub fn run(args: GenScriptArgs) -> anyhow::Result<()> {
//     match args.language.to_lowercase().as_str() {
//         "py" => {
//             crate::crate_recreator_py::recreate_crate_py(Path::new(&args.crate_location), args.src_only)?;
//         }
//         "rs" => {
//             crate::crate_recreator_rs::recreate_crate_rs(Path::new(&args.crate_location), args.src_only)?;
//         }
//         "summarize" => {
//             // Fallback: summarize the crate.
//             let rt = tokio::runtime::Runtime::new()?;
//             rt.block_on(async {
//                 let (summary, _) =
//                     summarize_a_crate(&args.crate_location).await?;
//                 println!("Summary:\n{}\n", summary);
//                 Ok(())
//             })?;
//         }
//         other => {
//             eprintln!("Unknown language option: {}. Supported options: py, rs, summarize", other);
//         }
//     }
//     Ok(())
// }
