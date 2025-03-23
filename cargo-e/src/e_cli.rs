use clap::Parser;

#[derive(Parser, Debug)]
#[command(author,version, about = "cargo-e is for Example.", long_about = None)]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// Run all examples for a given number of seconds.
    ///
    /// If provided with a value (e.g. `--run-all 10`), each target will run for 10 seconds.
    /// If provided without a value (i.e. just `--run-all`), it means run forever.
    /// If not provided at all, then the default wait time is used.
    #[arg(
        long,
        num_args = 0..=1,
        default_value_t = RunAll::NotSpecified,
        default_missing_value ="",
        value_parser,
        help = "Run all optionally specifying run time (in seconds) per target. \
                If the flag is present without a value, run forever."
    )]
    pub run_all: RunAll,

    #[arg(long, help = "Build and run in release mode.")]
    pub release: bool,
    #[arg(long, help = "Suppress cargo output when running the sample.")]
    pub quiet: bool,
    // /// Comma-separated list of package names.
    // #[clap(long, value_delimiter = ',', help = "Optional list of package names to run examples for. If omitted, defaults to ALL_PACKAGES.")]
    // pub specified_packages: Vec<String>,
    /// Pre-build examples before running.
    #[clap(
        long,
        help = "If enabled, pre-build the examples before executing them."
    )]
    pub pre_build: bool,

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

use std::str::FromStr;

/// Represents the state of the `--run-all` flag.
#[derive(Debug, Clone, PartialEq)]
pub enum RunAll {
    /// The flag was not specified.
    NotSpecified,
    /// The flag was specified without a value—indicating “run forever.”
    Forever,
    /// The flag was specified with a timeout value.
    Timeout(u64),
}

impl Default for RunAll {
    fn default() -> Self {
        RunAll::NotSpecified
    }
}

impl FromStr for RunAll {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // An empty string means the flag was provided without a value → run forever.
        if s.is_empty() {
            Ok(RunAll::Forever)
        } else if s.eq_ignore_ascii_case("not_specified") {
            Ok(RunAll::NotSpecified)
        } else {
            // Otherwise, try parsing a u64 value.
            s.parse::<u64>()
                .map(RunAll::Timeout)
                .map_err(|e| e.to_string())
        }
    }
}

impl std::fmt::Display for RunAll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunAll::NotSpecified => write!(f, "not_specified"),
            RunAll::Forever => write!(f, "forever"),
            RunAll::Timeout(secs) => write!(f, "{}", secs),
        }
    }
}
