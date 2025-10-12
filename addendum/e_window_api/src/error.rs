use thiserror::Error;

/// Error types for e_window_api operations
#[derive(Error, Debug)]
pub enum EWindowError {
    #[error("Failed to find e_window executable")]
    ExecutableNotFound,

    #[error("Failed to spawn e_window process: {0}")]
    ProcessSpawnError(#[from] std::io::Error),

    #[error("Failed to communicate with e_window process")]
    CommunicationError,

    #[error("Process has already exited")]
    ProcessExited,

    #[error("Invalid command format: {0}")]
    InvalidCommand(String),

    #[error("Timeout waiting for operation to complete")]
    Timeout,

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for e_window_api operations
pub type Result<T> = std::result::Result<T, EWindowError>;
