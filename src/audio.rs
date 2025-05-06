//! Audio recording functionality.
//!
//! This module provides functionality for recording audio from the default input device
//! and saving it to a WAV file. It handles device initialization, stream configuration,
//! and audio data processing.

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, StreamConfig};
use hound::{WavSpec, WavWriter};
use log::{error, info};
use rubato::{FftFixedInOut, Resampler};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::{AudioConfig, Config};

type WavWriterHandle = Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>;

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
    writer: WavWriterHandle,
    stream: cpal::Stream,
    recording_path: PathBuf,
    config: AudioConfig,
}

pub fn audio_resample(
    data: &[f32],
    sample_rate0: u32,
    sample_rate: u32,
    channels: u16,
) -> Vec<f32> {
    use samplerate::{ConverterType, convert};
    convert(
        sample_rate0 as _,
        sample_rate as _,
        channels as _,
        ConverterType::SincBestQuality,
        data,
    )
    .unwrap_or_default()
}

impl AudioRecorder {
    /// Creates a new WAV specification for recording.
    fn create_wav_spec(config: &AudioConfig) -> WavSpec {
        WavSpec {
            channels: config.channels,
            sample_rate: config.sample_rate,
            bits_per_sample: config.bits_per_sample,
            sample_format: match config.sample_format {
                crate::config::SampleFormat::Float => hound::SampleFormat::Float,
                crate::config::SampleFormat::Int => hound::SampleFormat::Int,
            },
        }
    }

    /// Creates a new AudioRecorder instance.
    ///
    /// This function initializes the default audio input device, configures it
    /// for recording, and sets up the WAV file writer.
    pub fn new(config: &Config) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("Cannot find input device"))?;

        info!("Using input device: {}", device.name()?);

        let stream_config = StreamConfig {
            channels: config.audio.channels,
            sample_rate: cpal::SampleRate(config.audio.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        // Check for supported formats
        let mut supported_configs = device.supported_input_configs()?;
        let has_desired_format = supported_configs.any(|f| {
            f.min_sample_rate().0 <= config.audio.sample_rate
                && f.max_sample_rate().0 >= config.audio.sample_rate
                && f.channels() == config.audio.channels
                && f.sample_format()
                    == match config.audio.sample_format {
                        crate::config::SampleFormat::Float => cpal::SampleFormat::F32,
                        crate::config::SampleFormat::Int => cpal::SampleFormat::I16,
                    }
        });

        if !has_desired_format {
            info!("Desired format not explicitly supported, falling back to resampling");
        }

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&config.paths.cache_dir)?;

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

        let stream = if let Ok(stream) = device.build_input_stream(
            &stream_config,
            move |data, _: &_| Self::write_input_data::<f32, f32>(data, &writer2),
            err_fn,
            None,
        ) {
            stream
        } else {
            let default_config = device.default_input_config()?;
            let samplerate_in = default_config.sample_rate().0;
            let samplerate_out = stream_config.sample_rate.0;
            let writer3 = writer.clone();
            let in_channels = default_config.channels();
            let sconfig: StreamConfig = default_config.into();
            let resample = Resample {
                samplerate_in,
                samplerate_out,
                in_channels,
            };
            device
                .build_input_stream(
                    &sconfig,
                    move |data, _: &_| {
                        Self::write_input_data_sample::<f32, f32>(data, &writer3, Some(resample))
                    },
                    err_fn,
                    None,
                )
                .context("Failed to create fallback stream")?
        };
        stream.pause()?;

        Ok(Self {
            writer,
            stream,
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
        Ok(())
    }

    /// Stops the audio recording and returns the path to the recorded file.
    ///
    /// This function stops the audio stream, finalizes the WAV file, and returns
    /// the path to the recorded audio file.
    pub fn stop_recording(&self) -> Result<PathBuf> {
        self.stream.pause()?;
        let writer = self
            .writer
            .lock()
            .map_err(|e| anyhow!("Failed to lock writer: {}", e))?
            .take()
            .ok_or_else(|| anyhow!("Writer is missing"))?;
        writer.finalize()?;
        Ok(self.recording_path.clone())
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
        } else {
            if let Ok(mut guard) = writer.try_lock() {
                if let Some(writer) = guard.as_mut() {
                    for &sample in input.iter() {
                        let sample: U = U::from_sample(sample);
                        writer.write_sample(sample).ok();
                    }
                }
            }
        }
    }

    /// Writes audio data to the WAV file.
    ///
    /// This function is called by the audio stream callback to write the captured
    /// audio data to the WAV file.
    fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
    where
        T: Sample + rubato::Sample,
        U: Sample + hound::Sample + FromSample<T>,
    {
        Self::write_input_data_sample::<T, U>(input, writer, None)
    }
}
