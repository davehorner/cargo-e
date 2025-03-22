use clap::Parser;

#[derive(Parser, Debug)]
#[command(author,version, about = "cargo-e is for Example.", long_about = None)]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// Print version and feature flags in JSON format.
    #[arg(
        long,
        short = 'v',
        help = "Print version and feature flags in JSON format."
    )]
    pub version: bool,

    #[arg(
        long,
        short = 't',
        help = "Launch the text-based user interface (TUI)."
    )]
    pub tui: bool,

    #[arg(long, short = 'w', help = "Operate on the entire workspace.")]
    pub workspace: bool,

    /// Print the exit code of the process when run.
    #[arg(
        long = "pX",
        default_value_t = false,
        value_parser = clap::value_parser!(bool),
        help = "Print the exit code of the process when run. (default: false)"
    )]
    pub print_exit_code: bool,

    /// Print the program name before execution.
    #[arg(
        long = "pN",
        default_value_t = false,
        value_parser = clap::value_parser!(bool),
        help = "Print the program name before execution. (default: false)"
    )]
    pub print_program_name: bool,

    /// Print the program name before execution.
    #[arg(
        long = "pI",
        default_value_t = true,
        value_parser = clap::value_parser!(bool),
        help = "Print the user instruction. (default: true)"
    )]
    pub print_instruction: bool,

    #[arg(
        long,
        short = 'p',
        default_value_t = true,
        help = "Enable or disable paging (default: enabled)."
    )]
    pub paging: bool,

    #[arg(
        long,
        short = 'r',
        default_value_t = false,
        help = "Relative numbers (default: enabled)."
    )]
    pub relative_numbers: bool,

    #[arg(
        long = "wait",
        short = 'W',
        default_value_t = 5,
        help = "Set wait time in seconds (default: 5)."
    )]
    pub wait: u64,

    #[arg(help = "Specify an explicit example to run.")]
    pub explicit_example: Option<String>,

    #[arg(last = true, help = "Additional arguments passed to the command.")]
    pub extra: Vec<String>,
}

/// Print the version and the JSON array of feature flags.
pub fn print_version_and_features() {
    // Print the version string.
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");

    // Build a list of feature flags. Enabled features are printed normally,
    // while disabled features are prefixed with an exclamation mark.
    let mut features = Vec::new();

    if cfg!(feature = "tui") {
        features.push("tui");
    } else {
        features.push("!tui");
    }
    if cfg!(feature = "concurrent") {
        features.push("concurrent");
    } else {
        features.push("!concurrent");
    }
    if cfg!(feature = "windows") {
        features.push("windows");
    } else {
        features.push("!windows");
    }
    if cfg!(feature = "equivalent") {
        features.push("equivalent");
    } else {
        features.push("!equivalent");
    }

    let json_features = format!(
        "[{}]",
        features
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<String>>()
            .join(", ")
    );
    println!("cargo-e {}", version);
    println!("{}", json_features);
    std::process::exit(0);
}

/// Returns a vector of feature flag strings.  
/// Enabled features are listed as-is while disabled ones are prefixed with "!".
pub fn get_feature_flags() -> Vec<&'static str> {
    let mut flags = Vec::new();
    if cfg!(feature = "tui") {
        flags.push("tui");
    } else {
        flags.push("!tui");
    }
    if cfg!(feature = "concurrent") {
        flags.push("concurrent");
    } else {
        flags.push("!concurrent");
    }
    if cfg!(feature = "windows") {
        flags.push("windows");
    } else {
        flags.push("!windows");
    }
    if cfg!(feature = "equivalent") {
        flags.push("equivalent");
    } else {
        flags.push("!equivalent");
    }
    flags
}
