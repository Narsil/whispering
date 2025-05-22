//! Whisper model integration for speech recognition.
//!
//! This module provides functionality for downloading and running the Whisper model
//! for speech-to-text transcription. It handles model management and audio processing.

use anyhow::{Context, Result, anyhow};
use hf_hub::api::tokio::ApiBuilder;
use hound::{SampleFormat, WavReader};
use log::{debug, info};
use std::path::{Path, PathBuf};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::config::Config;

/// Downloads the Whisper model from Hugging Face Hub.
///
/// This function fetches the model from the specified repository and filename.
pub async fn download_model(config: &Config) -> Result<PathBuf> {
    let api = ApiBuilder::from_env().build()?;
    let repo = api.model(config.model.repo.clone());
    let filename = repo.get(&config.model.filename).await?;
    Ok(filename)
}

pub struct Asr {
    // TODO potentially enable keeping the context alive
    // for slow disk users, tradeoff is you keep
    // accelerator's memory used.
    // context: WhisperContext,
    model_path: PathBuf,

    context: Option<(WhisperContext, WhisperState)>,
}

impl Asr {
    pub fn new(model_path: &Path) -> Result<Self> {
        Ok(Self {
            model_path: model_path.to_path_buf(),
            context: None,
        })
    }

    pub fn load(&mut self) -> Result<()> {
        debug!("Trying to load model {}", self.model_path.display());
        let context = WhisperContext::new_with_params(
            &self.model_path.to_string_lossy(),
            WhisperContextParameters::default(),
        )
        .context(format!("Loading model at {}", self.model_path.display()))?;
        let state = context.create_state().context("Creating state")?;
        self.context = Some((context, state));
        info!("Context loaded");
        Ok(())
    }

    pub fn samples_from_file(&self, wav_path: &Path) -> Result<Vec<f32>> {
        let mut reader = WavReader::open(wav_path).context("Opening wav reader")?;
        let samples: Vec<f32> = if reader.spec().sample_format == SampleFormat::Float {
            reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect()
        } else {
            reader
                .samples::<i16>()
                .map(|s| s.unwrap_or(0) as f32 / 32768.0)
                .collect()
        };
        Ok(samples)
    }

    /// Runs the Whisper model on the given audio file.
    ///
    /// This function takes a path to a WAV file and returns the transcribed text.
    pub fn run(&mut self, samples: Vec<f32>, config: &Config) -> Result<String> {
        // Take context to let it drop later.
        let (_context, mut state) = self.context.take().ok_or(anyhow!("Context was not warm"))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_language(None);

        // Set initial prompt if available
        if let Some(prompt) = config.model.prompt.get_prompt_text() {
            params.set_initial_prompt(&prompt);
        }

        state.full(params, &samples).context("Setting context")?;

        let num_segments = state.full_n_segments()?;
        let mut text = String::new();
        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i)?;
            text.push_str(&segment);
            text.push(' ');
        }

        let mut text = text.trim().to_string();

        // Apply replacements
        for (from, to) in &config.model.replacements {
            text = text.replace(from, to);
        }

        Ok(text)
    }
}
