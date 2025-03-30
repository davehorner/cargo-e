use crate::cargo_utils;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct GenHereArgs {
    /// Crate location to start the upward search for Cargo.toml.
    #[arg(short, long, value_hint = clap::ValueHint::DirPath, default_value = ".")]
    pub crate_location: String,
    /// Process only the 'src' subfolder.
    #[arg(long = "src-only")]
    pub src_only: bool,
    /// Language for heredoc output ("py" or "rs"). Defaults to "rs".
    #[arg(short = 'l', long = "language", default_value = "rs")]
    pub language: String,
}

pub fn run(args: GenHereArgs) -> anyhow::Result<()> {
    match args.language.to_lowercase().as_str() {
        "py" => {
            let toml_config =
                cargo_utils::find_cargo_toml(std::path::Path::new(&args.crate_location));
            if let Some(crate_toml_path) = toml_config {
                let (crate_name, crate_version) =
                    cargo_utils::get_crate_name_and_version(&crate_toml_path.to_path_buf())
                        .unwrap_or_default();
                let files =
                    cargo_utils::gather_files_from_crate(&args.crate_location, args.src_only)?;
                let output =
                    crate::summarizer::generate_heredoc_output(&crate_name, &crate_version, &files);
                std::fs::write("gen_here_output.txt", &output)?;
                crate::clipboard::copy_to_clipboard(&output)?;
                println!("Heredoc output written to gen_here_output.txt and copied to clipboard.");
            } else {
                println!("couldn't find a toml config.")
            }
        }
        "rs" => {
            let toml_config =
                cargo_utils::find_cargo_toml(std::path::Path::new(&args.crate_location));
            if let Some(crate_toml_path) = toml_config {
                let (crate_name, crate_version) =
                    cargo_utils::get_crate_name_and_version(&crate_toml_path.to_path_buf())
                        .unwrap_or_default();
                let files =
                    cargo_utils::gather_files_from_crate(&args.crate_location, args.src_only)?;
                let output =
                    crate::summarizer::generate_heredoc_output(&crate_name, &crate_version, &files);
                std::fs::write("gen_here_output.txt", &output)?;
                crate::clipboard::copy_to_clipboard(&output)?;
                println!("Heredoc output written to gen_here_output.txt and copied to clipboard.");
            } else {
                println!("couldn't find a toml config.")
            }
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
