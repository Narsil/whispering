//! Whispering is a real-time speech-to-text application that uses the Whisper model
//! to transcribe audio input from your microphone.
//!
//! The application is structured into several modules:
//! - `audio`: Handles audio input/output and recording functionality
//! - `keyboard`: Manages keyboard shortcuts and user input
//! - `whisper`: Provides speech recognition using the Whisper model
//! - `app`: Contains the main application logic and state management
//!
//! # Environment Variables
//! - `RUST_LOG`: Controls log level (defaults to "info" if not set)
//!   - Example: `RUST_LOG=debug cargo run` for more detailed logging
//!   - Available levels: error, warn, info, debug, trace
#![deny(missing_docs)]
#![deny(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use anyhow::Result;
use env_logger::{self, Builder, Env};
use log::LevelFilter;
use std::sync::Once;

mod app;
mod audio;
mod keyboard;
mod whisper;

static INIT: Once = Once::new();

/// Initialize the logger with the given filter.
///
/// This is a test helper function that allows us to control the log level
/// in our tests without affecting the global logger state.
fn init_test_logger() {
    INIT.call_once(|| {
        Builder::from_env(Env::default().default_filter_or("info"))
            .filter_module("whispering", LevelFilter::Info)
            .filter_module("whisper", LevelFilter::Info)
            .filter_level(LevelFilter::Off)
            .init();
    });
}

/// Main entry point for the Whispering application.
///
/// Initializes logging with a default "info" level (can be overridden via RUST_LOG environment variable),
/// sets up the application, and runs the main event loop.
#[tokio::main]
async fn main() -> Result<()> {
    init_test_logger();
    let mut app = app::App::new().await?;
    app.run().await
}
