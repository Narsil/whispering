//! Configuration management for the Whispering application.
//!
//! This module provides functionality for loading and managing application
//! configuration, including audio recording settings and model parameters.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Audio recording configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Number of audio channels (1 for mono, 2 for stereo)
    pub channels: u16,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Bits per sample
    pub bits_per_sample: u16,
    /// Sample format (Float or Int)
    pub sample_format: SampleFormat,
}

/// Sample format for audio recording.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SampleFormat {
    /// 32-bit floating point samples
    Float,
    /// 16-bit integer samples
    Int,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        }
    }
}

/// Main application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Audio recording settings
    pub audio: AudioConfig,
    /// Path to the cache directory
    pub cache_dir: PathBuf,
    /// Path to the recorded audio file
    pub recording_path: PathBuf,
    /// Model configuration
    pub model: ModelConfig,
}

/// Whisper model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model repository on Hugging Face
    pub repo: String,
    /// Model filename
    pub filename: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            repo: "ggerganov/whisper.cpp".to_string(),
            filename: "ggml-base.en.bin".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .context("Cannot find config directory")
            .unwrap_or_else(|_| PathBuf::from("~/.config"));
        let mut config_dir = config_dir;
        config_dir.push("whispering");
        
        let cache_dir = dirs::cache_dir()
            .context("Cannot find cache directory")
            .unwrap_or_else(|_| PathBuf::from("~/.cache"));
        let mut cache_dir = cache_dir;
        cache_dir.push("whispering");
        
        let mut recording_path = cache_dir.clone();
        recording_path.push("recorded.wav");

        Self {
            audio: AudioConfig::default(),
            cache_dir,
            recording_path,
            model: ModelConfig::default(),
        }
    }
}

impl Config {
    /// Gets the default configuration file path.
    fn default_config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .context("Cannot find config directory")
            .unwrap_or_else(|_| PathBuf::from("~/.config"));
        let mut path = config_dir;
        path.push("whispering");
        path.push("config.toml");
        path
    }

    /// Loads configuration from a TOML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Saves configuration to a TOML file.
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Loads configuration from the default location, creating it if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::default_config_path();
        Self::load_from_path(&path)
    }

    fn load_from_path(path: &Path) -> Result<Self> {
        // If config exists, use it
        if path.exists() {
            return Self::from_file(path);
        }

        // If no config exists, create default config
        let config = Self::default();
        // Create config directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        config.save_to_file(path)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.audio.channels, 1);
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.audio.bits_per_sample, 32);
        assert!(matches!(config.audio.sample_format, SampleFormat::Float));
        assert_eq!(config.model.repo, "ggerganov/whisper.cpp");
        assert_eq!(config.model.filename, "ggml-base.en.bin");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("channels = 1"));
        assert!(toml.contains("sample_rate = 16000"));
        assert!(toml.contains("bits_per_sample = 32"));
        assert!(toml.contains("sample_format = \"float\""));
        assert!(toml.contains("repo = \"ggerganov/whisper.cpp\""));
        assert!(toml.contains("filename = \"ggml-base.en.bin\""));
    }

    #[test]
    fn test_config_deserialization() {
        let toml = r#"
            cache_dir = "/tmp/test"
            recording_path = "/tmp/test/recorded.wav"

            [audio]
            channels = 2
            sample_rate = 48000
            bits_per_sample = 16
            sample_format = "int"

            [model]
            repo = "test/repo"
            filename = "test.bin"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.audio.channels, 2);
        assert_eq!(config.audio.sample_rate, 48000);
        assert_eq!(config.audio.bits_per_sample, 16);
        assert!(matches!(config.audio.sample_format, SampleFormat::Int));
        assert_eq!(config.model.repo, "test/repo");
        assert_eq!(config.model.filename, "test.bin");
        assert_eq!(config.cache_dir, PathBuf::from("/tmp/test"));
        assert_eq!(
            config.recording_path,
            PathBuf::from("/tmp/test/recorded.wav")
        );
    }

    #[test]
    fn test_config_file_io() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Create a test config
        let mut config = Config::default();
        config.audio.channels = 2;
        config.audio.sample_rate = 48000;
        config.audio.bits_per_sample = 16;
        config.audio.sample_format = SampleFormat::Int;
        config.model.repo = "test/repo".to_string();
        config.model.filename = "test.bin".to_string();
        config.cache_dir = PathBuf::from("/tmp/test");
        config.recording_path = PathBuf::from("/tmp/test/recorded.wav");

        // Save config to file
        config.save_to_file(&config_path).unwrap();

        // Load config from file
        let loaded_config = Config::from_file(&config_path).unwrap();

        // Verify loaded config matches original
        assert_eq!(loaded_config.audio.channels, config.audio.channels);
        assert_eq!(loaded_config.audio.sample_rate, config.audio.sample_rate);
        assert_eq!(
            loaded_config.audio.bits_per_sample,
            config.audio.bits_per_sample
        );
        assert!(matches!(
            loaded_config.audio.sample_format,
            SampleFormat::Int
        ));
        assert_eq!(loaded_config.model.repo, config.model.repo);
        assert_eq!(loaded_config.model.filename, config.model.filename);
        assert_eq!(loaded_config.cache_dir, config.cache_dir);
        assert_eq!(loaded_config.recording_path, config.recording_path);
    }

    #[test]
    fn test_invalid_config() {
        let toml = r#"
            [audio]
            channels = "invalid"  # Should be a number
            sample_rate = 48000
            bits_per_sample = 16
            sample_format = "int"
        "#;

        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_creation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("whispering").join("config.toml");

        // Load config (should create default config)
        let config = Config::load_from_path(&config_path).unwrap();

        // Verify config was created
        assert!(config_path.exists());

        // Verify default values
        assert_eq!(config.audio.channels, 1);
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.audio.bits_per_sample, 32);
        assert!(matches!(config.audio.sample_format, SampleFormat::Float));
        assert_eq!(config.model.repo, "ggerganov/whisper.cpp");
        assert_eq!(config.model.filename, "ggml-base.en.bin");
    }
}

