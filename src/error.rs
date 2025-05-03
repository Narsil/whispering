//! Error types for the Whispering application.
//!
//! This module defines the custom error types used throughout the application.
//! It uses the `thiserror` crate to derive error implementations and provides
//! convenient conversions from common error types.

use thiserror::Error;

/// Custom error type for the Whispering application.
///
/// This enum represents all possible error conditions that can occur during
/// the application's operation, including audio device errors, stream errors,
/// model errors, and more.
#[derive(Error, Debug)]
pub enum Error {
    /// Error related to audio device initialization or configuration
    #[error("Audio device error: {0}")]
    AudioDevice(String),

    /// Error related to audio stream operation
    #[error("Audio stream error: {0}")]
    AudioStream(String),

    /// Error related to Whisper model operations
    #[error("Whisper model error: {0}")]
    WhisperModel(String),

    /// Error related to keyboard input simulation
    #[error("Keyboard input error: {0}")]
    KeyboardInput(String),

    /// Error related to file system operations
    #[error("File system error: {0}")]
    FileSystem(String),

    /// Error related to application configuration
    #[error("Configuration error: {0}")]
    Config(String),

    /// Catch-all for unexpected errors
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Unknown(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::FileSystem(err.to_string())
    }
} 