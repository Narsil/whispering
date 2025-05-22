//! Audio recording functionality.
//!
//! This module provides functionality for recording audio from the default input device
//! and saving it to a WAV file. It handles device initialization, stream configuration,
//! and audio data processing.

use anyhow::{Context, Result, anyhow};
use cpal::SupportedStreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hf_hub::api::tokio::ApiBuilder;
// use hound::{WavSpec, WavWriter};
use log::{debug, error, info, warn};
use ringbuf::traits::Observer;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer},
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

use crate::audio::resample::{Resample, audio_resample};
use crate::config::Config;

mod silero;
use silero::Silero;

use super::Audio;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum VADEvent {
    StartSpeech,
    EndSpeech(Vec<f32>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VADStateEnum {
    /// Completely silent, no speech detected
    Silent,
    /// Speech detected but not yet reached threshold to start recording
    SpeechDetected,
    /// Actively recording speech
    Recording,
    /// Silence detected but still within the speech threshold
    SilenceDetected,
}

struct VADState {
    state: VADStateEnum,
    speech_samples: usize,
    silence_samples: usize,
    speech_threshold_samples: usize,
    silence_threshold_samples: usize,
    threshold: f32,
    audio_buffer: HeapRb<f32>,
    pre_buffer: HeapRb<f32>,
}

impl VADState {
    fn new(
        threshold: f32,
        speech_duration: f32,
        silence_duration: f32,
        pre_buffer_duration: f32,
    ) -> Self {
        // Calculate sizes based on sample rate (16kHz)
        let sample_rate = 16000.0;
        let pre_buffer_size = (sample_rate * pre_buffer_duration) as usize;
        let speech_threshold_samples = (sample_rate * speech_duration) as usize;
        let silence_threshold_samples = (sample_rate * silence_duration) as usize;

        Self {
            state: VADStateEnum::Silent,
            speech_samples: 0,
            silence_samples: 0,
            speech_threshold_samples,
            silence_threshold_samples,
            threshold,
            // Create a large enough buffer for the maximum possible recording length
            audio_buffer: HeapRb::new(16000 * 60), // 60 seconds buffer
            pre_buffer: HeapRb::new(pre_buffer_size),
        }
    }

    /// Process a frame: update state and manage buffers in sync
    fn process_frame(&mut self, speech_prob: f32, samples: &[f32; N_SAMPLES]) -> Option<VADEvent> {
        // Buffer management (pre-buffer and audio buffer) is now always in sync with state
        let pre_buffer_capacity: usize = self.pre_buffer.capacity().into();
        let samples_to_add = samples.len();
        // Pre-buffer management
        if self.pre_buffer.occupied_len() + samples_to_add > pre_buffer_capacity {
            let samples_to_drop =
                self.pre_buffer.occupied_len() + samples_to_add - pre_buffer_capacity;
            let mut drop_buffer = vec![0.0; samples_to_drop];
            let _ = self.pre_buffer.pop_slice(&mut drop_buffer);
        }
        let n = self.pre_buffer.push_slice(samples);
        if n != samples.len() {
            error!("Failed to add samples to pre-buffer");
        }

        // Audio buffer management (only if recording)
        if self.state != VADStateEnum::Silent {
            let audio_buffer_capacity: usize = self.audio_buffer.capacity().into();
            let samples_to_add = samples.len();
            if self.audio_buffer.occupied_len() + samples_to_add > audio_buffer_capacity {
                let samples_to_drop =
                    self.audio_buffer.occupied_len() + samples_to_add - audio_buffer_capacity;
                let mut drop_buffer = vec![0.0; samples_to_drop];
                let _ = self.audio_buffer.pop_slice(&mut drop_buffer);
            }
            let n = self.audio_buffer.push_slice(samples);
            if n != samples.len() {
                error!("Audio buffer full, dropping samples");
            }
        }

        match self.state {
            VADStateEnum::Silent => {
                if speech_prob > self.threshold {
                    self.speech_samples += N_SAMPLES;
                    self.silence_samples = 0;
                    if self.speech_samples >= self.speech_threshold_samples {
                        self.state = VADStateEnum::Recording;
                        self.audio_buffer.clear();
                        // Add pre-buffer to the start of audio_buffer
                        let mut temp = vec![0.0; self.pre_buffer.occupied_len()];
                        let n = self.pre_buffer.occupied_len();
                        let n2 = self.pre_buffer.pop_slice(&mut temp);
                        assert_eq!(n, n2);
                        let n3 = self.audio_buffer.push_slice(&temp[..n]);
                        assert_eq!(n2, n3);
                        info!(
                            "Got {n} samples for pre buffer: this is {}s",
                            n as f32 / 16_000.0
                        );
                        return Some(VADEvent::StartSpeech);
                    } else {
                        self.state = VADStateEnum::SpeechDetected;
                    }
                } else {
                    self.silence_samples += N_SAMPLES;
                    self.speech_samples = 0;
                }
            }
            VADStateEnum::SpeechDetected => {
                if speech_prob > self.threshold {
                    self.speech_samples += N_SAMPLES;
                    self.silence_samples = 0;
                    if self.speech_samples >= self.speech_threshold_samples {
                        self.state = VADStateEnum::Recording;
                        self.audio_buffer.clear();
                        // Add pre-buffer to the start of audio_buffer
                        let n = self.pre_buffer.occupied_len();
                        let mut temp = vec![0.0; n];
                        let n2 = self.pre_buffer.pop_slice(&mut temp);
                        let n3 = self.audio_buffer.push_slice(&temp[..n]);
                        assert_eq!(n, n2);
                        assert_eq!(n2, n3);
                        debug!(
                            "Got {n} samples for pre buffer: this is {}s",
                            n as f32 / 16_000.0
                        );
                        return Some(VADEvent::StartSpeech);
                    }
                } else {
                    self.state = VADStateEnum::Silent;
                    self.silence_samples += N_SAMPLES;
                    self.speech_samples = 0;
                }
            }
            VADStateEnum::Recording => {
                if speech_prob > self.threshold {
                    self.speech_samples += N_SAMPLES;
                    self.silence_samples = 0;
                } else {
                    self.silence_samples += N_SAMPLES;
                    self.speech_samples = 0;
                    if self.silence_samples >= self.silence_threshold_samples {
                        self.state = VADStateEnum::Silent;
                        // Collect all samples from the audio buffer
                        let mut samples = vec![0.0; self.audio_buffer.occupied_len()];
                        let n = self.audio_buffer.pop_slice(&mut samples);
                        samples.truncate(n);
                        return Some(VADEvent::EndSpeech(samples));
                    } else {
                        self.state = VADStateEnum::SilenceDetected;
                    }
                }
            }
            VADStateEnum::SilenceDetected => {
                if speech_prob > self.threshold {
                    self.state = VADStateEnum::Recording;
                    self.speech_samples += N_SAMPLES;
                    self.silence_samples = 0;
                } else {
                    self.silence_samples += N_SAMPLES;
                    self.speech_samples = 0;
                    if self.silence_samples >= self.silence_threshold_samples {
                        self.state = VADStateEnum::Silent;
                        // Collect all samples from the audio buffer
                        let mut samples = vec![0.0; self.audio_buffer.occupied_len()];
                        let n = self.audio_buffer.pop_slice(&mut samples);
                        samples.truncate(n);
                        return Some(VADEvent::EndSpeech(samples));
                    }
                }
            }
        }
        None
    }
}

/// Handles audio recording functionality.
///
/// This struct manages the audio recording process, including device initialization,
/// stream configuration, and writing audio data to a WAV file.
pub struct AudioRecorder {
    stream: Arc<Mutex<cpal::Stream>>,
}

pub const N_SAMPLES: usize = 512;

impl AudioRecorder {
    // fn create_wav_spec(config: &crate::config::AudioConfig) -> WavSpec {
    //     WavSpec {
    //         channels: config.channels,
    //         sample_rate: config.sample_rate,
    //         bits_per_sample: 32,
    //         sample_format: hound::SampleFormat::Float,
    //     }
    // }

    /// Creates a new AudioRecorder instance.
    ///
    /// This function initializes the default audio input device, configures it
    /// for recording, and sets up the WAV file writer.
    pub async fn new(
        config: &Config,
        threshold: f32,
        silence_duration: f32,
        speech_duration: f32,
        pre_buffer_duration: f32,
        tx_audio: UnboundedSender<Audio>,
    ) -> Result<Self> {
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
        let mut vad_state = VADState::new(
            threshold,
            speech_duration,
            silence_duration,
            pre_buffer_duration,
        );

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

        // let recording_path = config.paths.recording_path.clone();
        // let wav_spec = Self::create_wav_spec(&config.audio);

        // let recording_path2 = recording_path.clone();

        let mut i = 0;
        let stream = Arc::new(Mutex::new(
            device
                .build_input_stream(
                    &stream_config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let data = if let Some(resampler) = resampler {
                            // Convert the input samples to f32
                            let samples: Vec<f32> = data.to_vec();

                            // Resample the stereo audio to the desired sample rate
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
                            samples
                        } else {
                            data.to_vec()
                        };

                        // Write to WAV file
                        let buf = &mut buffer;
                        for &sample in &data {
                            if buf.try_push(sample).is_err() {
                                error!("Buffer full, dropping samples");
                            }
                        }

                        // Process chunks of N_SAMPLES samples while we have enough data
                        while buf.occupied_len() >= N_SAMPLES {
                            i += 1;
                            // Get a chunk of N_SAMPLES samples efficiently
                            let n = buf.pop_slice(&mut temp_chunk);
                            assert_eq!(n, N_SAMPLES, "Expected to pop N_SAMPLES from buffer");
                            // Process the chunk
                            let speech_prob: f32 =
                                if vad_state.state == VADStateEnum::Silent && i % 1 != 0 {
                                    0.4
                                } else {
                                    silero.calc_level(&temp_chunk).expect("Prob")
                                };
                            // Update VAD state and handle events
                            if let Some(event) = vad_state.process_frame(speech_prob, &temp_chunk) {
                                match event {
                                    VADEvent::StartSpeech => {
                                        tx_audio.send(Audio::Warm).expect("Send warm event");
                                        info!("Speech detected");
                                    }
                                    VADEvent::EndSpeech(audio) => {
                                        // TODO This is debugging audio range.
                                        // if let Ok(mut writer) =
                                        //     WavWriter::create(&recording_path2, wav_spec)
                                        // {
                                        //     for &sample in &audio {
                                        //         writer.write_sample(sample).ok();
                                        //     }
                                        //     writer.finalize().ok();
                                        // }
                                        // info!(
                                        //     "Wrote wav file at {} : {wav_spec:?}",
                                        //     recording_path2.display()
                                        // );

                                        tx_audio
                                            .send(Audio::Sample(audio))
                                            .expect("Send the example");
                                        info!("Speech finished");
                                    }
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .context("Failed to create audio stream")?,
        ));

        let result = Self { stream };

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn create_test_vad_state() -> VADState {
        VADState::new(
            0.5, // threshold
            0.1, // speech_duration (100ms)
            0.1, // silence_duration (100ms)
            0.1, // pre_buffer_duration (500ms)
        )
    }

    #[test]
    fn test_vad_state_transitions() {
        let mut state = create_test_vad_state();

        // Test Silent -> SpeechDetected transition
        // assert_eq!(state.state, VADStateEnum::Silent);
        // let event = state.process_frame(0.4, &[0.0; N_SAMPLES]);
        // assert!(event.is_none());
        assert_eq!(state.state, VADStateEnum::Silent);
        let event = state.process_frame(0.6, &[0.0; N_SAMPLES]);
        assert_eq!(state.state, VADStateEnum::SpeechDetected);
        assert!(event.is_none());

        // Test SpeechDetected -> Recording transition
        // Need to send enough samples to cross speech threshold
        let samples_needed = state.speech_threshold_samples / N_SAMPLES;
        for _ in 0..samples_needed - 1 {
            let event = state.process_frame(0.6, &[0.1; N_SAMPLES]);
            assert_eq!(event, None);
            assert_eq!(state.state, VADStateEnum::SpeechDetected);
        }
        let event = state.process_frame(0.6, &[0.2; N_SAMPLES]);
        assert_eq!(event, Some(VADEvent::StartSpeech));
        assert_eq!(state.state, VADStateEnum::Recording);

        // Test Recording -> SilenceDetected transition
        let event = state.process_frame(0.4, &[0.3; N_SAMPLES]);
        assert_eq!(state.state, VADStateEnum::SilenceDetected);
        assert_eq!(event, None);

        // Test SilenceDetected -> Recording transition (speech resumes)
        let event = state.process_frame(0.6, &[0.4; N_SAMPLES]);
        assert_eq!(state.state, VADStateEnum::Recording);
        assert!(event.is_none());

        // Need to send enough samples to cross silence threshold
        let samples_needed = state.silence_threshold_samples / N_SAMPLES;
        for _ in 0..samples_needed {
            let event = state.process_frame(0.4, &[0.5; N_SAMPLES]);
            assert_eq!(event, None);
            assert_eq!(state.state, VADStateEnum::SilenceDetected);
        }
        let event = state.process_frame(0.4, &[0.6; N_SAMPLES]);
        assert_eq!(state.state, VADStateEnum::Silent);
        let Some(VADEvent::EndSpeech(s)) = &event else {
            panic!("Expected end of speech")
        };
        let mut out = BTreeMap::new();
        for samp in s {
            let count = out.entry(samp.to_string()).or_insert(0);
            *count += 1;
        }
        println!("out {out:?}");
        assert_eq!(
            out,
            BTreeMap::from([
                ("0".to_string(), 64),
                ("0.1".to_string(), 1024),
                ("0.2".to_string(), 512),
                ("0.3".to_string(), 512),
                ("0.4".to_string(), 512),
                ("0.5".to_string(), 1536),
                ("0.6".to_string(), 512)
            ])
        );
    }

    #[test]
    fn test_threshold_edge_cases() {
        let mut state = create_test_vad_state();

        // Test exactly at threshold
        let event = state.process_frame(0.5, &[0.0; N_SAMPLES]);
        assert_eq!(state.state, VADStateEnum::Silent);
        assert!(event.is_none());

        // Test just above threshold
        let event = state.process_frame(0.5001, &[0.0; N_SAMPLES]);
        assert_eq!(state.state, VADStateEnum::SpeechDetected);
        assert!(event.is_none());

        // Test just below threshold
        let event = state.process_frame(0.4999, &[0.0; N_SAMPLES]);
        assert_eq!(state.state, VADStateEnum::Silent);
        assert!(event.is_none());
    }

    #[test]
    fn test_pre_buffer_content() {
        let mut state = create_test_vad_state();

        // Add some samples to pre-buffer
        let test_samples = &[1.0; N_SAMPLES];
        state.process_frame(0.0, test_samples);

        // Verify pre-buffer contains the samples
        let mut buffer = vec![0.0; test_samples.len()];
        let n = state.pre_buffer.pop_slice(&mut buffer);
        assert_eq!(n, test_samples.len());
        assert_eq!(buffer, test_samples);
    }
}
