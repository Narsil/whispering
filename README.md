# Whispering

A desktop application that allows you to record audio and transcribe it using OpenAI's Whisper model. The application runs locally and can be triggered using keyboard shortcuts.

## Features

- Record audio using keyboard shortcuts
- Transcribe audio using Whisper.cpp
- Automatic text pasting after transcription
- Desktop notifications for recording status
- Configurable audio settings and keyboard shortcuts

## Installation

### Manual Installation

1. Install Rust and Cargo from [rustup.rs](https://rustup.rs/)
2. (Optional) Install the required system dependencies:
   - ALSA development libraries (for audio recording)
   - X11 development libraries (for keyboard input)
   - OpenSSL development libraries (for model downloading)

#### MacOS
3. Build and run:
```bash
cargo run --release --features metal
```

#### Linux
3. Build and run:
```bash
cargo run --release --features cuda,x11  # Untested
cargo run --release --features cuda,wayland
```

### Using Nix

```bash
# Clone the repository
git clone https://github.com/yourusername/whispering.git
cd whispering

# Build and run using Nix
nix build
```


## Configuration

The application can be configured using a TOML file located at `~/.config/whispering/config.toml`. If no configuration file exists, the application will create one with default values.

### Example Configuration

```toml
# Whispering Configuration Example
#
# Copy this file to ~/.config/whispering/config.toml and modify the values as needed.
# If no config.toml is found, the application will use default values.

[audio]
# Number of audio channels (1 for mono, 2 for stereo)
channels = 1
# Sample rate in Hz
sample_rate = 16000
# Bits per sample
bits_per_sample = 32
# Sample format (float or int)
sample_format = "float"

[model]
# Hugging Face model repository
repo = "ggerganov/whisper.cpp"
# Model filename
filename = "ggml-base.en.bin"
# Prompt
# Options are `type = "none", "vocabulary", "raw"
#  prompt = { type = "vocabulary", vocabulary = ["Google", "HuggingFace"] } 
#     will insert  "Google, HuggingFace" as an initial prompt.
# 
#  prompt = { type = "raw", prompt = "HuggingFace likes Google"] } 
#     will insert  "HuggingFace like Google" as an initial prompt.
# 
# For more information on whisper prompting:
# https://cookbook.openai.com/examples/whisper_prompting_guide
prompt = { type = "none" }
replacements = {}

[paths]
# Cache directory for storing temporary files
cache_dir = "~/.cache/whispering"
# Path to the recorded audio file
recording_path = "~/.cache/whispering/recorded.wav"

[shortcuts]
# Keys that need to be pressed in sequence to start recording
# Available keys: control, alt, shift, super, space, enter, r, and many others
keys = ["ControlLeft", "Space"] 
# Automatically hit enter after sending the text (so sends a message in usual contexts).
autosend = false
```

### Configuration Options

#### Audio Settings
- `channels`: Number of audio channels (1 for mono, 2 for stereo)
- `sample_rate`: Sample rate in Hz (default: 16000)
- `bits_per_sample`: Bits per sample (default: 32)
- `sample_format`: Sample format ("float" or "int")

#### Model Settings
- `repo`: Hugging Face model repository
- `filename`: Model filename to download and use

#### Path Settings
- `cache_dir`: Directory for storing temporary files
- `recording_path`: Path to save recorded audio files

#### Shortcut Settings
- `keys`: List of keys to press in sequence to start recording
  - Available keys: control, alt, shift, super, space, enter, and many others
  - Default: ["control", "space"]

## Usage

1. Start the application
2. Press the configured shortcut keys (default: Control + Space) to start recording
3. Release the last key to stop recording
4. The transcribed text will be automatically pasted into the active window

## Troubleshooting

### Audio Issues
- Ensure your microphone is properly connected and selected as the default input device
- Check that your system's audio permissions are properly configured
- Verify that the ALSA development libraries are installed

### Keyboard Issues
- Make sure the X11 development libraries are installed
- Check that your window manager supports the keyboard shortcuts you've configured

### Model Issues
- Ensure you have sufficient disk space for the model (approximately 1.5GB)
- Check your internet connection for model downloading
- Verify that the model repository and filename are correct

## License

This project is licensed under the MIT License - see the LICENSE file for details. 
