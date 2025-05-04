//! Keyboard input simulation functionality.
//!
//! This module provides utilities for simulating keyboard input, including
//! character-to-key mapping and text pasting functionality.

use std::time::Duration;

use anyhow::Result;
use rdev::{EventType, Key, simulate};

/// Simulates typing the given text by generating keyboard events.
///
/// This function takes a string and simulates typing it by generating
/// appropriate key press and release events. It handles both regular
/// characters and special characters that require the shift key.
pub fn paste(output: String) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(output)?;

    #[cfg(target_os = "macos")]
    {
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
        simulate(&EventType::KeyPress(Key::ControlLeft))?;
        simulate(&EventType::KeyPress(Key::ShiftLeft))?;
        simulate(&EventType::KeyPress(Key::KeyV))?;
        simulate(&EventType::KeyRelease(Key::KeyV))?;
        simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
        simulate(&EventType::KeyRelease(Key::ControlLeft))?;
    }
    #[cfg(target_os = "windows")]
    {
        simulate(&EventType::KeyPress(Key::ControlLeft))?;
        simulate(&EventType::KeyPress(Key::KeyV))?;
        simulate(&EventType::KeyRelease(Key::KeyV))?;
        simulate(&EventType::KeyRelease(Key::ControlLeft))?;
    }
    Ok(())
}
