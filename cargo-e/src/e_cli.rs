use clap::Parser;

#[derive(Parser, Debug)]
#[command(author,version, about = "cargo-e is for Example.", long_about = None)]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// Print version and feature flags in JSON format.
    #[arg(long, short = 'v')]
    pub version: bool,

    #[arg(long, short = 't')]
    pub tui: bool,

    #[arg(long, short = 'w')]
    pub workspace: bool,

    #[arg(long = "wait", short = 'W', default_value_t = 5)]
    pub wait: u64,

    pub explicit_example: Option<String>,

    #[arg(last = true)]
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
