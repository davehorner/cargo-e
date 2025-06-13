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

