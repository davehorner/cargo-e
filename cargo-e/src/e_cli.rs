use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author,version, about = "cargo-e is for Example.", long_about = None)]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// Run all examples for a given number of seconds.

    /// Path to read/write the stdout of the executed command.
    #[arg(
        long,
        value_name = "PATH",
        help = "Path to read/write the stdout of the executed command."
    )]
    pub stdout: Option<std::path::PathBuf>,

    /// Path to read/write the stderr of the executed command.
    #[arg(
        long,
        value_name = "PATH",
        help = "Path to read/write the stderr of the executed command."
    )]
    pub stderr: Option<std::path::PathBuf>,
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

    #[arg(long, help = "Create GIST run_report.md on exit.")]
    pub gist: bool,
    #[arg(long, help = "Build and run in release mode.")]
    pub release: bool,
    #[arg(
        long,
        short = 'q',
        help = "Suppress cargo output when running the sample."
    )]
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

    #[clap(
        long,
        default_value_t = false,
        help = "If enabled, execute the existing target directly."
    )]
    pub cached: bool,

    /// Enable passthrough mode (no cargo output filtering, stdout is captured).
    #[arg(
        long = "filter",
        short = 'f',
        help = "Enable passthrough mode. No cargo output is filtered, and stdout is captured."
    )]
    pub filter: bool,
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
        default_value_t = 15,
        help = "Set wait time in seconds (default: 15)."
    )]
    pub wait: u64,

    /// Subcommands to run (e.g., `build|b`, `test|t`).
    #[arg(
        long = "subcommand",
        short = 's',
        value_parser,
        default_value = "run",
        help = "Specify subcommands (e.g., `build|b`, `test|t`)."
    )]
    pub subcommand: String,

    #[arg(help = "Specify an explicit target to run.")]
    pub explicit_example: Option<String>,

    #[arg(
        long = "run-at-a-time",
        short = 'J',
        default_value_t = 1,
        value_parser = clap::value_parser!(usize),
        help = "Number of targets to run at a time in --run-all mode (--run-at-a-time)"
    )]
    pub run_at_a_time: usize,

    #[arg(
        long = "nS",
        default_value_t = false,
        help = "Disable status lines during runtime loop output."
    )]
    pub no_status_lines: bool,

    #[arg(
        long = "nT",
        default_value_t = false,
        help = "Disable text-to-speech output."
    )]
    pub no_tts: bool,

    #[arg(long = "nW", default_value_t = false, help = "Disable window popups.")]
    pub no_window: bool,

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
    if cfg!(target_os = "windows") {
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
    if cfg!(target_os = "windows") {
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
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RunAll {
    /// The flag was not specified.
    #[default]
    NotSpecified,
    /// The flag was specified without a value—indicating “run forever.”
    Forever,
    /// The flag was specified with a timeout value.
    Timeout(u64),
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

pub fn custom_cli(args: &mut Vec<String>) -> (Option<usize>, Vec<&String>) {
    // If the first argument after the binary name is "e", remove it.
    if args.len() > 1 && args[1].as_str() == "e" {
        args.remove(1);
    }
    let mut run_at_a_time: Option<usize> = None;
    // default
    let mut filtered_args = vec![];
    for arg in &*args {
        if let Some(num) = arg
            .strip_prefix("--run-")
            .and_then(|s| s.strip_suffix("-at-a-time"))
        {
            if let Ok(n) = num.parse() {
                println!("run-at-a-time: {}", n);
                run_at_a_time = Some(n);
            }
            // Don't push this arg to filtered_args
            continue;
        }
        filtered_args.push(arg);
    }
    (run_at_a_time, filtered_args)
}
