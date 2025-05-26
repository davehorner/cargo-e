# e_window

`e_window` A window tool. Think WinAPI ShowMessageBox; but more than that.

---
feat(cargo-e): Integrate e_window for panic handling and structured output

Integrates the new e_window application into cargo-e to provide graphical panic windows and structured output display.

This enhancement introduces a robust mechanism for cargo-e to present error messages and other important information in a user-friendly GUI, rather than just console output. When a critical error or panic occurs, e_window can now be invoked by cargo-e to display the panic details in a dedicated, interactive window, improving user experience by providing clear, persistent feedback.

Key aspects of this integration and the new e_window application include:

Graphical Panic Reporting: cargo-e can now launch e_window to present detailed panic information, including stack traces and contextual data, in a dedicated GUI. This makes debugging and error reporting significantly more accessible to users.
Structured Output Display: e_window is capable of parsing and presenting structured data (like key-value pairs, headers, and body text) from input, making it a versatile tool for displaying various forms of output beyond panics.
Customizable Window Behavior: The window's title, size, and position can be configured programmatically or via arguments embedded in the input data itself, allowing cargo-e to tailor the error display.
External Window Monitoring (Windows only): The e_window can optionally monitor another process's window (by HWND) and signal its closure, which can be useful for coordinated application behavior or debugging within cargo-e's ecosystem.
This change significantly improves the diagnostic and user feedback capabilities of cargo-e.


## Features

- Create and manage an `eframe` window with custom title, size, and position.
- Input or pipe text and display the parsed output in real time.
- Supports command-line options via CLI or as the first line of input.
- Optionally follow another window by HWND and beep when it closes (Windows only).

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (with `cargo`)
- A compatible version of the `eframe` and `egui` libraries (handled by Cargo)

---

### Building the Project

From the workspace root or the `addendum/e_window` directory:

```sh
cargo build
```

---

### Running the Application

From the `addendum/e_window` directory:

```sh
cargo run
```

Or, to run with custom options:

```sh
cargo run -- --title "My Window" --width 1024 --height 768
```

You can also pipe input:

```sh
echo "some text" | cargo run
```

---

### Installing the Binary

If you want the `e_window` command available globally, install it from the workspace root:

```sh
cargo install --path addendum/e_window
```

Or, if your Cargo version supports it and you want all workspace binaries:

```sh
cargo install --workspace --bins
```

---

## Usage

You can launch `e_window` directly or use it as a library in your own project.

### Command-Line Options

- `--title <TITLE>`: Set window title
- `--width <WIDTH>`: Set window width
- `--height <HEIGHT>`: Set window height
- `--x <X>`: Set window X position
- `--y <Y>`: Set window Y position
- `--follow-hwnd <HWND>`: (Windows only) Follow another window and beep when it closes
- `-i, --input-file <FILE>`: Read input data from file
- Any other positional arguments are treated as input text

You can also specify options as the **first line** of piped or file input.

#### Example

```sh
e_window --title "Demo" --width 900 --height 600
```

Or with piped input:

```sh
echo "--title Demo Window --width 900" | e_window
```


---

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for suggestions, bug fixes, or improvements.

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.