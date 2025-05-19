//! Audio recording functionality.
//!
//! This module provides functionality for recording audio from the default input device
//! and saving it to a WAV file. It handles device initialization, stream configuration,
//! and audio data processing.

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SupportedStreamConfig};
use hound::{WavSpec, WavWriter};
use log::{debug, error, info, warn};
use rubato::{FftFixedInOut, Resampler};
use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

use crate::audio::resample::Resample;
use crate::config::{AudioConfig, Config};

use super::Audio;
use super::resample::audio_resample;

type WavWriterHandle = Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>;

/// Handles audio recording functionality.
///
/// This struct manages the audio recording process, including device initialization,
/// stream configuration, and writing audio data to a WAV file.
pub struct AudioRecorder {
    writer: WavWriterHandle,
    stream: cpal::Stream,
    recording_path: PathBuf,
    config: AudioConfig,
    tx_audio: UnboundedSender<Audio>,
}

impl AudioRecorder {
    /// Creates a new WAV specification for recording.
    fn create_wav_spec(config: &AudioConfig) -> WavSpec {
        WavSpec {
            channels: config.channels,
            sample_rate: config.sample_rate,
            bits_per_sample: config.sample_format.bits_per_sample(),
            sample_format: match config.sample_format {
                crate::config::SampleFormat::F32 => hound::SampleFormat::Float,
                crate::config::SampleFormat::I16 => hound::SampleFormat::Int,
            },
        }
    }

    /// Creates a new AudioRecorder instance.
    ///
    /// This function initializes the default audio input device, configures it
    /// for recording, and sets up the WAV file writer.
    pub fn new(config: &Config, tx_audio: UnboundedSender<Audio>) -> Result<Self> {
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

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&config.paths.cache_dir).context("Creating cache directory")?;

        // Create WAV writer
        let writer = WavWriter::create(
            &config.paths.recording_path,
            Self::create_wav_spec(&config.audio),
        )
        .context("Wav writer failed")?;
        let writer = Arc::new(Mutex::new(Some(writer)));
        let writer2 = writer.clone();
        let err_fn = move |err| {
            error!("Audio stream error: {}", err);
        };

        // Create resampler if needed
        let resampler = if stream_config.sample_rate().0 != config.audio.sample_rate
            || stream_config.channels() != config.audio.channels
            || stream_config.sample_format() != cpal::SampleFormat::F32
        {
            if stream_config.sample_format() != cpal::SampleFormat::F32 {
                todo!("Unimplemented resampling samples");
            }
            Some(Resample {
                samplerate_in: stream_config.sample_rate().0,
                samplerate_out: 16000,
                in_channels: stream_config.channels(),
            })
        } else {
            None
        };

        let stream = device
            .build_input_stream(
                &stream_config.into(),
                move |data, _: &_| {
                    Self::write_input_data_sample::<f32, f32>(data, &writer2, resampler);
                },
                err_fn,
                None,
            )
            .context("Failed to create audio stream")?;

        stream.pause().context("Cannot pause")?;

        Ok(Self {
            writer,
            stream,
            tx_audio,
            recording_path: config.paths.recording_path.clone(),
            config: config.audio.clone(),
        })
    }

    /// Starts the audio recording.
    ///
    /// This function begins capturing audio from the input device and writing
    /// it to the WAV file.
    pub fn start_recording(&self) -> Result<()> {
        let writer = WavWriter::create(&self.recording_path, Self::create_wav_spec(&self.config))?;
        *self
            .writer
            .lock()
            .map_err(|e| anyhow!("Failed to lock writer: {}", e))? = Some(writer);
        self.stream.play()?;
        self.tx_audio.send(Audio::Warm)?;
        Ok(())
    }

    /// Stops the audio recording and returns the path to the recorded file.
    ///
    /// This function stops the audio stream, finalizes the WAV file, and returns
    /// the path to the recorded audio file.
    pub fn stop_recording(&self) -> Result<()> {
        self.stream.pause()?;
        let writer = self
            .writer
            .lock()
            .map_err(|e| anyhow!("Failed to lock writer: {}", e))?
            .take()
            .ok_or_else(|| anyhow!("Writer is missing"))?;
        writer.finalize()?;
        let wav_path = self.recording_path.clone();
        self.tx_audio.send(Audio::Path(wav_path))?;
        Ok(())
    }

    fn write_input_data_sample<T, U>(
        input: &[T],
        writer: &WavWriterHandle,
        resampler: Option<Resample>,
    ) where
        T: Sample + rubato::Sample,
        U: Sample + hound::Sample + FromSample<T>,
        FftFixedInOut<T>: Resampler<T>,
    {
        if let Some(resampler) = resampler {
            // Convert the input samples to f32
            let samples: Vec<f32> = input
                .iter()
                .map(|s| s.to_float_sample().to_sample())
                .collect();

            // Resample the stereo audio to the desired sample rate
            // let resampled_stereo: Vec<f32> = audio_resample(&samples, sample_rate, 16000, channels);
            let resampled_stereo: Vec<f32> = audio_resample(
                &samples,
                resampler.samplerate_in,
                resampler.samplerate_out,
                resampler.in_channels,
            );

            let samples = if resampler.in_channels != 1 {
                let n = resampler.in_channels as usize;
                // Convert the resampled stereo audio to mono
                let mono_samples: Vec<_> = resampled_stereo
                    .chunks(n)
                    .map(|chunk| {
                        let mono_sample = (chunk.iter().sum::<f32>()) / n as f32; // Average channels
                        mono_sample
                    })
                    .collect();
                mono_samples
            } else {
                resampled_stereo
            };
            if let Ok(mut guard) = writer.try_lock() {
                if let Some(writer) = guard.as_mut() {
                    for &sample in samples.iter() {
                        // let sample: U = U::from_sample(sample);
                        writer.write_sample(sample).ok();
                    }
                }
            }
        } else if let Ok(mut guard) = writer.try_lock() {
            if let Some(writer) = guard.as_mut() {
                for &sample in input.iter() {
                    let sample: U = U::from_sample(sample);
                    writer.write_sample(sample).ok();
                }
            }
        }
    }
}
