# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.18](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.17...cargo-e-v0.1.18) - 2025-03-18

### Added

- add workaround for Cargo workspace misinterpretation and update documentation

## [0.1.17](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.16...cargo-e-v0.1.17) - 2025-03-18

### Added

- **Auto-resolve workspace errors:** When a Cargo package is mistakenly treated as part of a workspace, commands (like `cargo run`) now automatically detect the error and temporarily patch the manifest (by appending an empty `[workspace]` table) before executing the command. The original manifest is restored afterward. This behavior has been implemented for both CLI and TUI modes.

### Changed

- Updated target collection and run routines to use the new manifest patching mechanism, ensuring a smoother user experience without manual intervention.


## [0.1.16](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.15...cargo-e-v0.1.16) - 2025-03-17

### Added

- *(cli)* add partial search fallback for explicit example matching

## [0.1.15](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.14...cargo-e-v0.1.15) - 2025-03-17

### Other

- extended samples are now showing.

## [0.1.14](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.13...cargo-e-v0.1.14) - 2025-03-16

### Added

- require register_user_crate! call and update Windows self-update

### Other

- temp
- disable tests for version checking.
- fix unix/windows imports

## [0.1.13](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.12...cargo-e-v0.1.13) - 2025-03-16

### Added
- Enhanced interactive prompt to automatically switch between single-character input and full-line input based on the number of available targets.
- Displayed run history counts next to target names for improved user guidance.
- Introduced refined default argument behavior that goes beyond simply listing targets.

### Improved
- Refactored prompt logic and interactive CLI for clearer, more maintainable code.
- Simplified dependency imports and performed minor code cleanups across modules.
- *(cli, ctrlc, run-history)* add once_cell, global Ctrl+C handler & interactive paging

## [0.1.12](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.11...cargo-e-v0.1.12) - 2025-03-16

### Improved
- **Target Resolution in cargo-e:** The tool now prioritizes matching explicit targets by first checking the discovered examples and then binaries. This change ensures a smoother and more predictable user experience when specifying an explicit target.

### Added
- **e_update_readme Tool:** A new project under the addenda directory that updates version strings in README.md files by reading Cargo.toml. This tool can be leveraged by other projects to maintain consistent version information. When used with the `-p` flag, it updates the parent's README.md only if its content matches the local file.

### Fixed
- **Parent README.md Update Safety:** When the `-p` flag is provided, the tool now compares the parent's README.md with the local version before updating. If discrepancies are found, the update is aborted to avoid unintentional overwrites.


## [0.1.11](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.10...cargo-e-v0.1.11) - 2025-03-15

### Added

- implement self-update functionality for Windows (spawn batch) in

## [0.1.10](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.9...cargo-e-v0.1.10) - 2025-03-15

### Added

- add interactive prompts and update run flow

## [0.1.9](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.8...cargo-e-v0.1.9) - 2025-03-15

### Added

- better sample resolution and findmain support

## [0.1.8](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.7...cargo-e-v0.1.8) - 2025-03-15

### Added

- improve main file detection and update dependencies

## [0.1.7](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.6...cargo-e-v0.1.7) - 2025-03-15

### Other

- make find_main work better with workspaces.  enable auto

## [0.1.6](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.5...cargo-e-v0.1.6) - 2025-03-14

### Added

- *(cargo)* add e_crate_version_checker dependency and restructure

### Other

- temp
- update Cargo.toml to use resolver v2 and change naive_is_newer to public in e_crate_update.rs; add LICENSE files and initial README for cargo-e; include CHANGELOG.md
- clean up example code and comments in version checker files

## [0.2.0](https://github.com/davehorner/cargo-e/compare/v0.1.5...v0.2.0) - 2025-03-13

### Added

- *(addendum, docs, deps, cli)* [**breaking**] new version checking/interactive upgrade e_crate_version_checker addendum HORNER02

### Other

- update version and improve README documentation

## [0.1.5](https://github.com/davehorner/cargo-e/compare/v0.1.4...v0.1.5) - 2025-03-11

chore: update version and improve README documentation

- Bumped version in Cargo.toml from "0.1.4" to "0.1.5".
- Enhanced README.md with an updated badge display and added version notice for documentation.
- Improved image loading and error handling in README.md to provide fallback messages.
- Cleared CLIPPY_ARGS in support/checks.sh for cleaner integration.


## [0.1.4](https://github.com/davehorner/cargo-e/compare/v0.1.3...v0.1.4) - 2025-03-11

fix(docs): update README and examples for clarity and consistency
feat(bacon): integrate bacon tool configuration and checks
feat(quality): add quality assurance support script `support/checks.sh`

- Create `bacon.toml` to define job commands for building, linting (clippy), testing, and generating documentation.
- Add `support/checks.sh` to automate common checks and launch commands in a new terminal for both `bacon` and `cbacon`.
- Revise `README.md` to improve descriptions of execution options and overall format.
- Enhance `examples/funny_example.rs` with detailed comments and a test that illustrates compiler behavior with unused constants.
- Update test documentation in `src/a_funny_docs.rs` and add new tests for improved clarity.
- Refactor comments and argument handling in `src/e_bacon.rs`, `src/e_collect.rs`, and related files to boost readability.
- Streamline example handling and environment settings in `src/main.rs` and test files to simplify setup and ensure correctness.

### Fixed

- *(docs)* update README and examples for clarity and consistency

## [0.1.3](https://github.com/davehorner/cargo-e/compare/v0.1.2...v0.1.3) - 2025-03-10

### Other

- update Cargo.toml dependencies; added samples funny-docs for future testing. Happy Coding!

## [0.1.2](https://github.com/davehorner/cargo-e/compare/v0.1.1...v0.1.2) - 2025-03-10

### Other

- update permissions in release-plz.yml to allow write access for pull-requests and contents

## [0.1.1](https://github.com/davehorner/cargo-e/compare/v0.1.0...v0.1.1) - 2025-03-10

### Added

- feat: add anyhow dependency and implement command builder with target discovery
- add anyhow dependency and implement command builder with target discovery
- work on move towards targets of the test,bin,example,manifest kind.
- getting testing started.

## [0.1.0](https://github.com/davehorner/cargo-e/releases/tag/v0.1.0) - 2025-03-09

### Added

- equivalent feature flag for simple shortcut with no improvements
- *(aipack)* add crate recreator tool, config, and release workflow
- initial commit of cargo-e Cargo subcommand

### Other

- improve logging, formatting, and comments across modules
- skip bacon tests in GitHub workflow
- update README.md for clarity and improved grammar
- add Rust GitHub Actions workflow
- cleanup
