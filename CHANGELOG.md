# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
