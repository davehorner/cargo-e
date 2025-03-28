# e_ai_summarize

e_ai_summarize is a GenAI-powered Rust source code summarizer designed to analyze Rust source files and provide concise, insightful summaries. It evaluates key aspects of the code, including its main functionality, utilized crates, safety considerations, file operations, and any notable limitations.

---

## Features

- **Source Code Analysis:**
  Summarizes the primary functionality of Rust source code.

- **Crate Identification:**
  Identifies which crates are being used within the code.

- **Safety Assessment:**
  Evaluates if the code appears safe to run, providing a `SAFE_TO_RUN: YES/NO` verdict with an explanation.

- **File Operations Check:**
  Detects if the code performs any file operations, issuing a `FILE_OPERATIONS: YES/NO` verdict along with details.

- **Interactive Follow-up Mode:**
  Supports interactive questioning to drill down further into the code analysis through follow-up queries.

- **Streaming & Non-Streaming Modes:**
  Offers both streaming and non-streaming output options for flexibility in how responses are displayed.

---

## Installation

Add e_ai_summarize as a dependency in your `Cargo.toml`:

---

## Usage

``` Command-Line Interface

You can run the summarizer from the command line. Below are some of the supported options:

- **Specify a file to summarize:**

```
bash
cargo run -- path/to/source_file.rs
```

- **Interactive Follow-up Mode:**

Run the tool in interactive mode to ask follow-up questions after the summary is generated:

```
bash
cargo run -- path/to/source_file.rs --stdin
```

- **Single Follow-up Question:**

Provide a single follow-up question with the `-q` option:

```
bash
cargo run -- path/to/source_file.rs -q "Does this code handle errors properly?"
```

- **Enable Streaming Mode:**

Use the `--streaming` flag for a streaming output of the response:

```
bash
cargo run -- path/to/source_file.rs --streaming
```

If no file is specified, e_ai_summarize will default to analyzing its own source code as a demonstration.

``` Programmatic Usage

You can also integrate e_ai_summarize into your own Rust projects. Import the `summarizer` module and utilize its functions:

```
rust
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
```

---

## How It Works

1. **Parsing Input:**
   The tool reads the provided Rust source file or defaults to its own source code if no file is specified.

2. **Prompt Construction:**
   It constructs a detailed prompt that asks for an analysis covering main functionality, used crates, safety, file operations, and any limitations.

3. **GenAI-Powered Chat Session:**
   The summarization is powered by a GenAI model (e.g., "gpt-4o-mini") through a chat session. The session can operate in streaming or non-streaming mode based on the user's preference.

4. **Interactive Querying:**
   After the initial summary, users can enter interactive follow-up mode or ask single follow-up questions to get further insights.

---

## Dependencies

- **Rust Async Runtime:**
  Utilizes [Tokio](https://tokio.rs/) for asynchronous operations.

- **Command-Line Parsing:**
  Uses [clap](https://github.com/clap-rs/clap) for handling command-line arguments.

- **Interactive Input:**
  Integrates [rustyline](https://github.com/kkawakam/rustyline) for interactive command-line input.

- **Logging:**
  Uses [env_logger](https://github.com/env-logger-rs/env_logger) and the [log](https://docs.rs/log) crate for logging.

- **GenAI Client:**
  Powered by the GenAI library to perform the actual code analysis.

---

## Contributing

Contributions are welcome! Please feel free to open issues or submit pull requests for enhancements and bug fixes.

---

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.

---

## Acknowledgements

Thanks to the contributors of the GenAI library and the broader Rust community for their continuous support and innovation in open-source projects.

Enjoy using e_ai_summarize for all your Rust code analysis needs!

Created by Your David Horner around 3/25
