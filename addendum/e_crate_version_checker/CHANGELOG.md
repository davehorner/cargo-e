# Changelog

All notable changes to this project will be documented in this file.


## [0.1.4](https://github.com/davehorner/cargo-e/compare/e_crate_version_checker-v0.1.3...e_crate_version_checker-v0.1.4) - 2025-03-15

### Other

- *(version)* remove semver dependency and update version functions

## [0.1.3](https://github.com/davehorner/cargo-e/compare/e_crate_version_checker-v0.1.2...e_crate_version_checker-v0.1.3) - 2025-03-15

### Added

- better sample resolution and findmain support

## [0.1.2](https://github.com/davehorner/cargo-e/compare/e_crate_version_checker-v0.1.1...e_crate_version_checker-v0.1.2) - 2025-03-15

### Added

- improve main file detection and update dependencies

## [0.1.1](https://github.com/davehorner/cargo-e/compare/e_crate_version_checker-v0.1.0...e_crate_version_checker-v0.1.1) - 2025-03-15

### Added

- version pushed.


## [0.1.0] - 2025-03-14
### Added
- Initial release of **e_crate_version_checker**.
- Core functionality to query crates.io for the latest version of a specified crate.
- Semantic version comparison using both the `semver` crate and a naive fallback method.
- Interactive update prompt for upgrading crates via `cargo install`.
- Support for optional features: `check-version`, `uses_reqwest`, `uses_serde`, and `uses_semver`.
- Utility functions for clipboard copying and automated crate structure creation.

