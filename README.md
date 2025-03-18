<a href="https://crates.io/crates/cargo-e" rel="nofollow noopener noreferrer">
  <img src="https://img.shields.io/crates/v/cargo-e.svg" alt="Crates.io">
</a>

<!-- Version notice -->
<p style="font-style: italic; color: #ccc; margin-top: 0.5em;">
  You are reading documentation version <span id="doc-version" style="color: white;">0.1.18</span>.
  If this does not match the version displayed above, then you're not reading the latest documentation!
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
- Supports examples in different locations (bins, workspaces, etc.)
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
- **partial search for matching:** If an explicit name isn’t found, a case-insensitive partial search is performed to list matching targets for user selection. one command for binaries and examples.
- **seamless sample execution:** Run built-in examples and extended samples (located in the `examples` directory) with a simple command. Improved discoverability of examples and binaries, even across workspaces.
- **interactive terminal UI (TUI):** Optionally launch a feature-rich, interactive interface for browsing and selecting targets. (-t option)
- **[vscode](https://github.com/microsoft/vscode) integration:** Jump directly into your source `code` and navigate to the `fn main` entry point automatically. ('e' key in TUI)
- **[bacon](https://github.com/Canop/bacon) integration:** Run `bacon` on your project/example. ('b' key in TUI)
- **Workspace integration:** Automatically detects and uses workspace manifests for multi-crate projects. (-w option)
- **Configurable behavior** – Optional equivalent mode – `cargo-e` can behave identically to `cargo run --example` with bare minimum dependency.

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


## Prior Art and Alternative Approaches

Several tools and techniques have been developed to ease the exploration and execution of example code in Rust projects:

- **Built-in Cargo Support:**  
  
    Cargo provides support for running examples with the `cargo run --example` command. However, this approach places the example at the level of an option, requiring users to type out a longer command—at least 19 characters per invocation—and, in many cases, two separate invocations (one for seeing and another to actually do). This extra keystroke overhead can make the process less efficient for quick experimentation.

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
              
  The [cargo-play](https://crates.io/crates/cargo-play) tool is designed to run Rust code files without  the need to manually set up a Cargo project, streamlining rapid prototyping and experimentation.
  
## The Wild West of Code Organization

Many developers create their own custom scripts or tools to expose examples and binaries, leading to several issues:

  - **Inconsistency Across Projects:** Each project tends to have its own unique implementation. This inconsistency means that even though some crates offer excellent examples, these examples are often hidden or not uniformly accessible to users.
  - **Hidden Valuable Examples:** Custom solutions may showcase wonderful, context-specific examples, but their lack of standardization makes them difficult to discover without prior knowledge of the project’s internal tooling.
  - **Extra Compilation and Maintenance Effort:** These ad-hoc methods usually require additional effort to compile and manage. The time spent on maintaining such tools often exceeds their practical benefits.
  - **Safety Concerns When Executing Code:** Custom scripts may have hard-coded paths, exceptional test cases, or other assumptions specific to a developer’s environment. As a result, running every test, binary, or example without careful vetting can introduce risks. The lack of uniform safety checks means that it's not always safe to execute all available code without considering these potential pitfalls.

  While a unified tool like **`cargo-e`** may not eliminate every security concern, it mitigates some risks by providing a more predictable and consistent interface for running other people's code (`OPC`). This helps developers avoid the common pitfalls associated with individually maintained scripts and ad-hoc solutions.

## Keep your Zoo out of our Namespace

   - [src](https://crates.io/crates/src) - manage your personal zoo of repositories.
   - [rg](https://crates.io/crates/rg) - you don't want this crate

   These two crates are ridiculous and deserve special mention - as of 2025-03-10.
  
   - src has 27,876 downloads.
   - rg has 13,369 downloads.

   This is the stuff that makes cargo rust a dangerous place to play and I do not recommend blindly running every crate's `fn main`.  Both `src` and `rg` have gotten me.  Another time when I mistakenly installed something, the crate told me to take a long walk in so many words.  report or complain; be thankful you still own your hardware and data, but that's 40K people who are being exposed to a `Zoo`.  It's reason enough to not leave cargo enabled by default.  I'll type `cargo` when I mean to type `git`.  27,876 downloads in five years.


   A name for a common `src` folder or rust based program and walk away.
   I don't know that it's squatting; its worse than that.  `src` has 7 versions. `rg` 3.
   
   `this`, coming from someone trying to define the short 'e' cargo subcommand.

   `nvm` the the short code zOoo.  not everything is an example.  watch those fingers.  

## Not a Digital Junk Drawer

  Crates can easily become digital junk drawers—random directories and executables thrown around like confetti. When developers adopt the "stick it anywhere" with no annotation/metadata, the result is a cluttered mess of custom scripts and ad-hoc tools that expose testers systems to risk by just running everything.

## Embracing Cargo’s Convention

  When you bypass Cargo’s metadata-driven approach by using custom, ad-hoc methods, you lose the inherent benefits that come from a well-defined project structure that follow some convention. Cargo.toml encapsulates critical metadata—like dependency management, versioning, and build instructions—that ensures consistency and predictability across Rust projects.

## Contributing

  Contributions are welcome! If you have suggestions or improvements, feel free to open an issue or submit a pull request.

## License

  This project is dual-licensed under the MIT License or the Apache License (Version 2.0), at your option.

## Acknowledgements

- Built with the power of the Rust ecosystem and libraries like [clap](https://crates.io/crates/clap), [crossterm](https://crates.io/crates/crossterm) (optional), and [ratatui](https://crates.io/crates/ratatui) (optional).
- Special thanks to the Rust community and all contributors for their continued support.
  
<a href="https://crates.io/crates/cargo-e" rel="nofollow noopener noreferrer">
  <img src="https://img.shields.io/crates/v/cargo-e.svg" alt="Crates.io">
</a>

<!-- Version notice -->
<p style="font-style: italic; color: #ccc; margin-top: 0.5em;">
  You are reading documentation version <span id="doc-version" style="color: white;">0.1.18</span>.
  If this does not match the version displayed above, then you're not reading the latest documentation!
</p>

## Addendum

- [documents/1_Cool_Examples_With_Cargo_e.md](https://github.com/davehorner/cargo-e/blob/develop/documents/1_Cool_Examples_With_Cargo_e.md)

- HORNER EXAMPLE 1: 
  [examples/funny_examples.rs](https://github.com/davehorner/cargo-e/blob/develop/examples/funny_example.rs)

- HORNER EXAMPLE 2: 
  [addendum/e_crate_version_checker/src/main.rs](https://github.com/davehorner/cargo-e/blob/develop/addendum/e_crate_version_checker/src/main.rs)

- HORNER EXAMPLE 3: 
  [addenda/e_update_readme/src/bin/e_update_readme.rs](https://github.com/davehorner/cargo-e/tree/develop/addenda/e_update_readme)
