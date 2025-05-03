//! Main application logic and state management.
//!
//! This module contains the core application logic, including state management,
//! event handling, and coordination between different components of the application.

use anyhow::{Context, Result, anyhow};
use hound::WavReader;
use log::{error, info};
use rdev::{EventType, Key, listen, simulate};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, install_logging_hooks,
};

use crate::audio::AudioRecorder;
use crate::config::Config;
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
    context: WhisperContext,
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
        std::fs::create_dir_all(&config.cache_dir)?;

        // Download model if it doesn't exist
        let model_path = download_model(&config).await?;

        install_logging_hooks();
        // Load Whisper model
        let context = WhisperContext::new_with_params(
            &model_path.to_string_lossy(),
            WhisperContextParameters::default(),
        )?;

        Ok(Self {
            state: State {
                ctrl: false,
                recording: false,
            },
            recorder,
            model_path,
            context,
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

    /// Records audio and transcribes it.
    ///
    /// This function:
    /// 1. Records audio from the default input device
    /// 2. Saves it to a WAV file
    /// 3. Transcribes the audio using Whisper
    /// 4. Returns the transcription text
    pub fn record_and_transcribe(&self) -> Result<String> {
        // Start recording
        self.recorder.start_recording()?;

        // Wait for user input to stop recording
        println!("Press Enter to stop recording...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        // Stop recording and get the path to the recorded file
        let recording_path = self.recorder.stop_recording()?;

        // Read WAV file
        let mut reader = WavReader::open(&recording_path)?;
        let samples: Vec<f32> = if reader.spec().sample_format == hound::SampleFormat::Float {
            reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect()
        } else {
            reader
                .samples::<i16>()
                .map(|s| s.unwrap_or(0) as f32 / 32768.0)
                .collect()
        };

        // Transcribe the audio
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut state = self.context.create_state()?;
        state.full(params, &samples)?;

        let num_segments = state.full_n_segments()?;
        let mut text = String::new();
        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i)?;
            text.push_str(&segment);
            text.push(' ');
        }

        Ok(text.trim().to_string())
    }
}
