//! Library for `e_crate_version_checker`.
//!
//! Provides functionality to query crates.io for crate version information.

pub mod e_crate_update;
pub mod e_interactive_crate_upgrade;

// Create a prelude module that re-exports key items
pub mod prelude {
    // Re-export selected items from the modules
    // Adjust these as necessary to match the items in your modules.
    pub use crate::e_crate_update::*;
    pub use crate::e_interactive_crate_upgrade::*;
}
