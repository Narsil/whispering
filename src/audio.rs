//! Audio recording functionality.
//!
//! This module provides functionality for recording audio from the default input device
//! and saving it to a WAV file. It handles device initialization, stream configuration,
//! and audio data processing.

use anyhow::{Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, StreamConfig};
use hound::{WavSpec, WavWriter};
use log::{error, info, warn};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

type WavWriterHandle = Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>;

/// Handles audio recording functionality.
///
/// This struct manages the audio recording process, including device initialization,
/// stream configuration, and writing audio data to a WAV file.
pub struct AudioRecorder {
    writer: WavWriterHandle,
    stream: cpal::Stream,
    recording_path: PathBuf,
}

impl AudioRecorder {
    /// Creates a new AudioRecorder instance.
    ///
    /// This function initializes the default audio input device, configures it
    /// for 16kHz mono recording, and sets up the WAV file writer.
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("Cannot find input device"))?;

        info!("Using input device: {}", device.name()?);

        let config = StreamConfig {
            channels: 1,                           // mono
            sample_rate: cpal::SampleRate(16_000), // 16kHz
            buffer_size: cpal::BufferSize::Default,
        };

        // Check for supported formats
        let mut supported_configs = device.supported_input_configs()?;
        let has_desired_format = supported_configs.any(|f| {
            f.min_sample_rate().0 <= 16_000
                && f.max_sample_rate().0 >= 16_000
                && f.channels() == 1
                && f.sample_format() == cpal::SampleFormat::F32
        });

        if !has_desired_format {
            warn!("Desired format not explicitly supported, stream may not work");
        }

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut cache_dir: PathBuf =
            dirs::cache_dir().ok_or_else(|| anyhow!("Cannot find cache directory"))?;
        cache_dir.push("whispering");
        std::fs::create_dir_all(&cache_dir)?;
        let mut path = cache_dir.clone();
        path.push("recorded.wav");

        let writer = WavWriter::create(&path, spec)?;
        let writer = Arc::new(Mutex::new(Some(writer)));
        let writer2 = writer.clone();

        let err_fn = move |err| {
            error!("Audio stream error: {}", err);
        };

        let stream = device.build_input_stream(
            &config,
            move |data, _: &_| Self::write_input_data::<f32, f32>(data, &writer2),
            err_fn,
            None,
        )?;

        Ok(Self {
            writer,
            stream,
            recording_path: path,
        })
    }

    /// Starts the audio recording.
    ///
    /// This function begins capturing audio from the input device and writing
    /// it to the WAV file.
    pub fn start_recording(&self) -> Result<()> {
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

    /// Writes audio data to the WAV file.
    ///
    /// This function is called by the audio stream callback to write the captured
    /// audio data to the WAV file.
    fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
    where
        T: Sample,
        U: Sample + hound::Sample + FromSample<T>,
    {
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
