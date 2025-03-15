//! Library for `e_crate_version_checker`.
//!
//! Provides functionality to query crates.io for crate version information.

pub mod e_crate_update;
pub mod e_interactive_crate_upgrade;

// Create a prelude module that re-exports key items
pub mod prelude {
    pub use crate::e_crate_update::version::local_crate_version_via_executable;
    pub use crate::e_crate_update::version::lookup_local_version_via_cargo;
    pub use crate::e_crate_update::*;
    pub use crate::e_interactive_crate_upgrade::*;
}
