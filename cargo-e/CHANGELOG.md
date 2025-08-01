# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.46](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.45...cargo-e-v0.2.46) - 2025-07-20

### Other

- update Cargo.lock dependencies

## [0.2.45](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.44...cargo-e-v0.2.45) - 2025-07-14

### Fixed

- *(cargo-e)* avoid recursion in workspace parsing and improve process termination reliability

## [0.2.44](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.43...cargo-e-v0.2.44) - 2025-06-25

### Fixed

- *(target)* preserve name "main" for top-level example target

## [0.2.43](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.42...cargo-e-v0.2.43) - 2025-06-19

### Added

- *(detached)* add support for detached mode with hold and delay options

## [0.2.42](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.41...cargo-e-v0.2.42) - 2025-06-13

### Added

- *(cli)* add --scan-dir flag to recursively discover and run targets in subdirectories

## [0.2.41](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.40...cargo-e-v0.2.41) - 2025-06-12

### Added

- add new CLI arguments for manifest path, target, and JSON output in cargo-e; if explicit example is specified, it will not continue to a cli loop; remove debug output I missed.

## [0.2.40](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.39...cargo-e-v0.2.40) - 2025-06-09

### Added

- improve process tracking, timeouts, and logging in cargo-e

## [0.2.39](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.38...cargo-e-v0.2.39) - 2025-06-07

### Other

- update Cargo.lock dependencies

## [0.2.38](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.37...cargo-e-v0.2.38) - 2025-06-06

### Other

- add e_obs script for OBS control and recording functionality https://www.youtube.com/watch?v=5BXStX87Z0o

## [0.2.37](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.36...cargo-e-v0.2.37) - 2025-06-03

### Added

- add support for parsing available targets from stdin and default binary as runner option. wgpu\examples\features has a default binary that operates like cargo --example.

## [0.2.36](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.35...cargo-e-v0.2.36) - 2025-06-02

### Added

- add cached option to CLI and update command builder logic to use cache functionality.  Runs the executable directly if it exists.

## [0.2.35](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.34...cargo-e-v0.2.35) - 2025-06-01

### Fixed

- remove system information from status lines.  double up \r to ensure status line is properly displayed.

## [0.2.34](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.33...cargo-e-v0.2.34) - 2025-06-01

### Added

- add thread-local context and CLI flags for status, TTS, and window control

## [0.2.33](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.32...cargo-e-v0.2.33) - 2025-05-31

### Added

- *(e-window,failed_build_window)* Implement anchor links in e_window for launching code on error lines.  Failed builds now include a graphical window which includes just the errors.

## [0.2.32](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.31...cargo-e-v0.2.32) - 2025-05-29

### Other

- update Cargo.lock dependencies

## [0.2.31](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.30...cargo-e-v0.2.31) - 2025-05-26

### Added

- *(window_panics)* Integrate `e_window` for graphical panics

## [0.2.30](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.29...cargo-e-v0.2.30) - 2025-05-25

### Added

- Add Text-to-Speech (TTS) for panic messages
- when single target; if there are extra arguments provided, run the target directly.

## [0.2.29](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.28...cargo-e-v0.2.29) - 2025-05-21

### Added

- *(run-all)* support parallel execution with --run-at-a-time and custom CLI parsing

## [0.2.28](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.27...cargo-e-v0.2.28) - 2025-05-19

### Other

- updated the following local packages: e_crate_version_checker - using LAST_RELEASE via github for update check.

## [0.2.27](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.26...cargo-e-v0.2.27) - 2025-05-17

### Added

- `cargo e -s i` install . or explicit path; this filters the output

## [0.2.26](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.25...cargo-e-v0.2.26) - 2025-05-17

### Added

- panic and backtrace treatments. add is_could_not_compile field to track compilation failures and improve diagnostics handling in CargoCommandExt and related components

## [0.2.25](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.24...cargo-e-v0.2.25) - 2025-05-09

### Added

- add quality check scripts for Cargo project with qc.cmd and qc_cap.cmd files

### Other

- *(diagnostics)* print allow statements instead of warn. print urls instead of for more information, see. '|', '^', '-', '_'  condensed to a single line comment. full diag padding and other formatting. fix issue with targets that were not builtin example or binary executing when -s specified

## [0.2.24](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.23...cargo-e-v0.2.24) - 2025-05-08

### Fixed

- a panic!

## [0.2.23](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.22...cargo-e-v0.2.23) - 2025-05-08

### Fixed

- -s subcommand bypass prompt. fix extra args and features being

## [0.2.22](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.21...cargo-e-v0.2.22) - 2025-05-03

### Added

- *(plugins)* update script detection and command building

### Other

- *(e_runner)* remove unused extra args handling in run_example function

## [0.2.21](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.20...cargo-e-v0.2.21) - 2025-04-30

### Other

- callbacks can set build_finished_time; leptos --run-all -f

## [0.2.20](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.19...cargo-e-v0.2.20) - 2025-04-30

### Added

- add --gist to use github cli to create run_report gists.

## [0.2.19](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.18...cargo-e-v0.2.19) - 2025-04-29

### Added

- *(run_report)* generate run_report.md on exit.  includes diagnostic

## [0.2.18](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.17...cargo-e-v0.2.18) - 2025-04-29

### Added

- cargo error detection, tauri pnpm/npm install/build automation, automate tool runner installations.

## [0.2.17](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.16...cargo-e-v0.2.17) - 2025-04-20

### Added

- *(tauri)* Improved find_manifest_dir function to handle src-tauri/Cargo.toml fix warnings.

## [0.2.16](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.15...cargo-e-v0.2.16) - 2025-04-20

### Added

- *(cargo-e)* add plugin support to handle plugin-provided targets; dev release only not enabled by default

### Other

- add readme and update versions that were missed in 0.2.15

## [0.2.15](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.14...cargo-e-v0.2.15) - 2025-04-20

### Added

- *(e_crate_version_checker)* fortune and changelog features added.

## [0.2.14](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.13...cargo-e-v0.2.14) - 2025-04-17

### Added

- rust-script / scriptisto kind detection and filtering. first cargo-e lib egui rust-script in experiments

## [0.2.13](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.12...cargo-e-v0.2.13) - 2025-04-15

### Fixed

- rust-script/scriptisto logical error, if 1 binary exists, the

## [0.2.12](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.11...cargo-e-v0.2.12) - 2025-04-13

### Added

- *(cargo-e)* add realtime filtering for cargo stdout and stderr

## [0.2.11](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.10...cargo-e-v0.2.11) - 2025-04-11

### Fixed

- DefaultBinary will use package name if one exists.  Looking for

## [0.2.10](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.9...cargo-e-v0.2.10) - 2025-04-05

### Added

- *(scriptisto)* integrate scriptisto support with enhanced Ctrl+C handling

## [0.2.9](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.8...cargo-e-v0.2.9) - 2025-04-05

### Fixed

- *(leptos)* cargo leptos executes in manifest path.

### Other

- set MSRV

## [0.2.8](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.7...cargo-e-v0.2.8) - 2025-04-03

### Added

- *(leptos)* add support for leptos projects

## [0.2.7](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.6...cargo-e-v0.2.7) - 2025-04-01

### Added

- *(ctrlc)* allow triple Ctrl+C to exit when no child is running

## [0.2.6](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.5...cargo-e-v0.2.6) - 2025-04-01

### Added

- *(rust-script)* runs rust-script given valid existing rust-file script. scripts must # bang and rust-script on the first line.

## [0.2.5](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.4...cargo-e-v0.2.5) - 2025-03-31

### Added

- *(tui)* display #, kind, and runs.

## [0.2.4](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.3...cargo-e-v0.2.4) - 2025-03-29

### Other

- updated the following local packages: e_ai_summarize

## [0.2.3](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.2...cargo-e-v0.2.3) - 2025-03-29

### Other

- update Cargo.lock dependencies

## [0.2.2](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.1...cargo-e-v0.2.2) - 2025-03-28

### Added

- *(tui)* improve arrow key handling and update dioxus detection - ESC no longer exits application.

## [0.2.1](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.2.0...cargo-e-v0.2.1) - 2025-03-28

### Added

- *(ai-summarization)* integrate GenAI-powered Rust code summarizer

## [0.2.0](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.24...cargo-e-v0.2.0) - 2025-03-26

### Added

- [**breaking**] dioxus and tauri target detection, custom Cargo.toml discovery

## [0.1.24](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.23...cargo-e-v0.1.24) - 2025-03-23

### Added

- add support for required features in manifest handling

## [0.1.23](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.22...cargo-e-v0.1.23) - 2025-03-23

### Other

- improve input handling in prompt_line_with_poll_opts and

## [0.1.22](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.21...cargo-e-v0.1.22) - 2025-03-23

### Added

- *(cli)* add new --run-all flag for configurable run duration

## [0.1.21](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.20...cargo-e-v0.1.21) - 2025-03-22

### Added

- *(cli)* add relative numbering option and support for extended targets in e_collect.rs and e_runner.rs

## [0.1.20](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.19...cargo-e-v0.1.20) - 2025-03-22

### Added

- enhance prompt functionality to skip empty messages and clear

### Other

- improve user prompt handling and conditionally display messages in e_prompts.rs and main.rs

## [0.1.19](https://github.com/davehorner/cargo-e/compare/cargo-e-v0.1.18...cargo-e-v0.1.19) - 2025-03-19

### Added

- *(cli,tui)* add print flags and enhance CLI/TUI interactions

### Other

- update README files to include cargo-e walkthrough GIFs

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
- *(addendum, docs, deps, cli)* [**breaking**] new version checking/interactive upgrade e_crate_version_checker addendum HORNER02
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
