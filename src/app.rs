//! Main application logic and state management.
//!
//! This module contains the core application logic, including state management,
//! event handling, and coordination between different components of the application.

use anyhow::{Result, anyhow};
use log::{error, info};
use rdev::{EventType, Key, listen, simulate};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;

use crate::audio::AudioRecorder;
use crate::keyboard::paste;
use crate::whisper::{download_model, run_whisper};

/// Represents the current state of the application.
///
/// This struct tracks whether the control key is pressed and whether
/// audio recording is currently in progress.
#[derive(Debug, PartialEq)]
struct State {
    ctrl: bool,
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
}

impl App {
    /// Creates a new App instance.
    ///
    /// This function initializes the application by:
    /// 1. Downloading the Whisper model
    /// 2. Warming up the keyboard lock
    /// 3. Initializing the audio recorder
    pub async fn new() -> Result<Self> {
        info!("Starting whispering...");

        let model_path = download_model().await?;
        // Just warmup the lock
        simulate(&EventType::KeyPress(Key::ControlLeft))?;
        simulate(&EventType::KeyRelease(Key::ControlLeft))?;

        let recorder = AudioRecorder::new()?;

        Ok(Self {
            state: State {
                ctrl: false,
                recording: false,
            },
            recorder,
            model_path,
        })
    }

    /// Runs the main application loop.
    ///
    /// This function sets up the keyboard event listener and processes
    /// events until the application is terminated. It handles the Ctrl+Space
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

        info!("Ready to record. Press Ctrl+Space to start recording, release Space to stop.");

        while let Some(event) = rchan.recv().await {
            self.handle_event(event)?;
        }
        Ok(())
    }

    /// Handles keyboard events.
    ///
    /// This function processes keyboard events and updates the application state
    /// accordingly. It manages the recording state and triggers transcription
    /// when recording stops.
    fn handle_event(&mut self, event: rdev::Event) -> Result<()> {
        match event.event_type {
            EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
                self.state.ctrl = true;
            }
            EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
                self.state.ctrl = false;
            }
            EventType::KeyPress(Key::Space) => {
                if self.state.ctrl {
                    self.state.recording = true;
                    info!("Starting recording...");
                    self.recorder.start_recording()?;
                }
            }
            EventType::KeyRelease(Key::Space) => {
                if self.state.recording {
                    self.state.recording = false;
                    info!("Stopping recording...");
                    let wav_path = self.recorder.stop_recording()?;
                    info!("Transcribing audio...");
                    let output = run_whisper(&self.model_path, &wav_path)?;
                    paste(output)?;
                    // Always end by pressing Return to submit
                    std::thread::sleep(Duration::from_millis(200));
                    simulate(&EventType::KeyPress(Key::Return))?;
                    simulate(&EventType::KeyRelease(Key::Return))?;
                }
            }
            _ => (),
        }
        Ok(())
    }
}
