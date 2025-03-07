#![doc = include_str!("../README.md")]


pub mod e_findmain;
pub use e_findmain::*;
pub mod e_types;
pub use e_types::*;
pub mod e_bacon;
pub use e_bacon::*;
pub mod e_cli;
pub use e_cli::Cli;
pub mod e_manifest;
pub use e_manifest::{locate_manifest, collect_workspace_members};
pub mod e_parser;
pub use e_parser::parse_available;
pub mod e_runner;
pub use e_runner::run_example;
