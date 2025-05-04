//! Whispering is a real-time speech-to-text application that uses the Whisper model
//! to transcribe audio input from your microphone.
//!
//! The application is structured into several modules:
//! - `audio`: Handles audio input/output and recording functionality
//! - `config`: Manages application configuration
//! - `keyboard`: Manages keyboard shortcuts and user input
//! - `whisper`: Provides speech recognition using the Whisper model
//! - `app`: Contains the main application logic and state management
//!
//! # Configuration
//! The application can be configured through a TOML file named `config.toml` in the current directory:
//!
//! ```toml
//! [audio]
//! channels = 1
//! sample_rate = 16000
//! bits_per_sample = 32
//! sample_format = "float"
//!
//! [model]
//! repo = "ggerganov/whisper.cpp"
//! filename = "ggml-base.en.bin"
//!
//! cache_dir = "/path/to/cache"
//! recording_path = "/path/to/recording.wav"
//! ```
//!
//! If no configuration file is found, default values will be used.
#![deny(missing_docs)]
#![deny(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use anyhow::Result;

mod app;
mod asr;
mod audio;
mod config;
mod keyboard;
mod logging;

/// Main entry point for the Whispering application.
///
/// Initializes logging with a default "info" level (can be overridden via RUST_LOG environment variable),
/// sets up the application, and runs the main event loop.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    logging::init_logging();

    // Create and run the application
    let mut app = app::App::new().await?;
    app.run().await?;

    Ok(())
}
