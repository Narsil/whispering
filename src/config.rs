//! Configuration management for the Whispering application.
//!
//! This module provides functionality for loading and managing application
//! configuration, including audio recording settings and model parameters.

use anyhow::{Context, Result};
use log::error;
use notify_rust::Notification;
use rdev::Key;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

/// Audio recording configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
pub struct AudioConfig {
    /// Number of audio channels (1 for mono, 2 for stereo)
    pub channels: u16,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Sample format (F32 or I16)
    pub sample_format: SampleFormat,
    /// Audio input device name (e.g., "sysdefault:CARD=C920")
    /// If not specified, the default device will be used
    pub device: Option<String>,
}

impl From<SampleFormat> for cpal::SampleFormat {
    fn from(value: SampleFormat) -> Self {
        match value {
            SampleFormat::I16 => cpal::SampleFormat::I16,
            SampleFormat::F32 => cpal::SampleFormat::F32,
        }
    }
}

/// Sample format for audio recording.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(rename_all = "lowercase")]
pub enum SampleFormat {
    /// 32-bit floating point samples
    F32,
    /// 16-bit integer samples
    I16,
}

impl SampleFormat {
    pub fn bits_per_sample(&self) -> u16 {
        match self {
            SampleFormat::F32 => 32,
            SampleFormat::I16 => 16,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            channels: 1,
            sample_rate: 16000,
            sample_format: SampleFormat::F32,
            device: None,
        }
    }
}

/// Path configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
pub struct PathConfig {
    /// Cache directory for storing temporary files
    pub cache_dir: PathBuf,
    /// Path to the recorded audio file
    pub recording_path: PathBuf,
}

/// Type of activation for recording control
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    /// Use keyboard shortcuts for activation
    /// Will send on release
    PushToTalk,
    /// Use keyboard shortcuts to start VAD
    /// activated listening.
    /// Press again to stop listening
    #[serde(rename_all = "snake_case")]
    ToggleVad {
        /// Threshold for voice activity detection (0.0 to 1.0)
        #[serde(default = "default_05")]
        threshold: f32,
        /// Minimum duration of silence to stop recording (in seconds)
        #[serde(default = "default_2")]
        silence_duration: f32,
        /// Minimum duration of speech to start recording (in seconds)
        #[serde(default = "default_1")]
        speech_duration: f32,
        /// Amount of audio to keep before voice detection (in seconds)
        #[serde(default = "default_1")]
        pre_buffer_duration: f32,
    },
}

fn default_2() -> f32 {
    2.0
}
fn default_1() -> f32 {
    1.0
}
fn default_05() -> f32 {
    0.5
}

/// Recording activation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
pub struct ActivationConfig {
    /// Type of activation to use
    pub trigger: Trigger,
    /// Displays a notification about the capturing
    pub notify: bool,
    /// Automatically hit enter after sending the text
    pub autosend: bool,
    /// Keys that need to be pressed in sequence
    pub keys: HashSet<Key>,
}

impl Default for ActivationConfig {
    fn default() -> Self {
        Self {
            trigger: Trigger::PushToTalk {},
            notify: true,
            autosend: false,
            keys: HashSet::from([Key::ControlLeft, Key::Space]),
        }
    }
}

/// Main application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Audio recording settings
    pub audio: AudioConfig,
    /// Path configuration
    pub paths: PathConfig,
    /// Model configuration
    pub model: ModelConfig,
    /// Recording activation configuration
    pub activation: ActivationConfig,
}

/// Type of prompt to use for the model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptType {
    /// Use a list of vocabulary words joined by commas
    Vocabulary { vocabulary: Vec<String> },
    /// Use a custom initial prompt
    Raw { prompt: String },
    /// No prompt
    None,
}

impl Default for PromptType {
    fn default() -> Self {
        Self::None
    }
}

/// Whisper model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    /// Model repository on Hugging Face
    pub repo: String,
    /// Model filename
    pub filename: String,
    /// Type of prompt to use for the model
    pub prompt: PromptType,
    /// Map of text to replace with their replacements
    pub replacements: HashMap<String, String>,
}

impl PromptType {
    /// Returns true if this is the None variant
    /// Gets the prompt text to use with the model
    pub fn get_prompt_text(&self) -> Option<String> {
        match self {
            PromptType::Vocabulary { vocabulary } if !vocabulary.is_empty() => {
                Some(vocabulary.join(", "))
            }
            PromptType::Raw { prompt } => Some(prompt.clone()),
            _ => None,
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            repo: "ggerganov/whisper.cpp".to_string(),
            filename: "ggml-base.en.bin".to_string(),
            prompt: PromptType::None,
            replacements: HashMap::new(),
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
            paths: PathConfig {
                cache_dir,
                recording_path,
            },
            model: ModelConfig::default(),
            activation: ActivationConfig::default(),
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
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())?;
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
    pub fn load_or_write_default(path: Option<&Path>) -> Result<Self> {
        let default_path = Self::default_config_path();
        let path = path.unwrap_or(&default_path);
        // If config exists, use it
        if path.exists() {
            return Self::from_file(path)
                .context(format!("Reading default config from {}", path.display()));
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

    pub fn notify(&self, summary: &str, content: &str) {
        // Show desktop notification
        if self.activation.notify {
            if let Err(err) = Notification::new()
                .summary(summary)
                .body(content)
                .icon("audio-input-microphone")
                .show()
            {
                error!("Cannot show notification: {err} , content was : {summary} {content}")
            };
        }
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
        assert_eq!(config.audio.sample_format, SampleFormat::F32);
        assert_eq!(config.model.repo, "ggerganov/whisper.cpp");
        assert_eq!(config.model.filename, "ggml-base.en.bin");
        assert_eq!(config.model.prompt, PromptType::None);
        assert!(config.model.replacements.is_empty());
        assert_eq!(
            config.activation.keys,
            HashSet::from([Key::ControlLeft, Key::Space])
        );
        assert_eq!(config.activation.trigger, Trigger::PushToTalk);
    }

    #[test]
    fn test_config_serialization() -> Result<()> {
        let config = Config::default();
        let toml = toml::to_string(&config)?;
        println!("TOML output:\n{}", toml);
        assert!(toml.contains("sample_format = \"f32\""));
        assert!(toml.contains("channels = 1"));
        assert!(toml.contains("sample_rate = 16000"));
        Ok(())
    }

    #[test]
    fn test_config_deserialization() -> Result<()> {
        let toml = r#"
            [audio]
            channels = 2
            sample_rate = 48000
            sample_format = "i16"

            [model]
            repo = "test/repo"
            filename = "test.bin"
            prompt = { type = "vocabulary", vocabulary = ["test", "words"] }
            replacements = { "incorrect" = "correct", "wrong" = "right" }

            [paths]
            cache_dir = "/tmp/test"
            recording_path = "/tmp/test/recorded.wav"

            [activation]
            trigger.type = "push_to_talk"
            notify = true
            autosend = true 
            keys = ["ControlLeft", "Space"] 

        "#;

        let config: Config = toml::from_str(toml)?;
        assert_eq!(config.audio.channels, 2);
        assert_eq!(config.audio.sample_rate, 48000);
        assert_eq!(config.audio.sample_format, SampleFormat::I16);
        assert_eq!(config.model.repo, "test/repo");
        assert_eq!(config.model.filename, "test.bin");
        assert_eq!(config.paths.cache_dir, PathBuf::from("/tmp/test"));
        assert_eq!(
            config.paths.recording_path,
            PathBuf::from("/tmp/test/recorded.wav")
        );
        assert_eq!(
            config.model.prompt.get_prompt_text(),
            Some("test, words".to_string())
        );
        assert_eq!(
            config.model.prompt,
            PromptType::Vocabulary {
                vocabulary: vec!["test".to_string(), "words".to_string()]
            }
        );
        assert_eq!(
            config.model.replacements.get("incorrect"),
            Some(&"correct".to_string())
        );
        assert_eq!(
            config.model.replacements.get("wrong"),
            Some(&"right".to_string())
        );
        assert_eq!(
            config.activation.keys,
            HashSet::from([Key::ControlLeft, Key::Space]),
        );
        assert_eq!(config.activation.trigger, Trigger::PushToTalk);
        Ok(())
    }

    #[test]
    fn test_vad_config() -> Result<()> {
        let toml = r#"
            [audio]
            channels = 1
            sample_rate = 16000
            sample_format = "f32"

            [model]
            repo = "ggerganov/whisper.cpp"
            filename = "ggml-base.en.bin"
            prompt = { type = "none" }
            replacements = {}

            [paths]
            cache_dir = "~/.cache/whispering"
            recording_path = "~/.cache/whispering/recorded.wav"

            [activation]
            trigger = { type = "toggle_vad", threshold = 0.7, silence_duration = 1.5, speech_duration = 0.4, pre_buffer_duration = 0.3 }
            keys = ["ControlLeft", "Space"]
            notify = true
            autosend = true
        "#;

        let config: Config = toml::from_str(toml)?;
        assert_eq!(
            config.activation.trigger,
            Trigger::ToggleVad {
                threshold: 0.7,
                silence_duration: 1.5,
                speech_duration: 0.4,
                pre_buffer_duration: 0.3
            }
        );
        Ok(())
    }

    #[test]
    fn test_prompt_type() {
        // Test Vocabulary variant
        let prompt = PromptType::Vocabulary {
            vocabulary: vec!["word1".to_string(), "word2".to_string()],
        };
        assert_eq!(prompt.get_prompt_text(), Some("word1, word2".to_string()));

        // Test Custom variant
        let prompt = PromptType::Raw {
            prompt: "custom prompt".to_string(),
        };
        assert_eq!(prompt.get_prompt_text(), Some("custom prompt".to_string()));

        // Test None variant
        let prompt = PromptType::None;
        assert_eq!(prompt.get_prompt_text(), None);

        // Test empty Vocabulary
        let prompt = PromptType::Vocabulary { vocabulary: vec![] };
        assert_eq!(prompt.get_prompt_text(), None);
    }

    #[test]
    fn test_mutually_exclusive_options() {
        let toml = r#"
            [audio]
            channels = 1
            sample_rate = 16000
            sample_format = "f32"

            [model]
            repo = "test/repo"
            filename = "test.bin"
            prompt = { type = "Vocabulary", value = ["test", "words"] }
        "#;

        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_example_config() -> Result<()> {
        let minimal_config = r#"
            [audio]
            channels = 1
            sample_rate = 16000
            sample_format = "f32"

            [model]
            repo = "ggerganov/whisper.cpp"
            filename = "ggml-base.en.bin"
            prompt = {type ="none"}
            replacements = {}

            [paths]
            cache_dir = "~/.cache/whispering"
            recording_path = "~/.cache/whispering/recorded.wav"

            [activation]
            trigger.type = "push_to_talk"
            notify = true
            autosend = true 
            keys = ["ControlLeft", "Space"] 
        "#;

        let config: Config = toml::from_str(minimal_config)?;

        // Verify audio settings
        assert_eq!(config.audio.channels, 1);
        assert_eq!(config.audio.sample_rate, 16000);
        assert!(matches!(config.audio.sample_format, SampleFormat::F32));

        // Verify model settings
        assert_eq!(config.model.repo, "ggerganov/whisper.cpp");
        assert_eq!(config.model.filename, "ggml-base.en.bin");
        assert!(matches!(config.model.prompt, PromptType::None));

        // Verify paths
        assert_eq!(config.paths.cache_dir, PathBuf::from("~/.cache/whispering"));
        assert_eq!(
            config.paths.recording_path,
            PathBuf::from("~/.cache/whispering/recorded.wav")
        );

        // Verify activation
        assert_eq!(
            config.activation.keys,
            HashSet::from([Key::ControlLeft, Key::Space])
        );
        assert_eq!(config.activation.trigger, Trigger::PushToTalk);
        Ok(())
    }

    #[test]
    fn test_config_file_io() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        // Create a test config
        let mut config = Config::default();
        config.audio.channels = 2;
        config.audio.sample_rate = 48000;
        config.audio.sample_format = SampleFormat::I16;
        config.model.repo = "test/repo".to_string();
        config.model.filename = "test.bin".to_string();
        config.model.prompt = PromptType::Vocabulary {
            vocabulary: vec!["test".to_string(), "words".to_string()],
        };
        config.paths.cache_dir = PathBuf::from("/tmp/test");
        config.paths.recording_path = PathBuf::from("/tmp/test/recorded.wav");
        config.activation.trigger = Trigger::PushToTalk;
        config.activation.keys = HashSet::from([Key::ControlLeft, Key::Alt, Key::Space]);

        // Save config to file
        config.save_to_file(&config_path)?;

        // Load config from file
        let loaded_config = Config::from_file(&config_path)?;

        // Verify loaded config matches original
        assert_eq!(loaded_config.audio.channels, config.audio.channels);
        assert_eq!(loaded_config.audio.sample_rate, config.audio.sample_rate);
        assert!(matches!(
            loaded_config.audio.sample_format,
            SampleFormat::I16
        ));
        assert_eq!(loaded_config.model.repo, config.model.repo);
        assert_eq!(loaded_config.model.filename, config.model.filename);
        assert_eq!(loaded_config.model.prompt, config.model.prompt);
        assert_eq!(loaded_config.paths.cache_dir, config.paths.cache_dir);
        assert_eq!(
            loaded_config.paths.recording_path,
            config.paths.recording_path
        );
        assert_eq!(loaded_config.activation.trigger, config.activation.trigger);
        Ok(())
    }

    #[test]
    fn test_example_default_config_round_trip() -> Result<()> {
        // Verify that the default config matches the original
        let default = Config::default();
        let serialized = toml::to_string(&default)?;

        // Deserialize the serialized config
        let deserialized: Config = toml::from_str(&serialized)?;

        // Deserialize the serialized config
        let mut example: Config = toml::from_str(&std::fs::read_to_string("config.example.toml")?)?;
        example.paths.cache_dir = default.paths.cache_dir.clone();
        example.paths.recording_path = default.paths.recording_path.clone();

        // Verify that the deserialized config matches the original
        assert_eq!(default, deserialized);
        assert_eq!(default, example, "{default:#?} != {example:#?}");

        Ok(())
    }

    #[test]
    fn test_invalid_config() {
        let toml = r#"
            [audio]
            channels = "invalid"  # Should be a number
            sample_rate = 48000
            sample_format = "i16"
        "#;

        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_creation() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("whispering").join("config.toml");

        // Load config (should create default config)
        let config = Config::load_or_write_default(Some(&config_path))?;

        // Verify config was created
        assert!(config_path.exists());

        // Verify default values
        assert_eq!(config.audio.channels, 1);
        assert_eq!(config.audio.sample_rate, 16000);
        assert!(matches!(config.audio.sample_format, SampleFormat::F32));
        assert_eq!(config.model.repo, "ggerganov/whisper.cpp");
        assert_eq!(config.model.filename, "ggml-base.en.bin");
        Ok(())
    }
}
