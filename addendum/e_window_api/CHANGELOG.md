# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.1.3](https://github.com/davehorner/cargo-e/compare/e_window_api-v0.1.2...e_window_api-v0.1.3) - 2025-10-12

### Added

- *(e_window)* integrate cross-platform screen size detection with e_window_native module

## [0.1.2](https://github.com/davehorner/cargo-e/compare/e_window_api-v0.1.1...e_window_api-v0.1.2) - 2025-10-12

### Fixed

- *(macos)* fix coregraphics errors.

## [0.1.1] - 2025-10-10

### Changed
- **BREAKING**: Removed heavy dependencies to create minimal dependency footprint
- Removed `serde`, `serde_json`, `anyhow`, `shell-words`, and `winapi` dependencies
- Reduced tokio features from "full" to only required features: `process`, `io-util`, `time`, `macros`, `rt-multi-thread`
- Changed error handling from `anyhow::Result` to `Result<T, Box<dyn std::error::Error>>`
- Removed Serialize/Deserialize derives from structs (not needed for API functionality)

### Improved
- Significantly reduced compilation time and binary size
- Only 3 direct dependencies now: `tokio`, `thiserror`, `which`
- Maintained all functionality while minimizing external dependencies

## [0.1.0] - 2025-10-10

### Added
- Initial release of e_window_api
- High-level Rust API for controlling e_window instances
- `WindowConfig` for configuring window properties
- `Card` struct for structured content display
- `EWindow` handle for window lifecycle management
- `ControlCommand` enum for real-time window control
- Animation support with `AnimationEasing` types
- Comprehensive error handling with `EWindowError`
- Async/await support throughout the API
- Three example programs:
  - `realtime_control_api`: Demonstrates animated window positioning
  - `panic_card_api`: Shows error/panic information display
  - `demo_ewindow_grid`: Grid-based positioning demonstration
- Full documentation and README with usage examples

### Features
- Launch e_window processes with custom configurations
- Send real-time control commands via stdin
- Animate window position and size with various easing functions
- Update window content dynamically with structured cards
- Manage window lifecycle programmatically
- Type-safe API with comprehensive error handling
- Cross-platform support (Windows-optimized)

### Dependencies
- tokio 1.0+ for async runtime
- serde 1.0+ for serialization
- anyhow 1.0+ for error handling
- thiserror 1.0+ for error types
- which 8.0+ for executable discovery
- winapi 0.3+ for Windows-specific features
