# e_window_api

A high-level Rust API for controlling e_window instances programmatically, including comprehensive message box functionality.

## Overview

`e_window_api` provides a convenient Rust interface for launching and controlling e_window processes. Instead of manually spawning processes and writing to stdin, this crate offers a structured API for window management, content updates, real-time control, and modern message box functionality.

## Features

- **Message Box API**: Full-featured message box system with all standard button types
- **Notification System**: Auto-closing notifications with visual countdown timers
- **Easy Window Management**: Launch e_window instances with simple configuration
- **Real-time Control**: Send control commands to running windows
- **Animated Positioning**: Move and resize windows with easing animations
- **Structured Content API**: Build cards using structured fields instead of raw content strings
- **Document Protocol**: Full support for e_window's document format protocol
- **Async/Await Support**: Full async support with tokio
- **Type Safety**: Strongly typed API with comprehensive error handling

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
e_window_api = "0.1.0"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

### Message Box Usage

```rust
use e_window_api::msgbox::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Show an information message
    info("Welcome", "Welcome to e_window_api!")?;
    
    // Ask a yes/no question
    if ask_yes_no("Confirmation", "Do you want to continue?")? {
        println!("User chose to continue");
    }
    
    // Get text input from user
    if let MessageBoxResult::Text(name) = input_text("Name", "What's your name?")? {
        println!("Hello, {}!", name);
    }
    
    Ok(())
}
```

### Async Message Box Usage

```rust
use e_window_api::msgbox::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Async notification
    notify_async("Info", "Processing...", MessageBoxIcon::Information, Some(3)).await?;
    
    // Async question
    let result = message_box_async("Question", "Continue?", 
                                  MessageBoxType::YesNo, 
                                  MessageBoxIcon::Question, 
                                  None).await?;
    
    println!("User responded: {:?}", result);
    Ok(())
}
```

### Basic Usage

```rust
use e_window_api::{EWindow, WindowConfig, Card};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the window
    let config = WindowConfig::new()
        .title("My Window")
        .size(800, 600)
        .position(100, 100);
    
    // Launch the window
    let mut window = EWindow::launch(config).await?;
    
    // Create and show a card using the new structured API
    let card = Card::new()
        .window_title("Demo Window")
        .field("name", "Example")
        .typed_field("version", "1.0", "string")
        .card_title("Welcome!")
        .header("Getting Started")
        .caption("Status: Ready")
        .body_line("This demonstrates the structured card API.")
        .body_line("You can set title, header, caption, and body separately.");
    
    window.show_card(card).await?;
    
    // Close the window
    window.close().await?;
    
    Ok(())
}
```

### Simple Show Method

For quick demos, use the convenience `show()` method:

```rust
use e_window_api::Card;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let card = Card::new()
        .window_title("Quick Demo")
        .card_title("Hello World")
        .body("This is a simple demo using the show() method.");
    
    card.show_blocking()?;  // Blocks until window is closed
    Ok(())
}
```

## Message Box API

The `e_window_api` includes a comprehensive message box system that provides both blocking and async variants, supporting all standard message box types with emoji icons and automatic sizing.

### Quick Message Box Examples

```rust
use e_window_api::msgbox::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple info message (3 seconds auto-close)
    info("Information", "Operation completed successfully!")?;
    
    // Warning message (5 seconds auto-close)  
    warn("Warning", "This action cannot be undone.")?;
    
    // Error message (7 seconds auto-close)
    error("Error", "Failed to save file.")?;
    
    // Yes/No question
    let result = ask_yes_no("Confirmation", "Do you want to continue?")?;
    if result {
        println!("User chose Yes");
    }
    
    // OK/Cancel confirmation
    let confirmed = confirm("Save Changes", "Save before closing?")?;
    if confirmed {
        println!("User confirmed");
    }
    
    Ok(())
}
```

### Message Box Types

The API supports all standard Windows message box types:

```rust
use e_window_api::msgbox::*;

// Single button types
message_box("Title", "Message", MessageBoxType::Ok, None)?;

// Two button types  
let result = message_box("Title", "Continue?", MessageBoxType::OkCancel, None)?;
let result = message_box("Title", "Proceed?", MessageBoxType::YesNo, None)?;

// Three button types
let result = message_box("Title", "Save changes?", MessageBoxType::YesNoCancel, None)?;

// Default button variants
let result = message_box("Title", "Delete file?", MessageBoxType::YesNoDefNo, None)?;
let result = message_box("Title", "Retry?", MessageBoxType::RetryCancel, None)?;

// Special input types
let text = input_text("Input", "Enter your name:")?;
let file = select_file("File Selection", "Choose a configuration file:")?;
```

### Message Box Results

Message boxes return typed results that match the button configuration:

```rust
use e_window_api::msgbox::MessageBoxResult;

let result = message_box("Question", "Save changes?", MessageBoxType::YesNoCancel, None)?;
match result {
    MessageBoxResult::Yes => println!("Saving..."),
    MessageBoxResult::No => println!("Discarding changes..."),
    MessageBoxResult::Cancel => println!("Operation cancelled"),
    _ => {} // Other variants not possible for YesNoCancel
}

// Text input returns the entered text
if let MessageBoxResult::Text(user_input) = input_text("Name", "Enter name:")? {
    println!("User entered: {}", user_input);
}

// File selection returns the selected file path
if let MessageBoxResult::File(path) = select_file("Open", "Select file:")? {
    println!("Selected file: {}", path);
}
```

### Auto-closing Notifications

Notifications automatically close after a specified duration and show a visual countdown:

```rust
use e_window_api::msgbox::*;

// Built-in notification types with standard durations
info("Success", "File saved successfully!");     // 3 seconds
warn("Warning", "Disk space low");              // 5 seconds  
error("Error", "Network connection failed");     // 7 seconds

// Custom notification with specific duration
notify("Custom", "This will close in 10 seconds", 
       MessageBoxIcon::Information, Some(10))?;

// Persistent notification (no auto-close)
notify("Important", "Click OK to continue", 
       MessageBoxIcon::Warning, None)?;
```

### Async Message Box API

Full async support for non-blocking operations:

```rust
use e_window_api::msgbox::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Async notifications
    notify_async("Info", "Processing...", 
                MessageBoxIcon::Information, Some(3)).await?;
    
    // Async message boxes
    let result = message_box_async("Question", "Continue processing?", 
                                  MessageBoxType::YesNo, 
                                  MessageBoxIcon::Question, 
                                  Some(5)).await?;
    
    if result == MessageBoxResult::Yes {
        println!("Continuing...");
    }
    
    // Async input
    if let MessageBoxResult::Text(name) = 
        input_text_async("Name", "Enter your name:").await? {
        println!("Hello, {}!", name);
    }
    
    Ok(())
}
```

### Message Box Icons

Visual icons are automatically added based on the message type:

- **Information**: ℹ️ (info, notify with Information icon)
- **Warning**: ⚠️ (warn, warning messages)
- **Error**: ❌ (error, critical messages)
- **Question**: ❓ (yes/no, ok/cancel questions)

```rust
// Icons are automatically selected based on function used
info("Title", "Message");     // Uses ℹ️ Information icon
warn("Title", "Message");     // Uses ⚠️ Warning icon  
error("Title", "Message");    // Uses ❌ Error icon
ask_yes_no("Title", "Q?");    // Uses ❓ Question icon

// Or specify explicitly
message_box("Custom", "Message", MessageBoxType::Ok, 
           MessageBoxIcon::Warning, None)?;
```

### Advanced Features

#### Timeouts and Auto-close
```rust
// Message box with 10-second timeout
let result = message_box_ex("Question", "Decide quickly!", 
                           MessageBoxType::YesNo, 
                           MessageBoxIcon::Question, 
                           Some(10))?;

if result == MessageBoxResult::Timeout {
    println!("User didn't respond in time");
}
```

#### Input Validation
```rust
// Text input with validation
loop {
    if let MessageBoxResult::Text(input) = 
        input_text("Email", "Enter your email address:")? {
        if input.contains('@') {
            println!("Valid email: {}", input);
            break;
        } else {
            error("Invalid Email", "Please enter a valid email address")?;
        }
    } else {
        break; // User cancelled
    }
}
```

#### Convenience Functions
```rust
// Quick yes/no question
if ask_yes_no("Confirmation", "Delete all files?")? {
    println!("Deleting files...");
}

// Quick confirmation
if confirm("Save", "Save changes before closing?")? {
    println!("Saving...");
}

// Quick info with custom duration
notify_timed("Status", "Operation completed!", 5)?;
```

## Structured Card API

The e_window application uses a specific document format protocol for displaying content. This crate provides a structured API that automatically generates the correct format.

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

### Raw Document Example

```
--title "Project Status" --width 600 --height 400
project | e_window | string
version | 1.0.1 | string
status | active | string

Project Dashboard
Development Progress Report
Last Updated: 2025-10-11
We have made significant progress on the e_window project.

Key accomplishments:
- Implemented structured Card API
- Fixed positioning issues
- Added document protocol support
- Created comprehensive examples

Next steps:
- Add more animation options
- Improve error handling
- Write documentation
```

## Structured Card API

Instead of manually constructing document strings, use the structured Card API:

### Card Structure

```rust
use e_window_api::Card;

let card = Card::new()
    // Window configuration
    .window_title("My Window")          // Sets window title
    .size(800, 600)                   // Sets window size  
    .position(100, 100)               // Sets window position
    
    // Fields (key-value pairs)
    .field("name", "value")           // String field
    .typed_field("count", "42", "number")  // Typed field
    
    // Structured content
    .card_title("Main Title")         // Line 1 after blank separator
    .header("Header Text")            // Line 2  
    .caption("Caption Text")          // Line 3
    .body_line("Body line 1")         // Body content (line 4+)
    .body_line("Body line 2")
    .body("Multi-line\ncontent");     // Or set all body at once
```

### API Methods

#### Window Configuration
```rust
.window_title("text")              // Window title bar
.size(width, height)               // Window dimensions
.position(x, y)                    // Window position
```

#### Fields (Key-Value Pairs)
```rust
.field("key", "value")             // String field
.typed_field("key", "value", "type")  // Typed field
```

#### Structured Content
```rust
.card_title("title")               // Card title (line 1)
.header("header")                  // Header (line 2)
.caption("caption")                // Caption (line 3)
.body_line("line")                 // Add single body line
.body_lines(vec!["line1", "line2"]) // Set body from vector
.body("line1\nline2")              // Set body from string (splits on \n)
```

#### Convenience Methods
```rust
.show_blocking()                   // Show card and wait (blocking)
.show().await                      // Show card and wait (async)
```

### Migration from Legacy API

The old content-based API is deprecated but still supported:

```rust
// OLD (deprecated)
let card = Card::new()
    .title("Window Title")         // ❌ Deprecated - use window_title()
    .content("Main content")       // ❌ Deprecated - use card_title(), header(), etc.
    .add_content("More content");  // ❌ Deprecated - use body_line()

// NEW (recommended)  
let card = Card::new()
    .window_title("Window Title")  // ✅ Clear window title
    .card_title("Main content")    // ✅ Structured content title
    .body_line("More content");    // ✅ Structured body content
```

### Complete Example

```rust
use e_window_api::Card;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let card = Card::new()
        // Window configuration
        .window_title("Task Manager")
        .size(500, 400)
        .position(200, 150)
        
        // Fields section
        .field("project", "e_window")
        .field("language", "Rust")
        .typed_field("tasks_completed", "15", "number")
        .typed_field("progress", "75%", "percentage")
        
        // Content section
        .card_title("Daily Progress Report")
        .header("Development Status")
        .caption("Status: On Track")
        .body_line("Today's accomplishments:")
        .body_line("✅ Fixed positioning bugs")
        .body_line("✅ Implemented structured API")
        .body_line("✅ Updated documentation")
        .body_line("")
        .body_line("Tomorrow's goals:")
        .body_line("🎯 Add animation features")
        .body_line("🎯 Improve error messages")
        .body_line("🎯 Write more examples");
    
    // Display the card
    card.show_blocking()?;
    
    Ok(())
}
```
    
    // Close the window
    window.close().await?;
    
    Ok(())
}
```

## API Reference

### WindowConfig

Configure window properties before launching:

```rust
let config = WindowConfig::new()
    .title("My App")                   // Window title
    .size(800, 600)                   // Window size
    .position(100, 100)               // Window position
    .decode_debug(true)               // Enable debug output
    .add_arg("--custom-flag");        // Add custom CLI argument
```

### Card (Structured API)

Create structured content for display using the new API:

```rust
let card = Card::new()
    // Window configuration
    .window_title("Card Window")      // Window title bar
    .size(400, 300)                  // Window size
    .position(200, 200)              // Window position
    
    // Fields section (displayed as table)
    .field("author", "John Doe")     // String field
    .typed_field("count", "42", "number")  // Typed field
    
    // Structured content (after blank line separator)
    .card_title("Main Title")        // Content title (line 1)
    .header("Subtitle/Header")       // Header text (line 2)
    .caption("Status/Caption")       // Caption text (line 3)
    .body_line("Body content line 1")  // Body content (lines 4+)
    .body_line("Body content line 2")
    .body("Multi-line\nbody content"); // Or set body all at once
```

### Card (Legacy API - Deprecated)

The old content-based API is still supported but deprecated:

```rust
// ⚠️ DEPRECATED - Use structured API above instead
let card = Card::new()
    .title("Card Title")             // ❌ Use window_title() instead
    .size(400, 300)
    .position(200, 200)
    .field("author", "John Doe")
    .typed_field("count", "42", "number")
    .content("Main content goes here")      // ❌ Use card_title(), header(), etc.
    .add_content("Additional line 1")      // ❌ Use body_line() instead
    .add_content("Additional line 2");
```

### EWindow

Control a running window instance:

```rust
// Launch window
let mut window = EWindow::launch(config).await?;

// Show content
window.show_card(card).await?;

// Control window
window.set_title("New Title").await?;
window.set_rect(100, 100, 800, 600).await?;
window.set_rect_animated(200, 200, 400, 300, 1000, AnimationEasing::EaseInOut).await?;

// Wait and close
window.delay(2000).await?;
window.close().await?;
```

### Animation Easing

Supported easing types for animated operations:

- `AnimationEasing::Linear`
- `AnimationEasing::EaseIn`
- `AnimationEasing::EaseOut`
- `AnimationEasing::EaseInOut`
- `AnimationEasing::Bounce`
- `AnimationEasing::Elastic`
- `AnimationEasing::Back`
- `AnimationEasing::Custom("custom-easing")`

## Examples

The crate includes several comprehensive examples demonstrating different features:

### Message Box Examples

#### Message Box Demo
```bash
cargo run --example msgbox_demo
```
Comprehensive demonstration of the message box API:
- All message box types (Ok, OkCancel, YesNo, YesNoCancel, etc.)
- Auto-closing notifications with countdown timers
- Input dialogs and file selection
- Both blocking and async variants
- Error handling and timeout scenarios

#### Simple Message Box Demo
```bash
cargo run --example simple_msgbox_demo
```
Quick examples of common message box patterns:
- Info, warning, and error notifications
- Yes/No questions and confirmations
- Text input and file selection dialogs

#### Enhanced Message Box Demo
```bash
cargo run --example enhanced_msgbox_demo
```
Advanced message box features:
- Custom timeout values
- Complex button combinations
- Input validation patterns
- Async runtime integration

### Structured API Examples

#### Simple Structured Demo
```bash
cargo run --example simple_structured_demo
```
Demonstrates the new structured Card API with:
- Window configuration
- Fields section
- Structured content (title, header, caption, body)
- Multiple card creation patterns

#### Structured Card Demo  
```bash
cargo run --example structured_card_demo
```
Shows advanced structured content features:
- Complex field configurations
- Multi-line body content
- Window positioning with structured content

### Positioning Examples

#### Test Positioning
```bash
cargo run --example test_positioning
```
Tests explicit window positioning functionality:
- Fixed position windows
- Auto-centering behavior verification
- Content display with positioning

### Control API Examples

#### Simple Demo
```bash
cargo run --example simple_demo
```
A basic usage demonstration:
- Basic window creation with configuration
- Simple card display
- Automatic cleanup

#### Realtime Control Demo
```bash
cargo run --example realtime_control_api
```
Demonstrates dynamic window control with animated positioning:
- Animated window movement to different screen positions
- Dynamic card content updates
- Real-time title and size changes
- Smooth transitions with easing

#### Panic Card Simulation
```bash
cargo run --example panic_card_api
```
Shows how to display error/panic information:
- Structured error display
- Formatted panic information
- User-controlled window lifetime

#### Grid Demo
```bash
cargo run --example grid_demo_api
```
Demonstrates precise positioning in a grid layout:
- 3x3 grid positioning
- Animated movement between grid cells
- Visual grid representation
- Systematic position demonstration

## Error Handling

The crate provides comprehensive error handling through the `EWindowError` type:

```rust
use e_window_api::{EWindow, EWindowError};

match EWindow::launch_default().await {
    Ok(window) => { /* use window */ }
    Err(EWindowError::ExecutableNotFound) => {
        eprintln!("e_window executable not found in PATH");
    }
    Err(EWindowError::ProcessSpawnError(e)) => {
        eprintln!("Failed to spawn process: {}", e);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Prerequisites

- `e_window` executable must be available in PATH or built in the workspace
- Tokio runtime for async operations
- Windows OS for advanced window manipulation features

## API Evolution and Comparison

### Legacy vs Structured API

The crate has evolved from a content-based API to a structured API that better reflects the e_window document protocol:

#### Legacy API (Deprecated)
```rust
// Old approach - manual content construction
let card = Card::new()
    .title("My Window")              // ❌ Ambiguous - window or content title?
    .content("Title\nHeader\nCaption\nBody line 1\nBody line 2");  // ❌ Hard to manage
```

#### New Structured API (Recommended)
```rust
// New approach - structured fields
let card = Card::new()
    .window_title("My Window")       // ✅ Clear window title
    .card_title("Title")             // ✅ Clear content title
    .header("Header")                // ✅ Structured header
    .caption("Caption")              // ✅ Structured caption
    .body_line("Body line 1")        // ✅ Structured body
    .body_line("Body line 2");
```

### Direct e_window vs e_window_api

#### Direct Usage (Low-level)
```rust
// Manual process management and document construction
let mut child = Command::new("e_window")
    .args(&["--title", "Demo", "--width", "800"])
    .stdin(Stdio::piped())
    .spawn()?;

let stdin = child.stdin.as_mut().unwrap();
// Manual document format construction
stdin.write_all(b"!control: BeginDocument\n")?;
stdin.write_all(b"!control: Content --title \"Demo\" --width 800\n")?;
stdin.write_all(b"!control: Content field | value | string\n")?;
stdin.write_all(b"!control: Content \n")?;  // Blank line separator
stdin.write_all(b"!control: Content Title\n")?;
stdin.write_all(b"!control: Content Header\n")?;
stdin.write_all(b"!control: Content Caption\n")?;
stdin.write_all(b"!control: Content Body\n")?;
stdin.write_all(b"!control: EndDocument\n")?;
```

#### e_window_api Usage (High-level)
```rust
// Structured API with automatic document generation
let card = Card::new()
    .window_title("Demo")
    .size(800, 600)
    .field("field", "value")
    .card_title("Title")
    .header("Header")
    .caption("Caption")
    .body("Body");

card.show_blocking()?;  // Handles all the complexity automatically
```

### Generated Document Format

The structured API automatically generates the correct e_window document format:

```rust
let card = Card::new()
    .window_title("Demo")
    .field("version", "1.0")
    .card_title("Hello")
    .header("World")
    .body("Welcome!");

println!("{}", card.to_document());
```

Output:
```
--title "Demo"
version | 1.0 | string

Hello
World

Welcome!
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.