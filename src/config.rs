//! Configuration management for the Whispering application.
//!
//! This module provides functionality for loading and managing application
//! configuration, including audio recording settings and model parameters.

use anyhow::{Context, Result};
use rdev::Key;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

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

/// Path configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    /// Cache directory for storing temporary files
    pub cache_dir: PathBuf,
    /// Path to the recorded audio file
    pub recording_path: PathBuf,
}

/// Main application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Audio recording settings
    pub audio: AudioConfig,
    /// Path configuration
    pub paths: PathConfig,
    /// Model configuration
    pub model: ModelConfig,
    /// Keyboard shortcut configuration
    pub shortcuts: ShortcutConfig,
}

/// Whisper model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model repository on Hugging Face
    pub repo: String,
    /// Model filename
    pub filename: String,
}

/// Keyboard shortcut configuration.
#[derive(Debug, Clone)]
pub struct ShortcutConfig {
    /// Keys that need to be pressed in sequence
    pub keys: HashSet<Key>,
}

impl Serialize for ShortcutConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct ShortcutConfigHelper {
            keys: Vec<String>,
        }

        let helper = ShortcutConfigHelper {
            keys: self.keys.iter().map(key_to_string).collect(),
        };

        helper.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ShortcutConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ShortcutConfigHelper {
            keys: Vec<String>,
        }

        let helper = ShortcutConfigHelper::deserialize(deserializer)?;

        let keys = helper
            .keys
            .iter()
            .map(|k| {
                string_to_key(k)
                    .ok_or_else(|| serde::de::Error::custom(format!("Invalid key: {}", k)))
            })
            .collect::<Result<Vec<Key>, _>>()?;

        let hash_keys: HashSet<Key> = keys.iter().cloned().collect();
        if hash_keys.len() != keys.len() {
            return Err(serde::de::Error::custom(format!(
                "Duplicate keys in {keys:?}"
            )));
        }
        let keys = hash_keys;

        Ok(ShortcutConfig { keys })
    }
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            keys: HashSet::from([Key::ControlLeft, Key::Space]),
        }
    }
}

/// Converts a Key to a string representation
fn key_to_string(key: &Key) -> String {
    match key {
        // Modifier keys
        Key::ControlLeft | Key::ControlRight => "control".to_string(),
        Key::Alt | Key::AltGr => "alt".to_string(),
        Key::ShiftLeft | Key::ShiftRight => "shift".to_string(),
        Key::MetaLeft | Key::MetaRight => "super".to_string(),

        // Function keys
        Key::F1 => "f1".to_string(),
        Key::F2 => "f2".to_string(),
        Key::F3 => "f3".to_string(),
        Key::F4 => "f4".to_string(),
        Key::F5 => "f5".to_string(),
        Key::F6 => "f6".to_string(),
        Key::F7 => "f7".to_string(),
        Key::F8 => "f8".to_string(),
        Key::F9 => "f9".to_string(),
        Key::F10 => "f10".to_string(),
        Key::F11 => "f11".to_string(),
        Key::F12 => "f12".to_string(),

        // Navigation keys
        Key::DownArrow => "down".to_string(),
        Key::UpArrow => "up".to_string(),
        Key::LeftArrow => "left".to_string(),
        Key::RightArrow => "right".to_string(),
        Key::PageUp => "pageup".to_string(),
        Key::PageDown => "pagedown".to_string(),
        Key::Home => "home".to_string(),
        Key::End => "end".to_string(),

        // Special keys
        Key::Space => "space".to_string(),
        Key::Return => "enter".to_string(),
        Key::Tab => "tab".to_string(),
        Key::Escape => "escape".to_string(),
        Key::Backspace => "backspace".to_string(),
        Key::Delete => "delete".to_string(),
        Key::Insert => "insert".to_string(),
        Key::CapsLock => "capslock".to_string(),
        Key::ScrollLock => "scrolllock".to_string(),
        Key::NumLock => "numlock".to_string(),
        Key::PrintScreen => "printscreen".to_string(),
        Key::Pause => "pause".to_string(),

        // Letter keys
        Key::KeyA => "a".to_string(),
        Key::KeyB => "b".to_string(),
        Key::KeyC => "c".to_string(),
        Key::KeyD => "d".to_string(),
        Key::KeyE => "e".to_string(),
        Key::KeyF => "f".to_string(),
        Key::KeyG => "g".to_string(),
        Key::KeyH => "h".to_string(),
        Key::KeyI => "i".to_string(),
        Key::KeyJ => "j".to_string(),
        Key::KeyK => "k".to_string(),
        Key::KeyL => "l".to_string(),
        Key::KeyM => "m".to_string(),
        Key::KeyN => "n".to_string(),
        Key::KeyO => "o".to_string(),
        Key::KeyP => "p".to_string(),
        Key::KeyQ => "q".to_string(),
        Key::KeyR => "r".to_string(),
        Key::KeyS => "s".to_string(),
        Key::KeyT => "t".to_string(),
        Key::KeyU => "u".to_string(),
        Key::KeyV => "v".to_string(),
        Key::KeyW => "w".to_string(),
        Key::KeyX => "x".to_string(),
        Key::KeyY => "y".to_string(),
        Key::KeyZ => "z".to_string(),

        // Number keys
        Key::Num0 => "0".to_string(),
        Key::Num1 => "1".to_string(),
        Key::Num2 => "2".to_string(),
        Key::Num3 => "3".to_string(),
        Key::Num4 => "4".to_string(),
        Key::Num5 => "5".to_string(),
        Key::Num6 => "6".to_string(),
        Key::Num7 => "7".to_string(),
        Key::Num8 => "8".to_string(),
        Key::Num9 => "9".to_string(),

        // Numpad keys
        Key::Kp0 => "numpad0".to_string(),
        Key::Kp1 => "numpad1".to_string(),
        Key::Kp2 => "numpad2".to_string(),
        Key::Kp3 => "numpad3".to_string(),
        Key::Kp4 => "numpad4".to_string(),
        Key::Kp5 => "numpad5".to_string(),
        Key::Kp6 => "numpad6".to_string(),
        Key::Kp7 => "numpad7".to_string(),
        Key::Kp8 => "numpad8".to_string(),
        Key::Kp9 => "numpad9".to_string(),
        Key::KpDivide => "numpaddivide".to_string(),
        Key::KpMultiply => "numpadmultiply".to_string(),
        Key::KpMinus => "numpadsubtract".to_string(),
        Key::KpPlus => "numpadadd".to_string(),
        Key::KpReturn => "numpadenter".to_string(),
        Key::KpDelete => "numpaddecimal".to_string(),

        // Other keys
        Key::Minus => "minus".to_string(),
        Key::Equal => "equal".to_string(),
        Key::LeftBracket => "leftbracket".to_string(),
        Key::RightBracket => "rightbracket".to_string(),
        Key::BackSlash => "backslash".to_string(),
        Key::SemiColon => "semicolon".to_string(),
        Key::Quote => "quote".to_string(),
        Key::BackQuote => "backquote".to_string(),
        Key::Comma => "comma".to_string(),
        Key::Dot => "dot".to_string(),
        Key::Slash => "slash".to_string(),
        Key::IntlBackslash => "intlbackslash".to_string(),
        Key::Function => "function".to_string(),
        Key::Unknown(_) => "unknown".to_string(),
    }
}

/// Converts a string to a Key
fn string_to_key(s: &str) -> Option<Key> {
    match s.to_lowercase().as_str() {
        // Modifier keys
        "control" => Some(Key::ControlLeft),
        "alt" => Some(Key::Alt),
        "shift" => Some(Key::ShiftLeft),
        "super" => Some(Key::MetaLeft),

        // Function keys
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),

        // Navigation keys
        "down" => Some(Key::DownArrow),
        "up" => Some(Key::UpArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),

        // Special keys
        "space" => Some(Key::Space),
        "enter" => Some(Key::Return),
        "tab" => Some(Key::Tab),
        "escape" => Some(Key::Escape),
        "backspace" => Some(Key::Backspace),
        "delete" => Some(Key::Delete),
        "insert" => Some(Key::Insert),
        "capslock" => Some(Key::CapsLock),
        "scrolllock" => Some(Key::ScrollLock),
        "numlock" => Some(Key::NumLock),
        "printscreen" => Some(Key::PrintScreen),
        "pause" => Some(Key::Pause),

        // Letter keys
        "a" => Some(Key::KeyA),
        "b" => Some(Key::KeyB),
        "c" => Some(Key::KeyC),
        "d" => Some(Key::KeyD),
        "e" => Some(Key::KeyE),
        "f" => Some(Key::KeyF),
        "g" => Some(Key::KeyG),
        "h" => Some(Key::KeyH),
        "i" => Some(Key::KeyI),
        "j" => Some(Key::KeyJ),
        "k" => Some(Key::KeyK),
        "l" => Some(Key::KeyL),
        "m" => Some(Key::KeyM),
        "n" => Some(Key::KeyN),
        "o" => Some(Key::KeyO),
        "p" => Some(Key::KeyP),
        "q" => Some(Key::KeyQ),
        "r" => Some(Key::KeyR),
        "s" => Some(Key::KeyS),
        "t" => Some(Key::KeyT),
        "u" => Some(Key::KeyU),
        "v" => Some(Key::KeyV),
        "w" => Some(Key::KeyW),
        "x" => Some(Key::KeyX),
        "y" => Some(Key::KeyY),
        "z" => Some(Key::KeyZ),

        // Number keys
        "0" => Some(Key::Num0),
        "1" => Some(Key::Num1),
        "2" => Some(Key::Num2),
        "3" => Some(Key::Num3),
        "4" => Some(Key::Num4),
        "5" => Some(Key::Num5),
        "6" => Some(Key::Num6),
        "7" => Some(Key::Num7),
        "8" => Some(Key::Num8),
        "9" => Some(Key::Num9),

        // Numpad keys
        "numpad0" => Some(Key::Kp0),
        "numpad1" => Some(Key::Kp1),
        "numpad2" => Some(Key::Kp2),
        "numpad3" => Some(Key::Kp3),
        "numpad4" => Some(Key::Kp4),
        "numpad5" => Some(Key::Kp5),
        "numpad6" => Some(Key::Kp6),
        "numpad7" => Some(Key::Kp7),
        "numpad8" => Some(Key::Kp8),
        "numpad9" => Some(Key::Kp9),
        "numpaddivide" => Some(Key::KpDivide),
        "numpadmultiply" => Some(Key::KpMultiply),
        "numpadsubtract" => Some(Key::KpMinus),
        "numpadadd" => Some(Key::KpPlus),
        "numpadenter" => Some(Key::KpReturn),
        "numpaddecimal" => Some(Key::KpDelete),

        // Other keys
        "minus" => Some(Key::Minus),
        "equal" => Some(Key::Equal),
        "leftbracket" => Some(Key::LeftBracket),
        "rightbracket" => Some(Key::RightBracket),
        "backslash" => Some(Key::BackSlash),
        "semicolon" => Some(Key::SemiColon),
        "quote" => Some(Key::Quote),
        "backquote" => Some(Key::BackQuote),
        "comma" => Some(Key::Comma),
        "dot" => Some(Key::Dot),
        "slash" => Some(Key::Slash),
        "intlbackslash" => Some(Key::IntlBackslash),
        "function" => Some(Key::Function),
        "unknown" => Some(Key::Unknown(0)),

        // Unknown key
        _ => None,
    }
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
            paths: PathConfig {
                cache_dir,
                recording_path,
            },
            model: ModelConfig::default(),
            shortcuts: ShortcutConfig::default(),
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
            [audio]
            channels = 2
            sample_rate = 48000
            bits_per_sample = 16
            sample_format = "int"

            [model]
            repo = "test/repo"
            filename = "test.bin"

            [paths]
            cache_dir = "/tmp/test"
            recording_path = "/tmp/test/recorded.wav"

            [shortcuts]
            keys = ["control", "space"]
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.audio.channels, 2);
        assert_eq!(config.audio.sample_rate, 48000);
        assert_eq!(config.audio.bits_per_sample, 16);
        assert!(matches!(config.audio.sample_format, SampleFormat::Int));
        assert_eq!(config.model.repo, "test/repo");
        assert_eq!(config.model.filename, "test.bin");
        assert_eq!(config.paths.cache_dir, PathBuf::from("/tmp/test"));
        assert_eq!(
            config.paths.recording_path,
            PathBuf::from("/tmp/test/recorded.wav")
        );
        assert_eq!(
            config.shortcuts.keys,
            HashSet::from([Key::ControlLeft, Key::Space])
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
        config.paths.cache_dir = PathBuf::from("/tmp/test");
        config.paths.recording_path = PathBuf::from("/tmp/test/recorded.wav");
        config.shortcuts.keys = HashSet::from([Key::ControlLeft, Key::Alt, Key::Space]);

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
        assert_eq!(loaded_config.paths.cache_dir, config.paths.cache_dir);
        assert_eq!(
            loaded_config.paths.recording_path,
            config.paths.recording_path
        );
        assert_eq!(loaded_config.shortcuts.keys, config.shortcuts.keys);
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

    #[test]
    fn test_example_config() {
        let minimal_config = r#"
            [audio]
            channels = 1
            sample_rate = 16000
            bits_per_sample = 32
            sample_format = "float"

            [model]
            repo = "ggerganov/whisper.cpp"
            filename = "ggml-base.en.bin"

            [paths]
            cache_dir = "~/.cache/whispering"
            recording_path = "~/.cache/whispering/recorded.wav"

            [shortcuts]
            keys = ["control", "space"]
        "#;

        let config: Config = toml::from_str(minimal_config).unwrap();

        // Verify audio settings
        assert_eq!(config.audio.channels, 1);
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.audio.bits_per_sample, 32);
        assert!(matches!(config.audio.sample_format, SampleFormat::Float));

        // Verify model settings
        assert_eq!(config.model.repo, "ggerganov/whisper.cpp");
        assert_eq!(config.model.filename, "ggml-base.en.bin");

        // Verify paths
        assert_eq!(config.paths.cache_dir, PathBuf::from("~/.cache/whispering"));
        assert_eq!(
            config.paths.recording_path,
            PathBuf::from("~/.cache/whispering/recorded.wav")
        );

        // Verify shortcuts
        assert_eq!(
            config.shortcuts.keys,
            HashSet::from([Key::ControlLeft, Key::Space])
        );
    }

    #[test]
    fn test_example_config_serialization() -> Result<()> {
        let minimal_config = std::fs::read_to_string("config.toml.example")?;

        let config: Config = toml::from_str(&minimal_config).unwrap();

        // Serialize the config back to TOML
        let serialized = toml::to_string(&config).unwrap();

        // Deserialize the serialized config
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        // Verify that the deserialized config matches the original
        assert_eq!(deserialized.audio.channels, config.audio.channels);
        assert_eq!(deserialized.audio.sample_rate, config.audio.sample_rate);
        assert_eq!(
            deserialized.audio.bits_per_sample,
            config.audio.bits_per_sample
        );
        assert!(matches!(
            deserialized.audio.sample_format,
            SampleFormat::Float
        ));
        assert_eq!(deserialized.model.repo, config.model.repo);
        assert_eq!(deserialized.model.filename, config.model.filename);
        assert_eq!(deserialized.paths.cache_dir, config.paths.cache_dir);
        assert_eq!(
            deserialized.paths.recording_path,
            config.paths.recording_path
        );
        assert_eq!(deserialized.shortcuts.keys, config.shortcuts.keys);
        Ok(())
    }
}
