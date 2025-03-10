# cargo-e
[![Crates.io](https://img.shields.io/crates/v/cargo-e.svg)](https://crates.io/crates/cargo-e)

<img id="screenshot" src="https://raw.githubusercontent.com/davehorner/cargo-e/refs/heads/develop/doc/media/screenshot-cargo-e.webp" 
     alt="Cargo-e Screenshot" title="Cargo-e running in terminal"
     onerror="this.onerror=null; this.src='../media/screenshot-cargo-e.webp';">

e is for Example. cargo-e is a Cargo subcommand for running and exploring examples, binaries, and source code in Rust projects. Unlike `cargo run --example`, it executes the example directly if only one exists.

## Most Important Features
- Runs a single example automatically when only one example is defined.
- Supports examples in different locations (bins, workspaces, etc.)
- **cargo-e as an Example:**  
  cargo-e itself serves as a practical example of an attempt to write a well-managed Rust project. It adopts conventional commits and adheres to semantic versioning. The project leverages GitHub Actions to automate releases, generate a CHANGELOG, and handle versioning via [release-plz](https://release-plz.dev/docs/github/quickstart). As a learning vehicle for its creator, cargo-e also provides a model for others interested in effective coding and project management practices.

## Quick Start
```sh
cargo install cargo-e
cargo e
```

<div id="github-link">
  See the <a href="https://github.com/davehorner/cargo-e">GitHub repository</a> for more details.
</div>

<script>
  // Example condition: if the current URL contains "github.com"
  if (window.location.href.indexOf("github.com") !== -1) {
    document.getElementById("github-link").style.display = "none";
  }
</script>

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
  
## Features and Configuration

cargo-e leverages Cargo's feature flags to provide fine-grained control over the included components and dependencies. Using conditional compilation whenever possible, the dependency tree remains lean by including only what is necessary.

- **Default Features:**  
  Building cargo-e without specifying additional features enables the `tui` and `concurrent` features by default. Terminal UI support is provided via `crossterm` and `ratatui`, while concurrency support is offered through `threadpool`.

- **Optional and Platform-Specific Features:**  
  - **`windows`**: Includes Windows-specific dependencies to enhance compatibility on Windows systems and to limit unneeded energy, time, space on bloat.  
  - **`equivalent`**: Functions as an alias/shortcut for `--example` without enabling extra features.

- **Customizing the Build:**  
  Default features may be disabled using `--no-default-features`, and desired features can be enabled using `--features`. For example:

  ```bash
  cargo build --no-default-features --features tui
  ```

## Prior Art and Alternative Approaches

Several tools and techniques have been developed to ease the exploration and execution of example code in Rust projects:

- **Built-in Cargo Support:**  
  
    Cargo provides support for running examples with the `cargo run --example` command. However, this approach places the example at the level of an option, requiring users to type out a longer command—at least 19 characters per invocation—and, in many cases, two separate invocations (one for seeing and another to actually do something). This extra keystroke overhead can make the process less efficient for quick experimentation.

- **cargo-examples:**  
  
  The [cargo-examples](https://github.com/richardhozak/cargo-examples) project offers another approach to handling examples in Rust projects. It focuses on running all the examples in alphabetical order with options to start from a point in the list.  Simplifying the execution of example code, demonstrating a similar intent to cargo-e by reducing the overhead of managing example invocations.
    

    It handles various example structures:
    - **Single-file examples:** Located directly as `<project>/examples/foo.rs`
    - **Multi-file examples:** Structured as `<project>/examples/bar/main.rs`
    - **Manifest-based examples:** Defined in `Cargo.toml` using the `[[example]]` configuration
    - **Subproject examples:** Examples in subdirectories containing their own `Cargo.toml`, which standard Cargo commands cannot run out-of-the-box.
  
    - **Efficient Execution:**  
        Examples are run in alphabetical order, and the tool provides options such as `--from` to start execution at a specific example. This reduces the need for multiple long invocations and simplifies the workflow.
- **cargo-play:**  
              
  The [cargo-play](https://crates.io/crates/cargo-play) tool is designed to run Rust code files without  the need to manually set up a Cargo project, streamlining rapid prototyping and experimentation. Key aspects include:

  - **Ease of Use:**  
    Run Rust files directly with a simple command (`cargo play <files>`). External dependencies can be specified inline at the top of your file using the `//#` syntax, following the same TOML format as in `Cargo.toml`.

  - **Multi-file and Subdirectory Support:**  
    It supports running multiple files at once, and handles files located in subdirectories by copying them relative to the first file provided, enabling seamless execution of more complex code bases.

  - **Editor Integrations:**  
    With built-in support for editors like Vim, VS Code, and Micro, cargo-play enables you to test your current file directly from your development environment, enhancing workflow efficiency.

  - **Installation and Versatility:**  
    Installation is as simple as running `cargo install cargo-play`, making it an accessible and lightweight option for quickly executing and experimenting with Rust code without the overhead of a full project setup.


## Contributing

Contributions are welcome! If you have suggestions or improvements, feel free to open an issue or submit a pull request.

## License

This project is dual-licensed under the MIT License or the Apache License (Version 2.0), at your option.

## Repository

For source code, documentation, and updates, visit the [cargo-e repository](https://github.com/davehorner/cargo-e).

## Acknowledgements

- Built with the power of the Rust ecosystem and libraries like [clap](https://crates.io/crates/clap), [crossterm](https://crates.io/crates/crossterm) (optional), and [ratatui](https://crates.io/crates/ratatui) (optional).
- Special thanks to the Rust community and all contributors for their continued support.
