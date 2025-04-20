# e_crate_version_checker

`e_crate_version_checker` is a Rust library and command-line application that checks for new versions of a specified crate from [crates.io](https://crates.io) and optionally updates it using `cargo install`. The tool supports simple numeric version comparisons. It also provides an interactive upgrade prompt for improved usability.

## Features

- **Version Checking:**  
  Query [crates.io](https://crates.io) to retrieve the latest version of a crate and compare it with the currently installed version.

- **Interactive Update:**  
  Provides an interactive prompt that asks the user if they want to update a crate when a new version is available.

- **Flexible Configuration:**  
  Supports enabling/disabling features such as `check-version`, `uses_reqwest`, and `uses_serde` to tailor the functionality to your needs.
  
- **Fortune:**  
  When the `fortune` feature is enabled, displays a random developer “fortune” before the update prompt.
- **Changelog:**  
  When the `changelog` feature is enabled, embeds and displays the latest release notes from the parent crate’s CHANGELOG after a successful update.

## Installation

To use this crate in your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
e_crate_version_checker = "0.1.0"
```

Make sure to enable the features you need. For example:

```toml
[dependencies.e_crate_version_checker]
version = "0.1.0"
features = ["check-version", "uses_reqwest", "uses_serde", "fortune", "changelog"]
```

## Usage

### As a Library

You can use the version checking functions directly in your Rust code. For example:

```rust
use e_crate_version_checker::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // required : register the current crate in the User-Agent string.
    register_user_crate!();

    let crate_name = "cargo-e";
    let latest = get_latest_version(crate_name)?;
    println!("Latest version: {}", latest);

    if is_newer_version_available(env!("CARGO_PKG_VERSION"), crate_name)? {
        println!("A new version is available!");
    } else {
        println!("You are running the latest version.");
    }

    // Alternatively, check for update with a single call.
    check_for_update()?;
    Ok(())
}
```

### As a Command-Line Application

Build the project with Cargo:

```sh
cargo build --release
```

Run the application by passing the name of the crate you want to check:

```sh
./target/release/e_crate_version_checker <crate_name>
```

For example:

```sh
./target/release/e_crate_version_checker cargo-e
```

The application will print the current version, query crates.io for the latest version, and prompt you to update if a newer version is available.

## Testing

The crate includes tests to verify update arguments, version‑checking logic, and changelog embedding. To run the full test suite:

```sh
cargo test --features changelog
```

### Testing Version Checker & Changelog Manually

You can drive the interactive prompt, fortunes, and changelog display locally without publishing or installing prior versions by using these environment variables:

- `E_CRATE_CURRENT_VERSION`      Override the detected current version (e.g., simulate an old install).
- `E_CRATE_DRY_RUN`              Skip the actual `cargo install` step (dry-run mode).
- `E_CRATE_FORCE_INTERACTIVE`    Bypass non-interactive detection so prompts always fire.

#### Example: Test `e_crate_version_checker`
```sh
printf "\ny\n" \
  | E_CRATE_CURRENT_VERSION=0.0.1 \
    E_CRATE_DRY_RUN=1 \
    E_CRATE_FORCE_INTERACTIVE=1 \
    cargo run --features changelog -- e_crate_version_checker
```
You should see output like:
```
minor update for e_crate_version_checker: 0.0.1 -> 0.1.15
 want to install? [Yes/no] (wait 5 seconds)
Update complete (dry-run).

📜 Changelog for version 0.1.15:
### Other

- extended samples are now showing.
```

#### Example: Test `cargo-e` Integration
```sh
printf "\ny\n" \
  | E_CRATE_CURRENT_VERSION=0.1.0 \
    E_CRATE_DRY_RUN=1 \
    E_CRATE_FORCE_INTERACTIVE=1 \
    cargo run --features fortune,changelog -- package-name
```
Replace `package-name` with the crate you want to test. This will trigger fortunes and changelog display even for the default `interactive_crate_upgrade` flow.

## Contributing

Contributions are welcome! Please fork the repository and open a pull request with your changes. When contributing, ensure that your code adheres to the existing style and that you update tests as necessary.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Addendum

Addendums come at the beginning of the sort order, yet they appear at the end?

Instead of “Addendums come at the beginning…”, consider “Addenda come at the beginning…”
Rationale: “Addenda” is the traditional plural of “addendum.”

This `e_crate_version_checker` is a standalone example of quality expected in simple projects.  It's a library and its a executable.  It can be run using `cargo run` with no additional arguments.  It has inline tests, I didn't feel like integration tests were needed as I was not feeling it tonight.  **cargo-e** now has a mechanism to offer automatic upgrades.  Is there dead code? Spelling mistakes?  Commented code?  What's it matter?

It departs from typical Cargo project conventions (`examples`) by adopting the name `addendum`, reflecting a HORNER convention — a set of personal practices you may choose to adopt or laugh at.  It's also a project HEREDOC.  If that's not clear now; follow along and it will be clear.  This is example 2.

Conventions require examples.  **cargo-e** uses examples to properly test its ideas.  I communicate ideas with code and LLMs.

[What even is "literate programming"?](https://pqnelson.github.io/2024/05/29/literate-programming.html) I didn't read it.  I don't know if its worth your time.  **Tl;dr**: if you want to preserve knowledge, then literate programming is a good fit.

Releasing software and qualifying software is not easy to do if you can't point to what "good enough" is.  That's the problem.  There isn't much to point at.  I'm willing to be laughed at and make bad choices in naming things.

> There are only two hard things in Computer Science: cache invalidation and naming things.
> 
> -- Phil Karlton
[what name should i pick now?](https://martinfowler.com/bliki/TwoHardThings.html)

Conventions are good. You should—um, add them?
[https://github.com/davehorner/mkcmt](https://github.com/davehorner/mkcmt)

-- Dave Horner 3/2025
HORNER EXAMPLE 2
sort order, field separators, and huh?
### Challenges in Naming Things and Establishing Conventions in Software

#### Ambiguity of Natural Language
- **Multiple Interpretations:**  
  Natural language words often have multiple meanings or connotations. A name that conveys a clear and unambiguous intent in one context might be interpreted differently in another.
- **Context Dependency:**  
  A name that fits well in one part of a system may be confusing or misleading when used elsewhere, leading to inconsistencies.

#### Cognitive and Communication Challenges
- **Conceptual Overload:**  
  Developers need to encapsulate complex behaviors or abstract concepts in succinct names. This often results in names that are either too vague or carry too much overloaded meaning.
- **Team Communication:**  
  Naming conventions require consensus among team members. Different perspectives and experiences can make it challenging to agree on names that everyone finds clear and appropriate.

#### Domain Complexity and Evolving Requirements
- **Evolving Understanding:**  
  As projects grow and evolve, the initial understanding of the domain deepens. A name that seemed adequate at the start may become misleading as requirements change or new features are added.
- **Technical Debt:**  
  Early naming decisions might cause long-term issues if the names don’t adapt well to future changes, thereby contributing to technical debt.

#### Establishing Conventions
- **Lack of Universality:**  
  Although conventions like camelCase or snake_case exist, there is no one-size-fits-all solution. Conventions often need to be tailored to the specific context of a project or organization.
- **Resistance to Change:**  
  Once established, changing naming conventions can be difficult due to the inertia of an existing codebase, making initial decisions particularly critical.
- **Balancing Flexibility and Consistency:**  
  Developers must strike a balance between having expressive, flexible names and maintaining a consistent naming style across the entire codebase.
