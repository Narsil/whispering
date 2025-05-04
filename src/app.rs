//! Main application logic and state management.
//!
//! This module contains the core application logic, including state management,
//! event handling, and coordination between different components of the application.

use anyhow::{Context, Result, anyhow};
use log::{error, info};
use notify_rust::Notification;
use rdev::{EventType, Key, listen, simulate};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;
use whisper_rs::install_logging_hooks;

use crate::audio::AudioRecorder;
use crate::config::Config;
use crate::keyboard::paste;
use crate::whisper::{download_model, run_whisper};

/// Represents the current state of the application.
///
/// This struct tracks whether the modifier key is pressed and whether
/// audio recording is currently in progress.
#[derive(Debug, PartialEq)]
struct State {
    pressed_keys: HashSet<Key>,
    recording: bool,
}

/// Main application struct that coordinates all components.
///
/// This struct manages the application state, audio recording, and
/// keyboard event handling. It coordinates between the audio recorder,
/// Whisper model, and keyboard input simulation.
pub struct App {
    state: State,
    recorder: AudioRecorder,
    model_path: PathBuf,
    config: Config,
}

impl App {
    /// Creates a new App instance.
    ///
    /// This function initializes the application by:
    /// 1. Loading configuration from config.toml or using defaults
    /// 2. Setting up the audio recorder
    /// 3. Loading the Whisper model
    pub async fn new() -> Result<Self> {
        // Load configuration
        let mut config_file = dirs::config_dir()
            .context("Cannot find config directory")
            .unwrap_or_else(|_| PathBuf::from("~/.config"));
        config_file.push("whispering");
        config_file.push("config.toml");
        let config = Config::load()?;

        // Initialize audio recorder
        let recorder = AudioRecorder::new(&config)?;

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&config.paths.cache_dir)?;

        // Download model if it doesn't exist
        let model_path = download_model(&config).await?;

        install_logging_hooks();
        Ok(Self {
            state: State {
                pressed_keys: HashSet::new(),
                recording: false,
            },
            recorder,
            model_path,
            config,
        })
    }

    /// Runs the main application loop.
    ///
    /// This function sets up the keyboard event listener and processes
    /// events until the application is terminated. It handles the configured
    /// shortcut for starting/stopping recording.
    pub async fn run(&mut self) -> Result<()> {
        let (schan, mut rchan) = unbounded_channel();
        let _listener = tokio::task::spawn_blocking(move || {
            if let Err(e) = listen(move |event| {
                if let Err(e) = schan.send(event.clone()) {
                    error!("Could not send event {event:?}: {:#?}", e);
                }
            }) {
                error!("Could not listen for events: {:#?}", e);
                return Err(anyhow!("Failed to listen for events: {:#?}", e));
            }
            Ok(())
        });

        info!(
            "Press {:?} to start recording, release the last key to stop",
            self.config.shortcuts.keys
        );

        while let Some(event) = rchan.recv().await {
            self.handle_event(event)?;
        }
        info!("Done exiting");
        Ok(())
    }

    /// Handles keyboard events.
    ///
    /// This function processes keyboard events and updates the application state
    /// accordingly. It manages the recording state and triggers transcription
    /// when recording stops.
    fn handle_event(&mut self, event: rdev::Event) -> Result<()> {
        match event.event_type {
            EventType::KeyPress(key) => {
                if self.config.shortcuts.keys.contains(&key) {
                    self.state.pressed_keys.insert(key);
                }
                // Check if all required keys are pressed
                let all_keys_pressed = self.config.shortcuts.keys == self.state.pressed_keys;

                if all_keys_pressed && !self.state.recording {
                    self.state.recording = true;
                    info!("Starting recording...");
                    self.recorder.start_recording()?;

                    // Show desktop notification
                    Notification::new()
                        .summary("Recording...")
                        .body("Recording started")
                        .icon("audio-input-microphone")
                        .show()?;
                }
            }
            EventType::KeyRelease(key) => {
                self.state.pressed_keys.retain(|&k| k != key);

                // If we were recording and any required key is released, stop recording
                if self.state.recording && self.state.pressed_keys != self.config.shortcuts.keys {
                    self.state.recording = false;
                    info!("Stopping recording...");
                    let wav_path = self.recorder.stop_recording()?;
                    info!("Transcribing audio...");
                    let output = run_whisper(&self.model_path, &wav_path)?;
                    let summary = if output.len() > 20 {
                        &format!("{}..", &output[..20])
                    } else {
                        &output
                    };
                    // Show notification with transcribed text
                    Notification::new()
                        .summary(summary)
                        .body(&output)
                        .icon("audio-input-microphone")
                        .show()?;

                    paste(output)?;
                    // Always end by pressing Return to submit
                    simulate(&EventType::KeyPress(Key::Return))?;
                    simulate(&EventType::KeyRelease(Key::Return))?;
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            _ => (),
        }
        Ok(())
    }
}
