#![doc = include_str!("../README.md")]

// Re-export std common modules
pub mod prelude {
    pub use tracing::{info,debug,error};
    pub use std::env;
    pub use std::fs;
    pub use std::io;
    pub use std::path::{Path, PathBuf};
    pub use std::error::Error;
    pub use std::process::Command;
    pub use std::process::Child;
    pub use std::process::Stdio;
    pub use std::process::exit;
    pub use std::time::Instant;
    pub use std::sync::mpsc;
    pub use std::sync::{Arc, Mutex};
}

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
pub mod e_features;
pub mod e_tui;
pub mod e_collect;