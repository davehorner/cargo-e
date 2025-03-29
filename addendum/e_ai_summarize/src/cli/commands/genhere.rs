use crate::cargo_utils;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct GenHereArgs {
    /// Crate location to start the upward search for Cargo.toml.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    pub crate_location: String,
    /// Process only the 'src' subfolder.
    #[arg(long = "src-only")]
    pub src_only: bool,
    /// Language for heredoc output ("py" or "rs"). Defaults to "rs".
    #[arg(short = 'l', long = "language", default_value = "rs")]
    pub language: String,
}

/// Generates heredoc output for each file.
/// Each file is output as:
///
/// <<FILE: relative/path>>
/// file content here
/// <<END FILE>>
fn generate_heredoc_output(files: &std::collections::HashMap<PathBuf, String>) -> String {
    let mut out = String::new();
    for (path, content) in files {
        let path_str = path.to_string_lossy();
        out.push_str(&format!("<<FILE: {}>>\n", path_str));
        out.push_str(content);
        out.push_str("\n<<END FILE>>\n\n");
    }
    out
}

pub fn run(args: GenHereArgs) -> anyhow::Result<()> {
    match args.language.to_lowercase().as_str() {
        "py" => {
            let files = cargo_utils::gather_files_from_crate(&args.crate_location, args.src_only)?;
            let output = generate_heredoc_output(&files);
            std::fs::write("gen_here_output.txt", &output)?;
            crate::clipboard::copy_to_clipboard(&output)?;
            println!("Heredoc output written to gen_here_output.txt and copied to clipboard.");
        }
        "rs" => {
            let files = cargo_utils::gather_files_from_crate(&args.crate_location, args.src_only)?;
            let output = generate_heredoc_output(&files);
            std::fs::write("gen_here_output.txt", &output)?;
            crate::clipboard::copy_to_clipboard(&output)?;
            println!("Heredoc output written to gen_here_output.txt and copied to clipboard.");
        }
        other => {
            eprintln!(
                "Unknown language option: {}. Supported options: py, rs",
                other
            );
        }
    }
    Ok(())
}
