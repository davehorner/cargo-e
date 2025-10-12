//! # e_window_api
//!
//! A high-level Rust API for controlling e_window instances.
//! This crate provides a convenient interface for launching e_window processes,
//! sending control commands, and managing window content dynamically.
//!
//! ## Features
//!
//! - Launch e_window processes with custom configurations
//! - Send real-time control commands via stdin
//! - Animate window position and size with easing
//! - Update window content dynamically
//! - Manage window lifecycle programmatically
//!
//! ## Examples
//!
//! ```rust,no_run
//! use e_window_api::{EWindow, WindowConfig, Card};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = WindowConfig::new()
//!         .title("My Window")
//!         .size(800, 600)
//!         .position(100, 100);
//!     
//!     let mut window = EWindow::launch(config).await?;
//!     
//!     let card = Card::new()
//!         .field("name", "Example")
//!         .content("Hello from e_window_api!");
//!     
//!     window.show_card(card).await?;
//!     window.close().await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod card;
pub mod config;
pub mod control;
pub mod error;
pub mod msgbox;
pub mod window;

pub use card::{Anchor, Card};
pub use config::WindowConfig;
pub use control::{AnimationEasing, ControlCommand};
pub use error::{EWindowError, Result};
pub use msgbox::{
    message_box, notify, notify_async, MessageBoxIcon, MessageBoxResult, MessageBoxType,
};
pub use window::EWindow;
pub use e_window_native::get_screen_size as get_screen_size;
