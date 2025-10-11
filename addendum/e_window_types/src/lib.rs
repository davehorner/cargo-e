//! Shared types for e_window ecosystem
//! 
//! This crate provides common types used across e_window, e_window_api, and related crates.
//! It has minimal dependencies to ensure fast compilation times.

/// Message box functionality types
pub mod msgbox {
    /// Message box types similar to WinAPI MessageBox
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum MessageBoxType {
        /// Simple OK dialog
        Ok,
        /// OK/Cancel dialog  
        OkCancel,
        /// Yes/No dialog
        YesNo,
        /// Yes/No dialog with No as default
        YesNoDefNo,
        /// Yes/No/Cancel dialog
        YesNoCancel,
        /// Yes/No/Cancel dialog with No as default
        YesNoCancelDefNo,
        /// Retry/Cancel dialog
        RetryCancel,
        /// Text input dialog with OK/Cancel
        TextInput,
        /// File selection dialog with OK/Cancel
        FileSelection,
    }

    /// Message box icons
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum MessageBoxIcon {
        /// No icon
        None,
        /// Information icon (ℹ️)
        Information,
        /// Warning icon (⚠️) 
        Warning,
        /// Error icon (❌)
        Error,
        /// Question icon (❓)
        Question,
    }

    /// Message box result
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum MessageBoxResult {
        Ok,
        Cancel,
        Yes,
        No,
        Retry,
        /// Window was closed or timed out
        Timeout,
        /// User provided text input (check text_result for content)
        TextInput,
        /// User selected a file (check text_result for path)
        FileSelected,
    }

    /// Configuration for message box behavior
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct MessageBoxConfig {
        pub title: String,
        pub message: String,
        pub box_type: MessageBoxType,
        pub icon: MessageBoxIcon,
        pub timeout_secs: Option<u32>,
        /// Default button for Enter key (if not specified, uses first button)
        pub default_button: Option<MessageBoxResult>,
        /// Initial text for TextInput type
        pub initial_text: Option<String>,
        /// File filter for FileSelection type (e.g., "*.txt;*.log")
        pub file_filter: Option<String>,
        /// Default directory for FileSelection type
        pub initial_directory: Option<String>,
    }

    impl Default for MessageBoxConfig {
        fn default() -> Self {
            Self {
                title: "Message".to_string(),
                message: "".to_string(),
                box_type: MessageBoxType::Ok,
                icon: MessageBoxIcon::None,
                timeout_secs: None,
                default_button: None,
                initial_text: None,
                file_filter: None,
                initial_directory: None,
            }
        }
    }

    impl MessageBoxConfig {
        pub fn new(title: &str, message: &str) -> Self {
            Self {
                title: title.to_string(),
                message: message.to_string(),
                ..Default::default()
            }
        }

        pub fn with_type(mut self, box_type: MessageBoxType) -> Self {
            self.box_type = box_type;
            self
        }

        pub fn with_icon(mut self, icon: MessageBoxIcon) -> Self {
            self.icon = icon;
            self
        }

        pub fn with_timeout(mut self, timeout_secs: u32) -> Self {
            self.timeout_secs = Some(timeout_secs);
            self
        }

        pub fn with_default_button(mut self, default: MessageBoxResult) -> Self {
            self.default_button = Some(default);
            self
        }

        pub fn with_initial_text(mut self, text: &str) -> Self {
            self.initial_text = Some(text.to_string());
            self
        }

        pub fn with_file_filter(mut self, filter: &str) -> Self {
            self.file_filter = Some(filter.to_string());
            self
        }

        pub fn with_initial_directory(mut self, dir: &str) -> Self {
            self.initial_directory = Some(dir.to_string());
            self
        }
    }

    /// Extended result containing both button result and optional text
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct MessageBoxResponse {
        pub result: MessageBoxResult,
        /// Text result for TextInput or FileSelection types
        pub text_result: Option<String>,
    }

    impl MessageBoxResponse {
        pub fn new(result: MessageBoxResult) -> Self {
            Self {
                result,
                text_result: None,
            }
        }

        pub fn with_text(result: MessageBoxResult, text: String) -> Self {
            Self {
                result,
                text_result: Some(text),
            }
        }
    }

    impl MessageBoxType {
        /// Parses the `--type` argument and maps it to a `MessageBoxType`.
        pub fn from_str(arg: &str) -> Option<Self> {
            match arg.to_lowercase().as_str() {
                "ok" => Some(Self::Ok),
                "okcancel" => Some(Self::OkCancel),
                "yesno" => Some(Self::YesNo),
                "yesnodefno" => Some(Self::YesNoDefNo),
                "yesnocancel" => Some(Self::YesNoCancel),
                "yesnocanceldefno" => Some(Self::YesNoCancelDefNo),
                "retrycancel" => Some(Self::RetryCancel),
                "textinput" => Some(Self::TextInput),
                "fileselection" => Some(Self::FileSelection),
                _ => None,
            }
        }
    }
}

// Re-export common types at crate root for convenience
pub use msgbox::{MessageBoxType, MessageBoxIcon, MessageBoxResult, MessageBoxConfig, MessageBoxResponse};