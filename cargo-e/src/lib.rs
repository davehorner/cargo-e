#![doc = include_str!("../README.md")]

// Re-export std common modules
pub mod prelude {
    pub use std::env;
    pub use std::error::Error;
    pub use std::fs;
    pub use std::io;
    pub use std::path::{Path, PathBuf};
    pub use std::process::exit;
    pub use std::process::Child;
    pub use std::process::Command;
    pub use std::process::Stdio;
    pub use std::sync::mpsc;
    pub use std::sync::{Arc, Mutex};
    pub use std::time::Instant;
    //pub use tracing::{debug, error, info};
    pub use log::{debug, error, info, log_enabled, Level};
}

pub mod e_findmain;
pub use e_findmain::*;
pub mod e_bacon;
pub mod e_types;
pub use e_bacon::*;
pub mod e_cli;
pub use e_cli::Cli;
pub mod e_manifest;
pub use e_manifest::{collect_workspace_members, locate_manifest};
pub mod e_parser;
pub use e_parser::parse_available;
pub mod e_autosense;
pub mod e_cargocommand_ext;
pub mod e_collect;
pub mod e_command_builder;
pub mod e_diagnostics_dispatchers;
pub mod e_discovery;
pub mod e_eventdispatcher;
pub mod e_features;
pub mod e_fmt;
pub mod e_installer;
pub mod e_prebuild;
pub mod e_processmanager;
pub mod e_prompts;
pub mod e_reports;
pub mod e_runall;
pub mod e_runner;
pub mod e_target;
pub mod e_tui;
pub mod e_workspace;
#[cfg(feature = "uses_tts")]
use once_cell::sync::OnceCell;
use dashmap::DashSet;

#[cfg(feature = "uses_tts")]
pub static GLOBAL_TTS: OnceCell<std::sync::Mutex<tts::Tts>> = OnceCell::new();
pub static GLOBAL_MANAGER: OnceCell<std::sync::Arc<e_processmanager::ProcessManager>> =
    OnceCell::new();
pub static GLOBAL_CLI: OnceCell<Cli> = OnceCell::new();
/// A global set to track PIDs of ewindow processes.
pub static GLOBAL_EWINDOW_PIDS: OnceCell<dashmap::DashMap<u32,u32>> = OnceCell::new();
// Plugin system modules
/// Extension API: unified CLI+targets for embedding cargo-e
pub mod ext;
#[cfg(feature = "uses_plugins")]
pub mod plugins;

#[allow(unused_macros)]
macro_rules! doc_with_joke {
    (
        $(#[$attr:meta])*
        $vis:vis fn $name:ident($($arg:tt)*) -> $ret:ty $body:block
    ) => {
        $(#[$attr])*                           // Re-emit external doc attributes.
        #[doc = "Have you read the Guide for You To Read?"] // Extra injected doc comment.
        $vis fn $name($($arg)*) -> $ret $body    // Emit the function.
    };
}

// Define the helper macro.
// macro_rules! doc_with_funny {
//     ($doc:expr) => {
//         #[doc = concat!($doc, "\n\nSee also the [Guide for You To Read](index.html) for more details.")]
//     };
// }

// Define the helper macro that captures external attributes and injects an extra doc attribute.
// macro_rules! doc_with_funny {
//     (
//         $(#[$attr:meta])*
//         $vis:vis fn $name:ident($($arg:tt)*) -> $ret:ty $body:block
//     ) => {
//         $(#[$attr])*                           // Re-emit external doc attributes.
//         #[doc = "Have you read the Guide for You To Read?"] // Extra injected doc comment.
//         $vis fn $name($($arg)*) -> $ret $body    // Emit the function.
//     };
// }

// #[doc = include_str!("../documents/guide.md")]
// pub mod a_guide {
//     /// An example constant to force module inclusion.
//     doc_with_guide!("This function does something important.");
//     pub const a_const: () = ();

//     /// An example function to force module inclusion.
//     pub fn a_function() {}

//     /// An example struct to force module inclusion.
//     pub struct AStruct;

//     /// An example enum to force module inclusion.
//     pub enum AEnum {
//         AVariant,
//     }

//     /// An example trait to force module inclusion.
//     pub trait ATrait {}

//     /// An example type alias to force module inclusion.
//     pub type AType = ();
// }

// // Re-export so the module shows in the public API.
// pub use a_guide as __THE_GUIDE;

#[macro_use]
pub mod a_funny_docs;

#[doc = include_str!("../documents/guide.md")]
pub mod a_guide {
    // A constant with a humorous aside
    #[doc = "A wacky constant that reminds you of the fleeting nature of existence—because constants, like our dreams, never change."]
    #[doc = "Imagine staring into the abyss of an unchanging value and laughing at the cosmic joke: even when everything is fixed, there’s always room for a little absurdity. In a world of mutable chaos, this constant stands as a monument to the absurdity of permanence."]
    #[doc = "SEE ALSO THE **[GUIDE FOR YOU TO READ](index.html) FOR MORE DETAILS** ON THE MYSTERIES OF CONSTANTS."]
    pub const A_CONST: () = ();
    // A function with a humorous aside
    #[doc = "An eccentric function that performs its task with a whimsical twist."]
    #[doc = "Picture a function that cracks jokes as it runs—each call a mini stand-up routine where recursion becomes a humorous loop and error handling turns into a comedy of exceptions. This function reminds you that even in logic there is laughter."]
    #[doc = "SEE ALSO THE **[GUIDE FOR YOU TO READ](index.html) FOR MORE DETAILS** ON THE ART OF FUNCTIONAL HUMOR."]
    pub fn a_function() {}

    // A struct with a humorous aside
    #[doc = "A delightfully absurd struct that encapsulates the essence of lighthearted programming."]
    #[doc = "Think of it as the punchline to a well-crafted joke: simple on the surface yet bursting with hidden layers of wit. This struct is the blueprint for designing data structures that know how to have a good time even when they’re being strictly typed."]
    #[doc = "SEE ALSO THE **[GUIDE FOR YOU TO READ](index.html) FOR MORE DETAILS** ON STRUCTURING YOUR HUMOR."]
    pub struct AStruct;

    // An enum with a humorous aside
    #[doc = "An enum whose variants are as unpredictable as the punchline of an offbeat comedy routine."]
    #[doc = "Each variant is a different flavor of chaos—a reminder that sometimes, even in the binary world of enums, you need a twist of fate and a pinch of absurdity. Embrace the randomness with a hearty chuckle."]
    #[doc = "SEE ALSO THE **[GUIDE FOR YOU TO READ](index.html) FOR MORE DETAILS** ON ENUMERATING THE LAUGHTER."]
    //#[doc = doc_with_funny!("**ENUMERATE THE LAUGHTER:** This enum's variants are as surprising as a punchline in the middle of a monologue. Discover the unexpected twist in every variant.")]
    pub enum AEnum {
        #[doc = "A variant that boldly goes where no variant has gone before—capturing the essence of unexpected hilarity."]
        AVariant,
    }

    // A trait with a humorous aside
    #[doc = "A quirky trait that defines behaviors with a tongue-in-cheek twist."]
    #[doc = "Imagine a trait written by a stand-up comedian: each method is a punchline, each implementation an opportunity for subtle humor. Types implementing this trait are expected not just to act, but to entertain—blending functionality with a dash of wit."]
    #[doc = "SEE ALSO THE **[GUIDE FOR YOU TO READ](index.html) FOR MORE DETAILS** ON TRAIT-ORIENTED COMEDY."]
    pub trait ATrait {}

    // A type alias with a humorous aside
    #[doc = "A type alias that serves as a humorous shortcut to a more verbose reality."]
    #[doc = "Sometimes types need nicknames too. This alias is like that clever one-liner you whisper at a party—short, memorable, and unexpectedly delightful. It’s the wink in the midst of an otherwise serious type system."]
    #[doc = "SEE ALSO THE **[GUIDE FOR YOU TO READ](index.html) FOR MORE DETAILS** ON ALIASING THE ORDINARY INTO THE EXTRAORDINARY."]
    pub type AType = ();
}

/// If you didn't get it the first time, then look again!  
/// Check out the **[GUIDE FOR YOU TO READ](theSeachIsOver/index.html)** for all the brilliant details you missed.
pub use a_guide as theSeachIsOver;
