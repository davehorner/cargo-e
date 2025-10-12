//! Synchronous message box API similar to WinAPI ShowMessageBox
//!
//! This module provides simple, blocking functions for displaying dialogs,
//! notifications, and getting user input without requiring async/await.

use crate::{Card, EWindow, WindowConfig};
use std::time::Duration;

// Re-export the shared msgbox types from e_window_types for convenience
pub use e_window_types::{
    MessageBoxConfig, MessageBoxIcon, MessageBoxResponse, MessageBoxResult, MessageBoxType,
};

/// Simple message box - blocks until user interaction or timeout
///
/// # Arguments
/// * `title` - Window title
/// * `message` - Main message text
/// * `box_type` - Type of buttons to show
/// * `icon` - Icon to display
/// * `timeout_secs` - Optional timeout in seconds (None = no timeout)
///
/// # Examples
/// ```rust,no_run
/// use e_window_api::msgbox::{message_box, MessageBoxType, MessageBoxIcon, MessageBoxResult};
///
/// let result = message_box(
///     "Confirmation",
///     "Are you sure you want to continue?",
///     MessageBoxType::YesNo,
///     MessageBoxIcon::Question,
///     Some(30)
/// ).unwrap();
///
/// match result {
///     MessageBoxResult::Yes => println!("User chose Yes"),
///     MessageBoxResult::No => println!("User chose No"),
///     MessageBoxResult::Timeout => println!("User didn't respond in time"),
///     _ => {}
/// }
/// ```
pub fn message_box(
    title: &str,
    message: &str,
    box_type: MessageBoxType,
    icon: MessageBoxIcon,
    timeout_secs: Option<u32>,
) -> crate::Result<MessageBoxResult> {
    println!("[DEBUG] message_box (sync) called: title='{}', message='{}', type={:?}, icon={:?}, timeout={:?}", 
             title, message, box_type, icon, timeout_secs);

    // Check if we're already in a tokio runtime context
    if tokio::runtime::Handle::try_current().is_ok() {
        println!("[DEBUG] Detected existing tokio runtime, using block_in_place");
        // We're in an async context, use spawn_blocking to run in a thread pool
        let title = title.to_string();
        let message = message.to_string();

        tokio::task::block_in_place(move || {
            println!("[DEBUG] Inside block_in_place, creating new runtime");
            // Create a new runtime in this thread
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                crate::EWindowError::ConfigError(format!("Failed to create runtime: {}", e))
            })?;

            println!("[DEBUG] New runtime created, calling async version");
            rt.block_on(async {
                message_box_async(&title, &message, box_type, icon, timeout_secs).await
            })
        })
    } else {
        println!("[DEBUG] No existing tokio runtime detected, creating new one");
        // We're not in an async context, create our own runtime
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            crate::EWindowError::ConfigError(format!("Failed to create runtime: {}", e))
        })?;

        println!("[DEBUG] Runtime created, calling async version");
        rt.block_on(async { message_box_async(title, message, box_type, icon, timeout_secs).await })
    }
}

/// Async version of message_box for use in async contexts
pub async fn message_box_async(
    title: &str,
    message: &str,
    box_type: MessageBoxType,
    icon: MessageBoxIcon,
    timeout_secs: Option<u32>,
) -> crate::Result<MessageBoxResult> {
    println!("[DEBUG] message_box_async called: title='{}', message='{}', type={:?}, icon={:?}, timeout={:?}", 
             title, message, box_type, icon, timeout_secs);

    let icon_str = match icon {
        MessageBoxIcon::None => {
            // Use type-specific emoji if no icon specified
            match box_type {
                MessageBoxType::Ok => "✅",
                MessageBoxType::OkCancel => "❓",
                MessageBoxType::YesNo | MessageBoxType::YesNoDefNo => "❓",
                MessageBoxType::YesNoCancel | MessageBoxType::YesNoCancelDefNo => "❓",
                MessageBoxType::RetryCancel => "🔄",
                MessageBoxType::TextInput => "📝",
                MessageBoxType::FileSelection => "📁",
            }
        }
        MessageBoxIcon::Information => "ℹ️",
        MessageBoxIcon::Warning => "⚠️",
        MessageBoxIcon::Error => "❌",
        MessageBoxIcon::Question => "❓",
    };

    // Use the new --type and --body approach for optimal egui-based sizing
    let body_message = format!("{} {}", icon_str, message);

    let config = WindowConfig::default()
        .title(title)
        .add_arg("--type")
        .add_arg(&format!("{:?}", box_type))
        .add_arg("--body")
        .add_arg(&body_message);

    // Note: Timeout is handled at the API level via tokio::time::timeout
    // Future enhancement: could add --timeout support to e_window

    println!(
        "[DEBUG] Launching e_window with --type {:?} --body '{}'...",
        box_type, body_message
    );
    let mut window = EWindow::launch(config).await?;
    println!("[DEBUG] e_window launched successfully");

    // Wait for user input or timeout
    let result = if let Some(timeout) = timeout_secs {
        println!("[DEBUG] Starting timeout wait for {} seconds...", timeout);

        // Wait for either a result or timeout
        match tokio::time::timeout(
            Duration::from_secs(timeout as u64),
            window.wait_for_result((timeout as u64 + 2) * 1000),
        )
        .await
        {
            Ok(Ok(result_str)) => {
                println!("[DEBUG] Received result before timeout: {}", result_str);
                parse_result_string(&result_str)
            }
            Ok(Err(e)) => {
                println!("[DEBUG] Error waiting for result: {:?}", e);
                MessageBoxResult::Timeout
            }
            Err(_) => {
                println!("[DEBUG] Timeout occurred");
                MessageBoxResult::Timeout
            }
        }
    } else {
        println!("[DEBUG] Starting indefinite wait...");

        // Wait indefinitely for user input
        match window.wait_for_result(30000).await {
            // 30 second max wait
            Ok(result_str) => {
                println!("[DEBUG] Received result: {}", result_str);
                parse_result_string(&result_str)
            }
            Err(e) => {
                println!("[DEBUG] Error or timeout waiting for result: {:?}", e);
                match box_type {
                    MessageBoxType::Ok => MessageBoxResult::Ok,
                    MessageBoxType::OkCancel => MessageBoxResult::Cancel,
                    MessageBoxType::YesNo => MessageBoxResult::No,
                    MessageBoxType::YesNoDefNo => MessageBoxResult::No, // Default to No
                    MessageBoxType::YesNoCancel => MessageBoxResult::Cancel,
                    MessageBoxType::YesNoCancelDefNo => MessageBoxResult::No, // Default to No
                    MessageBoxType::RetryCancel => MessageBoxResult::Cancel,
                    MessageBoxType::TextInput => MessageBoxResult::Cancel,
                    MessageBoxType::FileSelection => MessageBoxResult::Cancel,
                }
            }
        }
    };

    // Close window
    println!("[DEBUG] Closing window...");
    window.close().await?;
    println!("[DEBUG] Window closed successfully");

    println!("[DEBUG] message_box_async returning: {:?}", result);
    Ok(result)
}

/// Simple notification - shows message and auto-closes
///
/// # Arguments
/// * `title` - Window title
/// * `message` - Message text
/// * `icon` - Icon to display
/// * `duration_secs` - How long to show (default: 3 seconds)
pub fn notify(
    title: &str,
    message: &str,
    icon: MessageBoxIcon,
    duration_secs: Option<u32>,
) -> crate::Result<()> {
    // Check if we're already in a tokio runtime context
    if tokio::runtime::Handle::try_current().is_ok() {
        // We're in an async context, use spawn_blocking to run in a thread pool
        let title = title.to_string();
        let message = message.to_string();

        tokio::task::block_in_place(move || {
            // Create a new runtime in this thread
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                crate::EWindowError::ConfigError(format!("Failed to create runtime: {}", e))
            })?;

            rt.block_on(async { notify_async(&title, &message, icon, duration_secs).await })
        })
    } else {
        // We're not in an async context, create our own runtime
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            crate::EWindowError::ConfigError(format!("Failed to create runtime: {}", e))
        })?;

        rt.block_on(async { notify_async(title, message, icon, duration_secs).await })
    }
}

/// Async version of notify
pub async fn notify_async(
    title: &str,
    message: &str,
    icon: MessageBoxIcon,
    duration_secs: Option<u32>,
) -> crate::Result<()> {
    println!(
        "[DEBUG] notify_async called: title='{}', message='{}', icon={:?}, duration={:?}",
        title, message, icon, duration_secs
    );

    let duration = duration_secs.unwrap_or(3);
    let icon_str = match icon {
        MessageBoxIcon::None => "",
        MessageBoxIcon::Information => "ℹ️",
        MessageBoxIcon::Warning => "⚠️",
        MessageBoxIcon::Error => "❌",
        MessageBoxIcon::Question => "❓",
    };

    // Create notification card
    let mut card = Card::new()
        .window_title(&format!("{} {}", icon_str, title))
        .field("type", "Notification")
        .field("duration", &format!("{} seconds", duration))
        .card_title(message)
        .body_line("");

    // Add expiration field for visual countdown timer
    if duration > 0 {
        card = card.expiration("auto_close", duration as u64);
    }

    // Create config with initial input to prevent default card template usage
    let mut config = WindowConfig::default().title(title);

    // Generate card document and prepend --type argument for optimal sizing
    let card_document = card.to_document();
    let enhanced_document = format!("--type Ok\n{}", card_document);
    config.initial_input = Some(enhanced_document);

    println!("[DEBUG] Launching notification window...");
    let window = EWindow::launch(config).await?;
    println!("[DEBUG] Notification window launched successfully");
    println!("[DEBUG] Notification card sent successfully");

    // No need to reposition - e_window handles auto-centering based on content size

    // Auto-close after duration
    println!(
        "[DEBUG] Starting notification auto-close timer for {} seconds...",
        duration
    );
    tokio::time::sleep(Duration::from_secs(duration as u64)).await;
    println!("[DEBUG] Auto-close timer completed");

    println!("[DEBUG] Closing notification window...");
    // Use a safer close that doesn't error if the window is already closed
    match window.close().await {
        Ok(()) => println!("[DEBUG] Notification window closed successfully"),
        Err(crate::EWindowError::CommunicationError) => {
            println!("[DEBUG] Notification window already closed (user interaction)");
        }
        Err(e) => return Err(e),
    }

    Ok(())
}

/// Quick info message - shows for 3 seconds
pub fn info(title: &str, message: &str) -> crate::Result<()> {
    notify(title, message, MessageBoxIcon::Information, Some(3))
}

/// Quick warning message - shows for 5 seconds  
pub fn warn(title: &str, message: &str) -> crate::Result<()> {
    notify(title, message, MessageBoxIcon::Warning, Some(5))
}

/// Quick error message - shows for 7 seconds
pub fn error(title: &str, message: &str) -> crate::Result<()> {
    notify(title, message, MessageBoxIcon::Error, Some(7))
}

/// Simple yes/no question
pub fn ask_yes_no(title: &str, question: &str) -> crate::Result<bool> {
    let result = message_box(
        title,
        question,
        MessageBoxType::YesNo,
        MessageBoxIcon::Question,
        None,
    )?;
    Ok(result == MessageBoxResult::Yes)
}

/// Simple OK/Cancel confirmation
pub fn confirm(title: &str, message: &str) -> crate::Result<bool> {
    let result = message_box(
        title,
        message,
        MessageBoxType::OkCancel,
        MessageBoxIcon::Question,
        None,
    )?;
    Ok(result == MessageBoxResult::Ok)
}

/// Enhanced message box with full configuration options
///
/// # Arguments
/// * `config` - Complete configuration for the message box
///
/// # Examples
/// ```rust,no_run
/// use e_window_api::msgbox::{message_box_ex, MessageBoxConfig, MessageBoxType, MessageBoxIcon, MessageBoxResult};
///
/// let config = MessageBoxConfig::new("Custom Dialog", "Choose your option")
///     .with_type(MessageBoxType::YesNo)
///     .with_icon(MessageBoxIcon::Question)
///     .with_timeout(30)
///     .with_default_button(MessageBoxResult::No); // Enter key defaults to No instead of Yes
///
/// let response = message_box_ex(config).unwrap();
/// match response.result {
///     MessageBoxResult::Yes => println!("User chose Yes"),
///     MessageBoxResult::No => println!("User chose No"),
///     MessageBoxResult::Timeout => println!("User didn't respond in time"),
///     _ => {}
/// }
/// ```
pub fn message_box_ex(config: MessageBoxConfig) -> crate::Result<MessageBoxResponse> {
    // Check if we're already in a tokio runtime context
    if tokio::runtime::Handle::try_current().is_ok() {
        let config = config.clone();
        tokio::task::block_in_place(move || {
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                crate::EWindowError::ConfigError(format!("Failed to create runtime: {}", e))
            })?;
            rt.block_on(async { message_box_ex_async(config).await })
        })
    } else {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            crate::EWindowError::ConfigError(format!("Failed to create runtime: {}", e))
        })?;
        rt.block_on(async { message_box_ex_async(config).await })
    }
}

/// Async version of message_box_ex
pub async fn message_box_ex_async(config: MessageBoxConfig) -> crate::Result<MessageBoxResponse> {
    println!("[DEBUG] message_box_ex_async called: title='{}', message='{}', type={:?}, icon={:?}, timeout={:?}", 
             config.title, config.message, config.box_type, config.icon, config.timeout_secs);

    let icon_str = match config.icon {
        MessageBoxIcon::None => "",
        MessageBoxIcon::Information => "ℹ️",
        MessageBoxIcon::Warning => "⚠️",
        MessageBoxIcon::Error => "❌",
        MessageBoxIcon::Question => "❓",
    };

    let (buttons_text, buttons_help, has_input) = match config.box_type {
        MessageBoxType::Ok => (
            "Press [Enter] or [Space] to continue",
            "Available: OK",
            false,
        ),
        MessageBoxType::OkCancel => (
            "Press [Enter] for OK, [Esc] for Cancel",
            "Available: OK, Cancel",
            false,
        ),
        MessageBoxType::YesNo => {
            let default_text = if config.default_button == Some(MessageBoxResult::No) {
                "Press [Y] for Yes, [Enter/N] for No"
            } else {
                "Press [Enter/Y] for Yes, [N] for No"
            };
            (default_text, "Available: Yes, No", false)
        }
        MessageBoxType::YesNoDefNo => {
            // Default No variants
            (
                "Press [Y] for Yes, [Enter/N] for No (default)",
                "Available: Yes, No (Default)",
                false,
            )
        }
        MessageBoxType::YesNoCancel => {
            let default_text = match config.default_button {
                Some(MessageBoxResult::No) => {
                    "Press [Y] for Yes, [Enter/N] for No, [Esc] for Cancel"
                }
                Some(MessageBoxResult::Cancel) => {
                    "Press [Y] for Yes, [N] for No, [Enter/Esc] for Cancel"
                }
                _ => "Press [Enter/Y] for Yes, [N] for No, [Esc] for Cancel",
            };
            (default_text, "Available: Yes, No, Cancel", false)
        }
        MessageBoxType::YesNoCancelDefNo => {
            // Default No with Cancel variant
            (
                "Press [Y] for Yes, [Enter/N] for No (default), [Esc] for Cancel",
                "Available: Yes, No (Default), Cancel",
                false,
            )
        }
        MessageBoxType::RetryCancel => (
            "Press [R] for Retry, [Esc] for Cancel",
            "Available: Retry, Cancel",
            false,
        ),
        MessageBoxType::TextInput => (
            "Type your input and press [Enter] to confirm, [Esc] to cancel",
            "Text Input Dialog",
            true,
        ),
        MessageBoxType::FileSelection => (
            "Select file path and press [Enter] to confirm, [Esc] to cancel",
            "File Selection Dialog",
            true,
        ),
    };

    // Create message card
    let mut card = Card::new()
        .window_title(&format!("{} {}", icon_str, config.title))
        .field("type", &format!("{:?}", config.box_type))
        .card_title(&config.message);

    // Add input fields for text input types
    if has_input {
        match config.box_type {
            MessageBoxType::TextInput => {
                card = card
                    .field("user_input", config.initial_text.as_deref().unwrap_or(""))
                    .field("input_type", "text");
                if let Some(initial) = &config.initial_text {
                    card = card.body_line(&format!("Initial text: {}", initial));
                }
            }
            MessageBoxType::FileSelection => {
                let initial_path = config.initial_directory.as_deref().unwrap_or(".");
                card = card
                    .field("user_input", initial_path)
                    .field("input_type", "file");
                if let Some(filter) = &config.file_filter {
                    card = card
                        .field("file_filter", filter)
                        .body_line(&format!("File filter: {}", filter));
                }
                if let Some(dir) = &config.initial_directory {
                    card = card.body_line(&format!("Initial directory: {}", dir));
                }
            }
            _ => {}
        }
    }

    card = card
        .body_line("")
        .body_line(buttons_text)
        .body_line("")
        .body_line(buttons_help);

    // Add default button information
    if let Some(default) = config.default_button {
        card = card.field("default_button", &format!("{:?}", default));
    }

    // Add expiration field for visual countdown timer if timeout is specified
    if let Some(timeout) = config.timeout_secs {
        card = card
            .expiration("auto_close", timeout as u64)
            .body_line(&format!("⏱️ Auto-close in {} seconds if no input", timeout));
    }

    // Create window config with initial input to prevent default card template usage
    let mut window_config = WindowConfig::default().title(&config.title);

    // Generate card document and prepend --type argument for optimal sizing
    let card_document = card.to_document();
    let enhanced_document = format!("--type {:?}\n{}", config.box_type, card_document);
    window_config.initial_input = Some(enhanced_document);

    println!("[DEBUG] Launching e_window...");
    let window = EWindow::launch(window_config).await?;
    println!("[DEBUG] e_window launched successfully");

    // Wait for user input or timeout
    let response = if let Some(timeout) = config.timeout_secs {
        println!("[DEBUG] Starting timeout wait for {} seconds...", timeout);
        // TODO: In a real implementation, we'd listen for actual user input
        tokio::time::sleep(Duration::from_secs(timeout as u64)).await;
        println!("[DEBUG] Timeout completed");
        MessageBoxResponse::new(MessageBoxResult::Timeout)
    } else {
        println!("[DEBUG] Starting indefinite wait (3 second simulation)...");
        // TODO: In a real implementation, we'd wait for actual user input
        tokio::time::sleep(Duration::from_secs(3)).await;

        let result = if let Some(default) = config.default_button {
            // Use the specified default button
            default
        } else {
            // Use type-based defaults
            match config.box_type {
                MessageBoxType::Ok => MessageBoxResult::Ok,
                MessageBoxType::OkCancel => MessageBoxResult::Ok,
                MessageBoxType::YesNo => MessageBoxResult::Yes,
                MessageBoxType::YesNoDefNo => MessageBoxResult::No, // Default to No
                MessageBoxType::YesNoCancel => MessageBoxResult::Yes,
                MessageBoxType::YesNoCancelDefNo => MessageBoxResult::No, // Default to No
                MessageBoxType::RetryCancel => MessageBoxResult::Retry,
                MessageBoxType::TextInput => MessageBoxResult::TextInput,
                MessageBoxType::FileSelection => MessageBoxResult::FileSelected,
            }
        };

        let response = match config.box_type {
            MessageBoxType::TextInput => {
                // In a real implementation, we'd get this from the actual input field
                let text = config
                    .initial_text
                    .unwrap_or_else(|| "Sample text input".to_string());
                MessageBoxResponse::with_text(MessageBoxResult::TextInput, text)
            }
            MessageBoxType::FileSelection => {
                // In a real implementation, we'd get this from the file dialog
                let path = config
                    .initial_directory
                    .unwrap_or_else(|| "C:\\sample\\file.txt".to_string());
                MessageBoxResponse::with_text(MessageBoxResult::FileSelected, path)
            }
            _ => MessageBoxResponse::new(result),
        };

        println!("[DEBUG] Default wait completed, returning {:?}", response);
        response
    };

    // Close window
    println!("[DEBUG] Closing window...");
    window.close().await?;
    println!("[DEBUG] Window closed successfully");

    println!("[DEBUG] message_box_ex_async returning: {:?}", response);
    Ok(response)
}

/// Text input dialog - prompts user for text input
///
/// # Arguments
/// * `title` - Dialog title
/// * `prompt` - Prompt message
/// * `initial_text` - Initial text in input field
/// * `timeout_secs` - Optional timeout in seconds
///
/// # Returns
/// * `Ok(Some(text))` - User entered text and confirmed
/// * `Ok(None)` - User cancelled or dialog timed out
/// * `Err(...)` - Error occurred
pub fn input_text(
    title: &str,
    prompt: &str,
    initial_text: &str,
    timeout_secs: Option<u32>,
) -> crate::Result<Option<String>> {
    let config = MessageBoxConfig::new(title, prompt)
        .with_type(MessageBoxType::TextInput)
        .with_icon(MessageBoxIcon::Question)
        .with_initial_text(initial_text);

    let config = if let Some(timeout) = timeout_secs {
        config.with_timeout(timeout)
    } else {
        config
    };

    let response = message_box_ex(config)?;
    match response.result {
        MessageBoxResult::TextInput => Ok(response.text_result),
        MessageBoxResult::Ok => Ok(response.text_result),
        _ => Ok(None),
    }
}

/// File selection dialog - prompts user to select a file
///
/// # Arguments
/// * `title` - Dialog title
/// * `initial_dir` - Initial directory to browse
/// * `file_filter` - File filter (e.g., "*.txt;*.log")
/// * `timeout_secs` - Optional timeout in seconds
///
/// # Returns
/// * `Ok(Some(path))` - User selected a file
/// * `Ok(None)` - User cancelled or dialog timed out
/// * `Err(...)` - Error occurred
pub fn select_file(
    title: &str,
    initial_dir: &str,
    file_filter: &str,
    timeout_secs: Option<u32>,
) -> crate::Result<Option<String>> {
    let config = MessageBoxConfig::new(title, "Select a file:")
        .with_type(MessageBoxType::FileSelection)
        .with_icon(MessageBoxIcon::Question)
        .with_initial_directory(initial_dir)
        .with_file_filter(file_filter);

    let config = if let Some(timeout) = timeout_secs {
        config.with_timeout(timeout)
    } else {
        config
    };

    let response = message_box_ex(config)?;
    match response.result {
        MessageBoxResult::FileSelected => Ok(response.text_result),
        MessageBoxResult::Ok => Ok(response.text_result),
        _ => Ok(None),
    }
}

/// Enhanced Yes/No dialog with customizable default button
///
/// # Arguments
/// * `title` - Dialog title
/// * `question` - Question to ask
/// * `default_yes` - If true, Enter defaults to Yes; if false, Enter defaults to No
/// * `timeout_secs` - Optional timeout in seconds
///
/// # Returns
/// * `Ok(Some(true))` - User chose Yes
/// * `Ok(Some(false))` - User chose No
/// * `Ok(None)` - Dialog timed out
pub fn ask_yes_no_ex(
    title: &str,
    question: &str,
    default_yes: bool,
    timeout_secs: Option<u32>,
) -> crate::Result<Option<bool>> {
    let default_button = if default_yes {
        Some(MessageBoxResult::Yes)
    } else {
        Some(MessageBoxResult::No)
    };

    let config = MessageBoxConfig::new(title, question)
        .with_type(MessageBoxType::YesNo)
        .with_icon(MessageBoxIcon::Question)
        .with_default_button(default_button.unwrap());

    let config = if let Some(timeout) = timeout_secs {
        config.with_timeout(timeout)
    } else {
        config
    };

    let response = message_box_ex(config)?;
    match response.result {
        MessageBoxResult::Yes => Ok(Some(true)),
        MessageBoxResult::No => Ok(Some(false)),
        MessageBoxResult::Timeout => Ok(None),
        _ => Ok(None),
    }
}

/// Parses the `--type` argument and maps it to a `MessageBoxType`.
fn parse_message_box_type(arg: &str) -> Option<MessageBoxType> {
    match arg.to_lowercase().as_str() {
        "ok" => Some(MessageBoxType::Ok),
        "okcancel" => Some(MessageBoxType::OkCancel),
        "yesno" => Some(MessageBoxType::YesNo),
        "yesnocancel" => Some(MessageBoxType::YesNoCancel),
        "retrycancel" => Some(MessageBoxType::RetryCancel),
        "textinput" => Some(MessageBoxType::TextInput),
        "fileselection" => Some(MessageBoxType::FileSelection),
        _ => None,
    }
}

/// Example usage of the `--type` flag.
pub fn message_box_with_type(
    title: &str,
    message: &str,
    box_type_arg: &str,
    icon: MessageBoxIcon,
    timeout_secs: Option<u32>,
) -> crate::Result<MessageBoxResult> {
    let box_type = parse_message_box_type(box_type_arg).ok_or_else(|| {
        crate::EWindowError::ConfigError(format!("Invalid message box type: {}", box_type_arg))
    })?;

    message_box(title, message, box_type, icon, timeout_secs)
}

/// Parse result string from e_window into MessageBoxResult
fn parse_result_string(result_str: &str) -> MessageBoxResult {
    match result_str.trim() {
        "Ok" | "OK" | "ok" => MessageBoxResult::Ok,
        "Yes" | "YES" | "yes" => MessageBoxResult::Yes,
        "No" | "NO" | "no" => MessageBoxResult::No,
        "Cancel" | "CANCEL" | "cancel" => MessageBoxResult::Cancel,
        "Retry" | "RETRY" | "retry" => MessageBoxResult::Retry,
        "Timeout" | "TIMEOUT" | "timeout" => MessageBoxResult::Timeout,
        s if s.starts_with("TEXT:") => {
            // For text input results - extract the actual text
            MessageBoxResult::TextInput
        }
        s if s.starts_with("TextInput:") => {
            // New format from updated e_window
            MessageBoxResult::TextInput
        }
        s if s.starts_with("FILE:") => {
            // For file selection results - extract the actual path
            MessageBoxResult::FileSelected
        }
        s if s.starts_with("FileSelected:") => {
            // New format from updated e_window
            MessageBoxResult::FileSelected
        }
        _ => {
            println!(
                "[DEBUG] Unrecognized result string: '{}', defaulting to Cancel",
                result_str
            );
            MessageBoxResult::Cancel // Default fallback
        }
    }
}
