use std::path::PathBuf;

use anyhow::Result;

use crate::config::{Config, Trigger};

mod push_to_talk;
mod vad;

pub enum AudioRecorder {
    Push(push_to_talk::AudioRecorder),
    Vad(vad::AudioRecorder),
}

impl AudioRecorder {
    pub fn new(config: &Config) -> Result<Self> {
        match config.activation.trigger {
            Trigger::PushToTalk => Ok(Self::Push(push_to_talk::AudioRecorder::new(config)?)),
            Trigger::ToggleVad {
                threshold,
                silence_duration,
                speech_duration,
            } => Ok(Self::Vad(vad::AudioRecorder::new(config)?)),
        }
    }
    pub fn start_recording(&mut self) -> Result<()> {
        match self {
            Self::Push(p) => p.start_recording(),
            Self::Vad(p) => p.start_recording(),
        }
    }

    pub fn stop_recording(&mut self) -> Result<PathBuf> {
        match self {
            Self::Push(p) => p.stop_recording(),
            Self::Vad(p) => p.stop_recording(),
        }
    }
}
