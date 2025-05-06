//! Keyboard input simulation functionality.
//!
//! This module provides utilities for simulating keyboard input, including
//! character-to-key mapping and text pasting functionality.

use std::time::Duration;

use anyhow::Result;
use log::{debug, info};
use rdev::{EventType, Key, simulate};

/// Simulates typing the given text by generating keyboard events.
///
/// This function takes a string and simulates typing it by generating
/// appropriate key press and release events. It handles both regular
/// characters and special characters that require the shift key.
pub fn paste(output: String) -> Result<()> {
    info!("Simulating keyboard input: {}", output);
    debug!("Getting clipboard");
    #[cfg(target_os = "macos")]
    {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(output)?;
        simulate(&EventType::KeyPress(Key::MetaLeft))?;
        std::thread::sleep(Duration::from_millis(2));
        simulate(&EventType::KeyPress(Key::KeyV))?;
        std::thread::sleep(Duration::from_millis(2));
        simulate(&EventType::KeyRelease(Key::KeyV))?;
        std::thread::sleep(Duration::from_millis(2));
        simulate(&EventType::KeyRelease(Key::MetaLeft))?;
        std::thread::sleep(Duration::from_millis(2));
    }
    #[cfg(target_os = "linux")]
    {
        #[cfg(feature = "wayland")]
        {
            use wl_clipboard_rs::copy::{MimeType, Options, Source};
            let opts = Options::new();
            opts.copy(
                Source::Bytes(output.into_bytes().into()),
                MimeType::Autodetect,
            )?;
        }
        #[cfg(feature = "x11")]
        {
            let mut clipboard = arboard::Clipboard::new()?;
            clipboard.set_text(output)?;
        }
        #[cfg(not(any(feature = "x11", feature = "wayland")))]
        {
            compile_error!("Wayland or x11 must be active");
        }
        debug!("Clipboard set");
        std::thread::sleep(Duration::from_millis(5));
        simulate(&EventType::KeyPress(Key::ControlLeft))?;
        debug!("Event ok");
        simulate(&EventType::KeyPress(Key::ShiftLeft))?;
        simulate(&EventType::KeyPress(Key::KeyV))?;
        simulate(&EventType::KeyRelease(Key::KeyV))?;
        simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
        simulate(&EventType::KeyRelease(Key::ControlLeft))?;
        debug!("Events simulated");
    }
    #[cfg(target_os = "windows")]
    {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(output)?;
        simulate(&EventType::KeyPress(Key::ControlLeft))?;
        simulate(&EventType::KeyPress(Key::KeyV))?;
        simulate(&EventType::KeyRelease(Key::KeyV))?;
        simulate(&EventType::KeyRelease(Key::ControlLeft))?;
    }
    Ok(())
}
