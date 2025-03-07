use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Run examples or binaries from a Rust project with extended workspace support.", long_about = None)]
pub struct Cli {
    #[arg(long, short = 't')]
    pub tui: bool,

    #[arg(long, short = 'w')]
    pub workspace: bool,

    #[arg(long = "wait", short = 'W', default_value_t = 2)]
    pub wait: u64,

    pub explicit_example: Option<String>,

    #[arg(last = true)]
    pub extra: Vec<String>,
}
