readme_content = """# e_ai_summarize

e_ai_summarize is a GenAI-powered Rust source code summarizer and crate recreator. It analyzes Rust source files to provide concise, insightful summaries—including key functionality, used crates, safety considerations, and file operations. Additionally, it now supports generating scripts (in Rust or Python) to recreate a crate from a given source folder.

---

## Features

- **Source Code Analysis:**
  - Summarizes the primary functionality of Rust source code.
  - Identifies which crates are used within the code.
  - Assesses code safety with a `SAFE_TO_RUN: YES/NO` verdict and explanation.
  - Detects file operations and provides a `FILE_OPERATIONS: YES/NO` verdict with details.

- **Interactive Follow-up Mode:**
  - Supports interactive questioning for deeper insights into the code analysis.
  - Offers a single follow-up question option for quick queries.

- **Output Flexibility:**
  - Choose between streaming and non-streaming output for summarization.

- **Crate Recreation:**
  - **Generate a Python script:** Creates a script that recreates the crate structure from the provided source folder.
  - **Generate a Rust script:** Creates a Rust script to recreate the crate.
  - Option to process only the `src` subfolder when recreating the crate.

---

## Installation

Add e_ai_summarize as a dependency in your `Cargo.toml`.

---

## Usage

### Command-Line Interface

The tool now offers two main modes: **Summarization** and **Crate Recreation**.

#### Summarization Mode

- **Summarize a Rust source file:**

  {{CODE}}bash
  cargo run -- path/to/source_file.rs
  {{CODE}}

- **Interactive Follow-up Mode:**

  Run in interactive mode to ask follow-up questions after the summary is generated:

  {{CODE}}bash
  cargo run -- path/to/source_file.rs --stdin
  {{CODE}}

- **Single Follow-up Question:**

  Provide a single follow-up question with the `-q` option:

  {{CODE}}bash
  cargo run -- path/to/source_file.rs -q "Does this code handle errors properly?"
  {{CODE}}

- **Enable Streaming Mode:**

  Use the `--streaming` flag for streaming output:

  {{CODE}}bash
  cargo run -- path/to/source_file.rs --streaming
  {{CODE}}

If no file is specified, e_ai_summarize will default to analyzing its own source code as a demonstration.

#### Crate Recreation Mode

- **Generate a Rust script to recreate the crate:**

  Use the `--recreate-crate-rs` flag. Optionally, provide a source folder (defaults to the current directory) and add `--src-only` to process only the `src` subfolder.

  {{CODE}}bash
  cargo run -- path/to/source_folder --recreate-crate-rs [--src-only]
  {{CODE}}

- **Generate a Python script to recreate the crate:**

  Use the `--recreate-crate-py` flag. Similar to the Rust mode, you can specify a source folder and use `--src-only` if needed.

  {{CODE}}bash
  cargo run -- path/to/source_folder --recreate-crate-py [--src-only]
  {{CODE}}

### Programmatic Usage

You can also integrate e_ai_summarize into your own Rust projects. For example:

{{CODE}}rust
use e_ai_summarize::summarizer::{self, ChatSession};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Summarize a source file
    let (summary, mut session) = summarizer::summarize_source_session(Some("path/to/source_file.rs"), true).await?;
    println!("Summary:\n{}", summary);

    // Optionally, ask a follow-up question
    let followup = session.ask("Can you elaborate on the error handling?").await?;
    println!("Follow-up Answer:\n{}", followup);

    Ok(())
}
{{CODE}}

---

## How It Works

1. **Parsing Input:**
   - Reads the provided Rust source file or, if no file is given, defaults to its own source code.
   - When in crate recreation mode, processes the provided source folder (or defaults to the current directory), optionally limiting to the `src` subfolder.

2. **Prompt Construction:**
   - Constructs a detailed prompt for analyzing the code's main functionality, used crates, safety, file operations, and any limitations.

3. **GenAI-Powered Chat Session:**
   - Leverages a GenAI model (e.g., "gpt-4o-mini") to generate the summary or script, with support for both streaming and non-streaming outputs.

4. **Interactive Querying:**
   - After the initial summary, users can enter interactive mode or ask single follow-up questions for further insights.

5. **Crate Recreation:**
   - Depending on the flags provided, the tool generates either a Python or Rust script that recreates the crate structure.

---

## Dependencies

- **Rust Async Runtime:** Uses [Tokio](https://tokio.rs/) for asynchronous operations.
- **Command-Line Parsing:** Uses [clap](https://github.com/clap-rs/clap) for handling command-line arguments.
- **Interactive Input:** Integrates [rustyline](https://github.com/kkawakam/rustyline) for interactive command-line input.
- **Logging:** Uses [env_logger](https://github.com/env-logger-rs/env_logger) and [log](https://docs.rs/log) for logging.
- **GenAI Client:** Powered by the GenAI library to perform the actual code analysis.

---

## Contributing

Contributions are welcome! Please feel free to open issues or submit pull requests for enhancements and bug fixes.

---

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.

---

## Acknowledgements

Thanks to the contributors of the GenAI library and the broader Rust community for their continuous support and innovation in open-source projects.

Enjoy using e_ai_summarize for all your Rust code analysis and crate recreation needs!

*Created by Your David Horner around 3/25*
"""

# Replace the placeholder with actual triple backticks
readme_content = readme_content.replace("{{CODE}}", "```")

# Write the content to README.md
with open("README.md", "w", encoding="utf-8") as file:
    file.write(readme_content)

print("README.md has been written successfully.")

