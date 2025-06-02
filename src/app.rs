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

use crate::asr::{Asr, download_model};
use crate::audio::{Audio, AudioRecorder};
use crate::config::{Config, Trigger};
use crate::keyboard::paste;

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
    // asr: Asr,
    config: Config,
    // rx_audio: UnboundedReceiver<PathBuf>,
}

async fn handle_audio(asr: &mut Asr, config: &Config, audio: Audio) -> Result<()> {
    let samples: Option<Vec<f32>> = match audio {
        Audio::Warm => {
            asr.load().expect("Load");
            None
        }
        Audio::Sample(samples) => Some(samples),
        Audio::Path(wav_path) => {
            info!("Transcribing audio...");
            let samples = asr.samples_from_file(&wav_path).expect("Read wav");
            Some(samples)
        }
    };
    if let Some(samples) = samples {
        info!("Transcribing audio...");
        let output = asr.run(samples, &config).context("Error running ASR")?;
        if output.is_empty() {
            // Show notification with transcribed text
            config.notify("No voice detected", &output);
            return Ok(());
        }

        // let output = "Toto".to_string();
        info!("Transcribed: {output}");
        let summary = if output.len() > 20 {
            &format!("{}..", &output[..20])
        } else {
            &output
        };
        // Show notification with transcribed text
        config.notify(summary, &output);

        paste(output).context("Pasting").expect("Pasting");
        // Always end by pressing Return to submit
        if config.activation.autosend {
            std::thread::sleep(Duration::from_millis(2));
            simulate(&EventType::KeyPress(Key::Return)).expect("simulate");
            std::thread::sleep(Duration::from_millis(2));
            simulate(&EventType::KeyRelease(Key::Return)).expect("simulate");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    Ok(())
}

impl App {
    /// Creates a new App instance.
    ///
    /// This function initializes the application by:
    /// 1. Loading configuration from config.toml or using defaults
    /// 2. Setting up the audio recorder
    /// 3. Loading the Whisper model
    pub async fn new(config_path: Option<PathBuf>) -> Result<Self> {
        // Load configuration
        let config = if let Some(path) = config_path {
            Config::from_file(&path).context(format!("Reading config {}", path.display()))?
        } else {
            Config::load_or_write_default(None)?
        };

        // Warm the handle.
        simulate(&EventType::KeyPress(Key::ControlLeft))?;
        std::thread::sleep(Duration::from_millis(2));
        simulate(&EventType::KeyRelease(Key::ControlLeft))?;

        // Initialize audio recorder
        let (tx_audio, mut rx_audio) = unbounded_channel();
        let recorder = AudioRecorder::new(&config, tx_audio)
            .await
            .context("Failed to create audio recorder")?;

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&config.paths.cache_dir)?;

        // Download model if it doesn't exist
        let model_path = download_model(&config)
            .await
            .context("Failed to download model")?;

        let asr = Asr::new(&model_path)?;
        let asr_config = config.clone();
        tokio::task::spawn(async move {
            let mut asr = asr;
            let asr_config = asr_config;
            while let Some(audio) = rx_audio.recv().await {
                if let Err(err) = handle_audio(&mut asr, &asr_config, audio).await {
                    error!("Error handling audio {err:?}");
                }
            }
        });
        Ok(Self {
            state: State {
                pressed_keys: HashSet::new(),
                recording: false,
            },
            recorder,
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

        let keys = &self.config.activation.keys;
        info!(
            "Press {:?} to start recording, release the last key to stop",
            keys
        );

        while let Some(event) = rchan.recv().await {
            if let Err(err) = self.handle_event(event) {
                error!("error handling event: {err}");
            }
        }

        info!("Done exiting");
        Ok(())
    }

    fn notify(&self, summary: &str, content: &str) {
        // Show desktop notification
        if self.config.activation.notify {
            if let Err(err) = Notification::new()
                .summary(summary)
                .body(content)
                .icon("audio-input-microphone")
                .show()
            {
                error!("Cannot show notification: {err} , content was : {summary} {content}")
            };
        }
    }

    /// Handles keyboard events.
    ///
    /// This function processes keyboard events and updates the application state
    /// accordingly. It manages the recording state and triggers transcription
    /// when recording stops.
    fn handle_event(&mut self, event: rdev::Event) -> Result<()> {
        match &self.config.activation.trigger {
            Trigger::PushToTalk => self.handle_event_push_to_talk(event),
            Trigger::ToggleVad { .. } => self.handle_event_vad(event),
        }
    }
    fn handle_event_vad(&mut self, event: rdev::Event) -> Result<()> {
        match event.event_type {
            EventType::KeyPress(key) => {
                let keys = &self.config.activation.keys;
                if keys.contains(&key) {
                    self.state.pressed_keys.insert(key);
                }
                // Check if all required keys are pressed
                let all_keys_pressed = keys == &self.state.pressed_keys;

                if all_keys_pressed {
                    self.state.recording = !self.state.recording;
                    if self.state.recording {
                        info!("Starting recording...");
                        self.notify("Start listening..", "");
                        self.recorder.start_recording()?;
                    } else {
                        info!("Stopped recording");
                        self.notify("Stop listening.", "");
                        self.recorder.stop_recording()?;
                    }
                }
            }
            EventType::KeyRelease(key) => {
                self.state.pressed_keys.retain(|&k| k != key);
            }
            _ => (),
        }
        Ok(())
    }
    fn handle_event_push_to_talk(&mut self, event: rdev::Event) -> Result<()> {
        match event.event_type {
            EventType::KeyPress(key) => {
                let keys = &self.config.activation.keys;
                if keys.contains(&key) {
                    self.state.pressed_keys.insert(key);
                }
                // Check if all required keys are pressed
                let all_keys_pressed = keys == &self.state.pressed_keys;

                if all_keys_pressed && !self.state.recording {
                    self.state.recording = true;
                    info!("Starting recording...");
                    self.recorder.start_recording()?;
                }
            }
            EventType::KeyRelease(key) => {
                self.state.pressed_keys.retain(|&k| k != key);

                let keys = &self.config.activation.keys;
                if self.state.recording && self.state.pressed_keys != *keys {
                    self.state.recording = false;
                    info!("Stopping recording...");
                    self.recorder.stop_recording()?;
                }
            }
            _ => (),
        }
        Ok(())
    }
}
