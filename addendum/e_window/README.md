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
- `--type <TYPE>`: Display as a message box with specified button type
- Any other positional arguments are treated as input text

You can also specify options as the **first line** of piped or file input.

### Message Box Types

The `--type` argument enables message box mode with different button configurations:

- `ok` - Single OK button
- `okcancel` - OK and Cancel buttons
- `yesno` - Yes and No buttons  
- `yesnodefno` - Yes and No buttons (No is default)
- `yesnocancel` - Yes, No, and Cancel buttons
- `yesnokansdefno` - Yes, No, and Cancel buttons (No is default)
- `retrycancel` - Retry and Cancel buttons
- `textinput` - Text input dialog with OK/Cancel
- `fileselection` - File selection dialog

#### Message Box Examples

```sh
# Simple information dialog
echo "Operation completed successfully!" | e_window --type ok

# Confirmation dialog
echo "Are you sure you want to delete this file?" | e_window --type yesno

# Text input
echo "Enter your name:" | e_window --type textinput

# File selection
echo "Select a configuration file:" | e_window --type fileselection
```

#### Example

```sh
e_window --title "Demo" --width 900 --height 600
```

Or with piped input:

```sh
echo "--title Demo Window --width 900" | e_window
```

---

## Document Format Protocol

`e_window` uses a specific document format protocol for displaying structured content. Understanding this format is essential for creating properly formatted input that displays correctly.

### Document Structure

A valid e_window document follows this structure:

```
--title "Window Title" --width 800 --height 600 --x 100 --y 100
field1 | value1 | type1
field2 | value2 | type2

Card Title
Header Text
Caption Text
Body line 1
Body line 2
...
```

### Format Breakdown

1. **CLI Arguments Line** (optional): Window configuration parameters
   - `--title "text"` - Window title
   - `--width N` - Window width in pixels
   - `--height N` - Window height in pixels  
   - `--x N` - Window X position
   - `--y N` - Window Y position
   - `--follow-hwnd 0xHEX` - Follow parent window

2. **Fields Section** (optional): Key-value pairs in `key | value | type` format
   - Displayed as a structured table in the UI
   - Common types: `string`, `number`, `boolean`, `date`

3. **Blank Line Separator** (crucial!): Required between configuration/fields and content

4. **Content Section**: Structured content after the blank line
   - **Line 1**: Card title (can be empty)
   - **Line 2**: Header text (can be empty)  
   - **Line 3**: Caption text (can be empty)
   - **Lines 4+**: Body content (multiple lines supported)

### Document Examples

#### Simple Document
```
--title "Hello World" --width 400 --height 300

Welcome
Getting Started
Ready to go!
This is a simple example of e_window content.
```

#### Complex Document with Fields
```
--title "Project Status" --width 600 --height 400
project | e_window | string
version | 1.0.1 | string
status | active | string
progress | 75% | percentage

Project Dashboard
Development Progress Report
Last Updated: 2025-10-11
We have made significant progress on the e_window project.

Key accomplishments:
- Implemented structured document protocol
- Added real-time control commands
- Created comprehensive examples
- Fixed positioning and auto-centering

Next steps:
- Add more animation options
- Improve error handling
- Write comprehensive documentation
```

#### Using with Command Line
```bash
# Simple text display
echo "Hello World" | e_window

# With window configuration
echo "--title 'Demo' --width 500
name | John Doe | string
status | active | string

User Profile
Account Information
Status: Online
Welcome to your dashboard!" | e_window

# From file
cat document.txt | e_window
```

### Protocol Notes

- The **blank line separator** is mandatory when you have both configuration/fields AND content
- Empty lines in content are preserved and displayed
- Field types affect how values are displayed (e.g., numbers may be right-aligned)
- CLI arguments in the first line override command-line parameters
- If no content section exists, only the fields table is displayed


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

### e_window_hydra
- Demonstrates advanced Win32 window management and automation for Chrome windows over a grid.
- Features robust grid mapping, dynamic topmost/z-order control, and automated modal closure via JS injection.
- Includes cross-platform mouse click simulation to grid cells, with all Win32 API usage encapsulated for maintainability.
- Useful for automated UI testing, demos, and interactive control of external applications.

### e_window_uxn 
- Integrates the [Uxn virtual machine](https://wiki.xxiivv.com/site/uxn.html)
- Allows you to run Uxn ROMs in a managed window, with dynamic grid-based positioning and real-time title updates.
- Supports auto-cycling through multiple ROMs, with the window title reflecting the current ROM filename.
- Shows ROMs from the [raven/roms collection](https://github.com/mkeeter/raven/tree/main/roms) and other select ROMs.
- All Uxn integration is feature-flagged (`uses_uxn`), so you can build with or without Uxn support.


### position_grid_demo
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


## Attribution

The files `uxn.rs` and `e_window_uxn.rs` are derived from code in the [raven](https://github.com/mkeeter/raven/) project by Matthew Keeter, released under the Mozilla Public License 2.0.

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

