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

## Pool Manager Mode

`e_window` includes a **pool manager mode** that allows you to automatically keep a specified number of windows open at all times. This is useful for testing, demonstrations, or any scenario where you want to ensure a persistent set of windows.

### How It Works

- When you launch `e_window` with the `--w-pool-cnt <N>` option, a special **pool manager GUI** window is started.
- The pool manager is always on top and displays the current status of the window pool.
- The manager automatically spawns and tracks child windows, each with a unique index.
- If any child window is closed, the manager will re-spawn it to maintain the desired count.
- Closing the pool manager window will automatically close all managed child windows.

### Pool Manager Options

- `--w-pool-cnt <N>`: Keep at least N windows open at all times.
- `--w-pool-rate <MS>`: Minimum milliseconds between opening new windows (default: 1000).
- `--w-pool-ndx <N>`: (internal) Index of this window instance (used by the manager for child windows).
- `--manager-hwnd <HWND>`: (internal) HWND of the pool manager window.

### Example Usage

```sh
e_window --w-pool-cnt 4
```

---

## July 2025

### Keyboard Shortcuts
- Pressing Enter or Escape in the main card view closes the window (same as clicking OK).

### Improved Scroll Behavior
- Scroll areas now use unique IDs to avoid egui warnings and ensure reliable scrolling.
- Scroll bars are always visible and support drag-to-scroll and arrow key navigation.

### e_window_hydra Example
- Demonstrates advanced Win32 window management and automation for Chrome windows over a grid.
- Features robust grid mapping, dynamic topmost/z-order control, and automated modal closure via JS injection.
- Includes cross-platform mouse click simulation to grid cells, with all Win32 API usage encapsulated for maintainability.
- Useful for automated UI testing, demos, and interactive control of external applications.

### position_grid_demo Example
- Shows how to use the PositionGrid utility for interactive grid-based input and diagnostics.
- Visualizes grid cell mapping, DPI scaling, and mouse click simulation within an eframe window.
- Includes detailed logging and diagnostics for grid geometry, cell selection, and input mapping.
- Ideal for learning, debugging, and validating grid-based UI logic in Rust/egui applications.

### Real-Time Control Example and Supported Commands

`e_window` can process special control commands from stdin to update its window in real time. These commands must start with `!control:` and are parsed as structured instructions.

#### Supported Commands

- `!control:exit`  
    Closes the window.

- `!control:set_rect x y w h`  
    Sets the window rectangle to position (`x`, `y`) and size (`w`, `h`).

- `!control:set_rect_eased x y w h duration_ms easing`  
    Animates the window rectangle to the given position and size over `duration_ms` milliseconds using the specified `easing` function.

- `!control:set_title <title>`  
    Changes the window title.

- `!control:begin_document`  
    Marks the start of a document.

- `!control:end_document`  
    Marks the end of a document.

- `!control:delay <milliseconds>`  
    Pauses processing for the specified number of milliseconds.

Any other line is treated as content and displayed in the window.

#### Example Usage

```text
!control:set_rect 10 20 300 400
!control:set_rect_eased 10 20 300 400 500 ease-in-out
!control:set_title My Window
!control:begin_document
Some regular content line
!control:end_document
!control:delay 1000
!control:exit
```

These commands can be sent interactively or piped into `e_window` to control its behavior while running.

Take a look at the `animated_control` example for more detail.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for suggestions, bug fixes, or improvements.

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.