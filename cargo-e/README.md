<a href="https://crates.io/crates/cargo-e" rel="nofollow noopener noreferrer">
  <img src="https://img.shields.io/crates/v/cargo-e.svg" alt="Crates.io">
</a>

<!-- Version notice -->
<p style="font-style: italic; color: #ccc; margin-top: 0.5em;">
  You are reading documentation version <span id="doc-version" style="color: white;">0.2.46</span>.
  If this does not match the version displayed above, then you're not reading the latest documentation.
</p>
<img id="screenshot"
     src="https://raw.githubusercontent.com/davehorner/cargo-e/refs/heads/develop/documents/media/screenshot-cargo-e.webp"
     alt="Cargo-e Screenshot" title="Cargo-e running in terminal"
     onload="
       // When the image loads (which happens locally), check if we're not on GitHub.
       if (!window.location.hostname.includes('github') && !this.dataset.messageInserted) {
         this.insertAdjacentHTML('afterend', ' <div style=\'color: #ccc; font-style: italic;\'>See the <a href=\'https://github.com/davehorner/cargo-e\' target=\'_blank\'>GitHub repository</a> for more details.</div>');
         this.dataset.messageInserted = 'true';
       }
     "
     onerror="
       // First error: if remote image fails, try loading the local image.
       if (this.src.indexOf('../media/screenshot-cargo-e.webp') === -1) {
         this.onerror = this.onerror;
         this.src = '../media/screenshot-cargo-e.webp';
       } else if (!window.location.hostname.includes('github')) {
         // If the local image also fails and we're not on GitHub, insert the fallback message.
         this.insertAdjacentHTML('afterend', ' <div style=\'color: #ccc; font-style: italic;\'>See the <a href=\'https://github.com/davehorner/cargo-e\' target=\'_blank\'>GitHub repository</a> for more details.</div>');
       }
     ">


e is for Example. `cargo-e` is a Cargo subcommand for running and exploring examples, binaries, and source code in Rust projects. Unlike `cargo run --example`, it executes the example directly if only one exists.

## Most Important Features
- Runs a single example automatically when only one example is defined.
- Supports examples in different locations (bins, workspaces, etc.), detects [tauri](https://github.com/tauri-apps/tauri), [dioxus](https://github.com/DioxusLabs/dioxus), and [leptos](https://github.com/leptos-rs/leptos) projects/examples.
- **`cargo-e` as an Example:**  
  `cargo-e` itself serves as a practical example of an attempt to write a well-managed Rust project. It adopts conventional commits and adheres to [SemVer](https://semver.org/). The project leverages GitHub Actions to automate releases, generate a [CHANGELOG](CHANGELOG.md), and handle versioning via [release-plz](https://release-plz.dev/docs/github/quickstart). As a learning vehicle for its creator, `cargo-e` also provides a model for others interested in effective coding and project management practices.

## Quick Start

```sh
  cargo install cargo-e
  cd cool_examples
  cargo e
```


## Overview

**`cargo-e`** makes it easy to run and explore sample code from your Rust projects. Whether you are working with built-in examples, extended samples, or binaries, `cargo-e` provides a unified interface to quickly launch your code, inspect its structure, and integrate with editors/tools.

## Features
- **runs the default example if there is only one example defined.**
- **ai summarization:** info or 'i' sends the example code for summarization; including a YES or NO answer on if the code is safe to run.  Interactive follow up questions may be asked to allow for concept/code exploration.
- **partial search for matching:** If an explicit name isn't found, a case-insensitive partial search is performed to list matching targets for user selection. one command for binaries and examples.
- **rust-script/scriptisto:** If an explicit name is a file, the first line is checked for a valid bang # [rust-script](https://github.com/fornwall/rust-script)/[scriptisto](https://github.com/igor-petruk/scriptisto).  `cargo e` runs your scripts and is less typing.
- **framework sample support:** detects [tauri](https://github.com/tauri-apps/tauri), [dioxus](https://github.com/DioxusLabs/dioxus), [leptos](https://github.com/leptos-rs/leptos) projects; running the target calls the associated framework runner.
- **seamless sample execution:** Run built-in examples and extended samples (located in the `examples` directory) with a simple command. Improved discoverability of examples and binaries, even across workspaces.
- **automatic required feature detection:** Examples and binaries marked with required features will automatically have those features applied. `cargo-e` takes care of that hassle for you.
- **complete view of all targets:** Not just for binaries and examples. Tests and bench targets too.
- **interactive terminal UI (TUI):** Optionally launch a feature-rich, interactive interface for browsing and selecting targets. (-t option)
- **[vscode](https://github.com/microsoft/vscode) integration:** Jump directly into your source `code` and navigate to the `fn main` entry point automatically. ('e' key in TUI)
- **[bacon](https://github.com/Canop/bacon) integration:** Run `bacon` on your project/example. ('b' key in TUI)
- **workspace integration:** Automatically detects and uses workspace manifests for multi-crate projects. (-w option)
- **configurable behavior:** – Optional equivalent mode – `cargo-e` can behave identically to `cargo run --example` with bare minimum dependency
- **cargo and target stderr and stdout filtering:** `-f` sends all output from cargo and the target through a filter to determine accurate timing when `--run-all` is specified.  cargo warnings and errors are rewritten to be more concise, numbered, and timed format. Errors are written in realtime and a table of errors is displayed conviently at the end of output,  file references are all absolute and fully specified so your ctrl+clicks take you there.  If you require a terminal, don't use `-f`, and your output will be unfiltered.
- **subcommands:** you may find that you like the rewritten cargo output and the additional detail provided in `-f` filtering.  Specify a `-s` subcommand to run a subcommand other than the default `run` that `cargo-e` uses normally.
- **autosense/tool installer:** `cargo-e` will parse the output of a failed cargo builds and prompt to suggest the user to install the required library or tool runner.
- **run_report.md:** on exit, a run_report.md is generated which includes details of the commands run and diagnostic information if the `-f` filtering is enabled.
- **tts panics:** When a panic is detected, cargo-e will speak the panic message aloud using text-to-speech (TTS) for immediate feedback. `-f` required.
- **graphical panics:** Panics are also displayed in a graphical window using [e_window](https://crates.io/crates/e_window), providing a clear and interactive error report. `-f` required.
- **graphical failed build:** A failed build displays a graphical window using [e_window](https://crates.io/crates/e_window); Errors are anchors and clicking them opens code directly to the error line. `-f` required.
- **cached builds:**  
  When the `--cached` flag is used, `cargo-e` will attempt to reuse previously built artifacts instead of rebuilding examples or binaries from scratch.
- **`--json-all-targets`**:  
  Outputs a comprehensive JSON list of all discovered targets (examples, binaries, tests, benches, etc.) in the project. This is useful for tooling, scripting, or integration with editors and CI systems. The JSON includes metadata such as target names, types, required features, and paths, enabling automated processing or custom workflows.
- **`--scan-dir <DIR>`:**  
  Scan a specific directory recursively for Rust targets (examples, binaries, etc.) outside the current project or workspace.
- **detached execution and options:**  
  Run targets in detached mode using the `--detached` flag, which launches each target in a separate terminal window (e.g., `cmd /c start` or `xterm/alacritty/terminal`). Additional options include `--detached-hold <SECONDS>` to specify how long the detached window remains open after execution, and `--detached-delay <SECONDS>` to delay execution after opening the window. This is useful for running multiple targets concurrently or keeping output visible after completion.

## Introduction

When using `cargo run --example` in a project with a single example, Cargo does not execute the example. Instead of running the obvious example, it displays that there is one example available. This behavior differs from that of `cargo run`, which automatically runs the default build target without requiring additional arguments.

If you read `cargo --help`, you'll notice short keys such as `r` for run and `b` for build. **`cargo-e`** fills the `e` gap by serving as a dedicated tool for examples. It functions similarly to `cargo run --example`; it takes the example name and passes arguments just like `--example`, but with the added benefit that it will automatically run the single example if that is the only one defined.

Running the single example if there is only one example defined is a primary feature of `cargo-e`; it's what brought about this project. `--example` and `--bin` are often parsed, so changing Cargo's behavior is out of the question. In fact, this tool relies upon `--example` returning the list of examples instead of running the single example.

Projects organize examples in different ways – some using binaries, others placing them in an `examples` directory – `cargo-e` helps navigate and execute targets across diverse structures. Whether your project uses bins, examples, or even workspace configurations, `cargo-e` unifies these scenarios and simplifies the process of running and exploring your sample code.

## Installation

Install `cargo-e` via Cargo:

```bash
cargo install cargo-e
```

Install `cargo-e` via git:

```bash
git clone https://github.com/davehorner/cargo-e 
cd cargo-e
cargo install --path .
```

## Usage

Run an example directly from your project:

```bash
Usage: cargo-e [OPTIONS] [EXPLICIT_EXAMPLE] [-- <EXTRA>...]

Arguments:
  [EXPLICIT_EXAMPLE]  Specify an explicit target to run.
  [EXTRA]...          Additional arguments passed to the command.

Options:
      --stdout <PATH>                  Path to read/write the stdout of the executed command.
      --stderr <PATH>                  Path to read/write the stderr of the executed command.
      --run-all [<RUN_ALL>]            Run all optionally specifying run time (in seconds) per target. If the flag is present without a value, run forever. [default: not_specified]
      --gist                           Create GIST run_report.md on exit.
      --release                        Build and run in release mode.
  -q, --quiet                          Suppress cargo output when running the sample.
      --pre-build                      If enabled, pre-build the examples before executing them.
      --cached                         If enabled, execute the existing target directly.
      --detached                       Run the targets in detached mode. (cmd /c show | alacritty)
      --scan-dir <DIR>                 Scan the given directory for targets to run.
  -f, --filter                         Enable filter mode. cargo output is filtered and captured.
  -v, --version                        Print version and feature flags in JSON format.
  -t, --tui                            Launch the text-based user interface (TUI).
  -w, --workspace                      Operate on the entire workspace.
      --pX                             Print the exit code of the process when run. (default: false)
      --pN                             Print the program name before execution. (default: false)
      --pI                             Print the user instruction. (default: true)
  -p, --paging                         Enable or disable paging (default: enabled).
  -r, --relative-numbers               Relative numbers (default: enabled).
  -W, --wait <WAIT>                    Set wait time in seconds (default: 15). [default: 15]
  -s, --subcommand <SUBCOMMAND>        Specify subcommands (e.g., `build|b`, `test|t`). [default: run]
  -J, --run-at-a-time <RUN_AT_A_TIME>  Number of targets to run at a time in --run-all mode (--run-at-a-time) [default: 1]
      --nS                             Disable status lines during runtime loop output.
      --nT                             Disable text-to-speech output.
      --parse-available                Parse available targets from stdin (one per line).
      --default-binary-is-runner       If enabled, treat the default binary as the runner for targets.
      --nW                             Disable window popups.
      --log <PATH>                     Enable logging to a file at the given path, or to stdout if not specified.
      --manifest-path <PATH>           Specify the path to the Cargo.toml manifest file.
      --target <TARGET>                Specify the target triple for the build.
      --json-all-targets               Output the list of all targets as JSON.
      --detached-hold <SECONDS>        Time in seconds to keep detached windows open before killing.
      --detached-delay <SECONDS>       Time in seconds for detached windows to delay before executing target
  -h, --help 
```

If there is only one example, it will run that example, did I mention that already?

### Command-line Options

- `-t, --tui`  
  Launch the interactive terminal UI for selecting an example or binary.
  
- `-w, --workspace`  
  Use the workspace manifest (the root `Cargo.toml` of your workspace) instead of the current directory.

- `-W, --wait <seconds>`  
  Specify how many seconds to wait after the target process finishes so you can view its output.

### Examples

- **Run a specific example:**

  ```bash
  cargo e my_example -- --flag1 value1
  ```

- **Launch the TUI:**

  ```bash
  cargo e --tui
  ```

- **Use workspace mode:**
  ```bash
  cargo e --workspace
  ```

- **Partial Searches**
  
  ```bash
  cargo e wgpu
  builtin: rust/nannou/Cargo.toml
  0 built-in examples (213 alternatives: 208 examples, 5 binaries).
  error: 0 named 'wgpu' found in examples or binaries.
  partial search results for 'wgpu':
    1: [ex.] wgpu_compute_shader
    2: [ex.] wgpu_image
    3: [ex.] wgpu_image_sequence
    4: [ex.] wgpu_instancing
    5: [ex.] wgpu_teapot
    6: [ex.] wgpu_teapot_camera
    7: [ex.] wgpu_triangle
    8: [ex.] wgpu_triangle_raw_frame
  * == # to run, tui, e<#> edit, 'q' to quit (waiting 5 seconds)
  ```
- **Run All**
  
  Execute all discovered examples or binaries in your project in a single run.  Use a partial search to run only those matching targets.
  ```bash
  cargo e --run-all --quiet --release [partial_search_term]
  ```
  `--run-all` by itself (without a numeric value) 

     will set the run mode to "forever"—meaning each target is allowed to run until it terminates naturally.

  `--run-all 10`
    
     means that each target will be run for 10 seconds. After 10 seconds, Cargo-e will prompt for a key press; if no key is pressed (or if a non-quit key is pressed), the running process will be killed, and the next target is started.

  `startt -f -g1x4 cargo e --run-all --run-at-a-time 4`
    
     cargo-e can be called by [startt](https://crates.io/crates/startt) to run the targets in a grid; given `--run-at-a-time` argument it will run the targets concurrently and `startt` will position the windows in a grid.


Displays detailed help information. Use the -h option for additional details on all available flags.


## The `--scan-dir` Option

The `--scan-dir <DIR>` option allows you to specify a directory to scan for Rust targets (examples, binaries, etc.) outside of the current project or workspace. This is useful if you want to run examples or binaries located in a different directory structure, or if your project organizes targets in non-standard locations.

**Usage Example:**

```bash
cargo e --scan-dir path/to/other/examples
```

This command will search the specified directory for valid Rust targets and present them for selection or execution, just as if they were part of your main project.

**Typical Use Cases:**
- Scan all your projects and see them all together.
- Running examples from a shared or external examples directory.
- Exploring binaries or tests in subdirectories not covered by the main `Cargo.toml`.
- Integrating with custom project layouts or monorepos - keeping generated files in cwd and targets executed from their manifest parent.

If you combine `--scan-dir` with other options (like `--tui` or `--run-all`), `cargo-e` will operate on the discovered targets within the specified directory.  You can use it with `--json-all-targets` to get a json array of all the targets in the scanned directories.

## Features and Configuration

`cargo-e` leverages Cargo's feature flags to provide fine-grained control over the included components and dependencies. Using conditional compilation whenever possible, the dependency tree remains lean by including only what is necessary.

- **Default Features:**  
  Building `cargo-e` without specifying additional features enables the `tui` and `concurrent` features by default. Terminal UI support is provided via `crossterm` and `ratatui`, while concurrency support is offered through `threadpool`.

- **Optional and Platform-Specific Features:**  
  - **`windows`**: Includes Windows-specific dependencies to enhance compatibility on Windows systems and to limit unneeded energy, time, space on bloat.  
  - **`equivalent`**: Functions as an alias/shortcut for `--example` without enabling extra features.

- **Customizing the Build:**  
  Default features may be disabled using `--no-default-features`, and desired features can be enabled using `--features`. For example:

  ```bash
  cargo build --no-default-features --features tui
  ```
## Want to stop the version check prompts and queries?
By default, cargo-e bundles the [e_crate_version_checker](addendum/e_crate_version_checker) crate through the "check-version" feature. This means that when you run cargo-e, it performs a version check on startup and prompts you if a newer version is available. This helps keep you informed about the latest and greatest, but it also serves as a safeguard to prevent legacy builds from being used inadvertently. It may feel intrusive or annoying for some.

If you prefer to avoid that automatic version check, additional output, delay, and process, you can disable the default features during installation and then re-enable only the ones you want (like "tui", "concurrent", "funny-docs", "uses_reqwest", and "uses_serde"). Use the following command:

```bash
cargo install cargo-e --no-default-features --features "tui concurrent funny-docs uses_reqwest uses_serde"
```
This command installs cargo-e without the "check-version" feature, ensuring that no version check or upgrade prompt occurs at runtime. The funny-docs are a joke to be filled in.  The joke is that a user would actually open `rust docs --open` and read the funny or find a guide worth reading.  It is the default docs.  Did you read the guide?

Note: Disabling the version check means you forgo a mechanism designed to ensure that you’re not using less desirable builds.

## Work Arounds for Cargo

- **workspace package believes it's in a workspace when it's not**
  - [auto-resolve-workspace-errors.md](documents/auto-resolve-workspace-errors.md)

## Contributing

  Contributions are welcome! If you have suggestions or improvements, feel free to open an issue or submit a pull request.

## License

  This project is dual-licensed under the MIT License or the Apache License (Version 2.0), at your option.

## Acknowledgements

- Built with the power of the Rust ecosystem and libraries like [clap](https://crates.io/crates/clap), [crossterm](https://crates.io/crates/crossterm) (optional), and [ratatui](https://crates.io/crates/ratatui) (optional).
- Special thanks to the Rust community and all contributors for their continued support.

[![cargo-e_walkthru](https://github.com/davehorner/cargo-e_walkthrus/blob/main/cargo-e_walkthru_nu-ansi-term.gif?raw=true)](https://github.com/davehorner/cargo-e_walkthrus/tree/main)


<a href="https://crates.io/crates/cargo-e" rel="nofollow noopener noreferrer">
  <img src="https://img.shields.io/crates/v/cargo-e.svg" alt="Crates.io">
</a>

<!-- Version notice -->
<p style="font-style: italic; color: #ccc; margin-top: 0.5em;">
  You are reading documentation version <span id="doc-version" style="color: white;">0.2.46</span>.
  If this does not match the version displayed above, then you're not reading the latest documentation.
</p>

## Addendum

- [documents/1_Cool_Examples_With_Cargo_e.md](https://github.com/davehorner/cargo-e/blob/develop/documents/1_Cool_Examples_With_Cargo_e.md)

- HORNER EXAMPLE 1: 
  [examples/funny_examples.rs](https://github.com/davehorner/cargo-e/blob/develop/examples/funny_example.rs)

- HORNER EXAMPLE 2: 
  [addendum/e_crate_version_checker/src/main.rs](https://github.com/davehorner/cargo-e/blob/develop/addendum/e_crate_version_checker/src/main.rs)

- HORNER EXAMPLE 3: 
  [addenda/e_update_readme/src/bin/e_update_readme.rs](https://github.com/davehorner/cargo-e/tree/develop/addenda/e_update_readme)
