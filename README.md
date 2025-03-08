# cargo-e

<img id="screenshot" src="https://raw.githubusercontent.com/davehorner/cargo-e/refs/heads/develop/doc/media/screenshot-cargo-e.webp" 
     alt="Cargo-e Screenshot" title="Cargo-e running in terminal"
     onerror="this.onerror=null; this.src='../media/screenshot-cargo-e.webp';">

e is for Example. cargo-e is a Cargo subcommand for running and exploring examples, binaries, and source code in Rust projects. Unlike `cargo run --example`, it executes the example directly if only one exists.

## Most Important Features
- Runs a single example automatically when only one example is defined.
- Supports examples in different locations (bins, workspaces, etc.)

## Quick Start
```sh
cargo install cargo-e
cargo e
```

See the [GitHub repository](https://github.com/davehorner/cargo-e) for more details.

## Overview

**cargo-e** makes it easy to run and explore sample code from your Rust projects. Whether you are working with built-in examples, extended samples, or binaries, cargo-e provides a unified interface to quickly launch your code, inspect its structure, and integrate with editors/tools.

## Features

- **Runs the default example if there is only one example defined.**
- **Seamless Sample Execution:** Run built-in examples and extended samples (located in the `examples` directory) with a simple command. Improved discoverability of examples and binaries, even across workspaces.
- **Interactive Terminal UI (TUI):** Optionally launch a feature-rich, interactive interface for browsing and selecting targets. (-t option)
- **VSCode Integration:** Jump directly into your source code and navigate to the `fn main` entry point automatically. ('e' key in TUI)
- **bacon Integration:** Quickly run bacon on your project/example. ('b' key in TUI)
- **Workspace Integration:** Automatically detects and uses workspace manifests for multi-crate projects. (-w option)
- **Configurable behavior** – Optional equivalentibility mode – cargo-e can behave identically to `cargo run --example`.

## Introduction

When using `cargo run --example` in a project with a single example, Cargo does not execute the example. Instead of running the obvious example, it displays that there is one example available. This behavior differs from that of `cargo run`, which automatically runs the default build target without requiring additional arguments.

If you read `cargo --help`, you'll notice short keys such as `r` for run and `b` for build. **cargo-e** fills the 'e' gap by serving as a dedicated tool for examples. It functions similarly to `cargo run --example`; it takes the example name and passes arguments just like `--example`, but with the added benefit that it will automatically run the single example if that is the only one defined.

Running the single example if there is only one example defined is a primary feature of cargo-e; it's what brought about this project. `--example` and `--bin` are often parsed, so changing Cargo's behavior is out of the question. In fact, this tool relies upon `--example` returning the list of examples instead of running the single example.

Projects organize examples in different ways – some using binaries, others placing them in an `examples` directory – cargo-e helps navigate and execute targets across diverse structures. Whether your project uses bins, examples, or even workspace configurations, cargo-e unifies these scenarios and simplifies the process of running and exploring your sample code.

## Installation

Install cargo-e via Cargo:

```bash
cargo install cargo-e
```

Install cargo-e via git:

```bash
git clone https://github.com/davehorner/cargo-e 
cd cargo-e
cargo install --path .
```

## Usage

Run an example directly from your project:

```bash
cargo e [OPTIONS] [EXAMPLE] [-- extra arguments]
```

If there is only one example, it will run that example.

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

## Contributing

Contributions are welcome! If you have suggestions or improvements, feel free to open an issue or submit a pull request.

## License

This project is dual-licensed under the MIT License or the Apache License (Version 2.0), at your option.

## Repository

For source code, documentation, and updates, visit the [cargo-e repository](https://github.com/davehorner/cargo-e).

## Acknowledgements

- Built with the power of the Rust ecosystem and libraries like [clap](https://crates.io/crates/clap), [crossterm](https://crates.io/crates/crossterm) (optional), and [ratatui](https://crates.io/crates/ratatui) (optional).
- Special thanks to the Rust community and all contributors for their continued support.
