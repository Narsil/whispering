use std::path::PathBuf;

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::{Config, Trigger};

mod push_to_talk;
mod resample;
mod vad;

pub enum AudioRecorder {
    Push(push_to_talk::AudioRecorder),
    Vad(vad::AudioRecorder),
}

pub enum Audio {
    Warm,
    Path(PathBuf),
    Sample(Vec<f32>),
}

impl AudioRecorder {
    pub async fn new(config: &Config, tx_audio: UnboundedSender<Audio>) -> Result<Self> {
        match config.activation.trigger {
            Trigger::PushToTalk => Ok(Self::Push(push_to_talk::AudioRecorder::new(
                config, tx_audio,
            )?)),
            Trigger::ToggleVad {
                threshold,
                silence_duration,
                speech_duration,
                pre_buffer_duration,
            } => Ok(Self::Vad(
                vad::AudioRecorder::new(
                    config,
                    threshold,
                    silence_duration,
                    speech_duration,
                    pre_buffer_duration,
                    tx_audio,
                )
                .await?,
            )),
        }
    }
    pub fn start_recording(&mut self) -> Result<()> {
        match self {
            Self::Push(p) => p.start_recording(),
            Self::Vad(p) => p.start_recording(),
        }
    }

    pub fn stop_recording(&mut self) -> Result<()> {
        match self {
            Self::Push(p) => p.stop_recording(),
            Self::Vad(p) => p.stop_recording(),
        }
    }
}
