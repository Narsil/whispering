//! Audio recording functionality.
//!
//! This module provides functionality for recording audio from the default input device
//! and saving it to a WAV file. It handles device initialization, stream configuration,
//! and audio data processing.

use anyhow::{Context, Result, anyhow};
use cpal::SupportedStreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hf_hub::api::tokio::ApiBuilder;
use log::{debug, error, info, warn};
use ringbuf::traits::Observer;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer},
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

use crate::config::{AudioConfig, Config};

mod silero;
use silero::Silero;

use super::Audio;

#[derive(Clone, Copy)]
struct Resample {
    samplerate_in: u32,
    samplerate_out: u32,
    in_channels: u16,
}

/// Handles audio recording functionality.
///
/// This struct manages the audio recording process, including device initialization,
/// stream configuration, and writing audio data to a WAV file.
pub struct AudioRecorder {
    stream: Arc<Mutex<cpal::Stream>>,
    config: AudioConfig,
}

pub const N_SAMPLES: usize = 512;

impl AudioRecorder {
    /// Creates a new AudioRecorder instance.
    ///
    /// This function initializes the default audio input device, configures it
    /// for recording, and sets up the WAV file writer.
    pub async fn new(config: &Config, tx_audio: UnboundedSender<Audio>) -> Result<Self> {
        let host = cpal::default_host();
        debug!("Available hosts: {:?}", cpal::available_hosts());
        debug!("Default host: {:?}", host.id());

        let devices = host.input_devices()?;
        let names: HashSet<_> = devices.into_iter().flat_map(|d| d.name()).collect();
        debug!("Available input devices: {names:?}");

        let mut devices = host.input_devices()?;
        // Find the requested device or use default
        let device = if let Some(device_name) = &config.audio.device {
            devices
                .find(|d| {
                    if let Ok(name) = d.name() {
                        name == *device_name
                    } else {
                        false
                    }
                })
                .ok_or_else(|| {
                    anyhow!(
                        "Requested audio device '{}' not found, available: {:?}",
                        device_name,
                        names
                    )
                })?
        } else {
            host.default_input_device()
                .ok_or_else(|| anyhow!("No default input device found"))?
        };

        info!("Using input device: {}", device.name()?);

        // Try to find a supported configuration that matches what we want
        let stream_config = if let Ok(supported_configs) = device.supported_input_configs() {
            let mut stream_config = None;

            for config_range in supported_configs {
                let sample_rate = cpal::SampleRate(config.audio.sample_rate);
                if config_range.min_sample_rate() <= sample_rate
                    && config_range.max_sample_rate() >= sample_rate
                    && config_range.sample_format() == config.audio.sample_format.into()
                {
                    stream_config = Some(config_range.with_sample_rate(sample_rate));
                    break;
                }
            }
            stream_config
        } else {
            None
        };
        let stream_config = if let Some(stream_config) = stream_config {
            Some(stream_config)
        } else {
            debug!("Could not find supported configs");
            if let Ok(default_config) = device.default_input_config() {
                debug!("Device default config: {:?}", default_config);
                Some(default_config)
            } else {
                warn!("Could not default_config");
                None
            }
        };

        // If we can't find an exact match, use the default config
        let stream_config = stream_config.unwrap_or_else(|| {
            warn!("Falling back to config defined configuration, It might not work");
            SupportedStreamConfig::new(
                config.audio.channels,
                cpal::SampleRate(config.audio.sample_rate),
                cpal::SupportedBufferSize::Unknown,
                config.audio.sample_format.into(),
            )
        });

        debug!("Using stream config: {:?}", stream_config);

        let err_fn = move |err| {
            error!("Audio stream error: {}", err);
        };

        let mut buffer = HeapRb::new(16000 * 2); // 2 seconds buffer at 16kHz
        let mut temp_chunk = [0.0; N_SAMPLES];
        let sample_rate = 16_000;
        let api = ApiBuilder::from_env().build()?;
        let model = api.model("Narsil/silero".to_string());
        let model_path = model.get("silero_vad.onnx").await?;
        let mut silero = Silero::new(sample_rate, model_path)?;
        let mut is_talking = false;
        let mut audio_chunk = vec![];
        let stream = Arc::new(Mutex::new(
            device
                .build_input_stream(
                    &stream_config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let buf = &mut buffer;
                        for &sample in data {
                            if buf.try_push(sample).is_err() {
                                eprintln!("Buffer full, dropping samples");
                            }
                        }
                        // Process chunks of 1024 samples while we have enough data
                        while buf.occupied_len() >= N_SAMPLES {
                            // Get a chunk of 1024 samples
                            for i in 0..N_SAMPLES {
                                let sample = buf.try_pop().expect("Sample to exist");
                                temp_chunk[i] = sample as f32 / i16::MAX as f32;
                            }

                            // Process the chunk
                            let speech_prob: f32 = silero.calc_level(&temp_chunk).expect("Prob");
                            // let speech_prob: f32 = temp_chunk.iter().map(|s| *s as f32).sum();
                            if speech_prob > 0.5 && !is_talking {
                                tx_audio.send(Audio::Warm).expect("Send the example");
                                info!("Speech detected");
                            } else if speech_prob < 0.5 && is_talking {
                                let chunk = audio_chunk.drain(..).collect();
                                tx_audio
                                    .send(Audio::Sample(chunk))
                                    .expect("Send the example");
                                info!("Speech finished");
                            }
                            is_talking = speech_prob > 0.5;
                            if is_talking {
                                audio_chunk.extend(temp_chunk);
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .context("Failed to create audio stream")?,
        ));

        // stream.pause().context("Cannot pause")?;

        let result = Self {
            stream,
            config: config.audio.clone(),
        };

        Ok(result)
    }

    /// Starts the audio recording.
    ///
    /// This function begins capturing audio from the input device and writing
    /// it to the WAV file.
    pub fn start_recording(&self) -> Result<()> {
        self.stream.lock().unwrap().play()?;
        Ok(())
    }

    /// Stops the audio recording and returns the path to the recorded file.
    ///
    /// This function stops the audio stream, finalizes the WAV file, and returns
    /// the path to the recorded audio file.
    pub fn stop_recording(&self) -> Result<()> {
        self.stream.lock().unwrap().pause()?;
        Ok(())
    }
}
