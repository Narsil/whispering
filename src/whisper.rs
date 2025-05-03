//! Whisper model integration for speech recognition.
//!
//! This module provides functionality for downloading and running the Whisper model
//! for speech-to-text transcription. It handles model management and audio processing.

use anyhow::{Result, anyhow};
use hf_hub::api::tokio::ApiBuilder;
use std::path::{Path, PathBuf};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, install_logging_hooks,
};

/// Downloads the Whisper model from Hugging Face Hub.
///
/// This function fetches the "ggml-base.en.bin" model from the "ggerganov/whisper.cpp"
/// repository. The model is cached locally after the first download.
pub async fn download_model() -> Result<PathBuf> {
    let api = ApiBuilder::from_env().build()?;
    let repo = api.model("ggerganov/whisper.cpp".to_string());
    let filename = repo.get("ggml-base.en.bin").await?;
    Ok(filename)
}

/// Runs the Whisper model on an audio file.
///
/// This function takes a WAV file, processes it through the Whisper model,
/// and returns the transcribed text.
pub fn run_whisper(model_path: &Path, wav_path: &Path) -> Result<String> {
    install_logging_hooks();

    // load a context and model
    let ctx = WhisperContext::new_with_params(
        &model_path.display().to_string(),
        WhisperContextParameters::default(),
    )
    .map_err(|e| anyhow!("Failed to load model: {}", e))?;

    // create a params object
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 5 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Load and process the WAV file
    let reader =
        hound::WavReader::open(wav_path).map_err(|e| anyhow!("Failed to open WAV file: {}", e))?;
    let samples: Vec<f32> = reader
        .into_samples::<f32>()
        .filter_map(Result::ok)
        .collect();

    // Create state and run the model
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow!("Failed to create state: {}", e))?;

    state
        .full(params, &samples[..])
        .map_err(|e| anyhow!("Failed to run model: {}", e))?;

    // Fetch results
    let num_segments = state
        .full_n_segments()
        .map_err(|e| anyhow!("Failed to get number of segments: {}", e))?;

    let mut output = String::new();
    for i in 0..num_segments {
        let segment = state
            .full_get_segment_text(i)
            .map_err(|e| anyhow!("Failed to get segment {}: {}", i, e))?;
        output.push_str(&segment);
    }
    Ok(output)
}
