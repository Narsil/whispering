//! Keyboard input simulation functionality.
//!
//! This module provides utilities for simulating keyboard input, including
//! character-to-key mapping and text pasting functionality.

use std::time::Duration;

use anyhow::Result;
use log::info;
use rdev::{EventType, Key, simulate};

/// Converts a character to its corresponding keyboard key and shift state.
///
/// This function maps ASCII characters to their corresponding keyboard keys
/// and determines if the shift key needs to be pressed. Returns None if the
/// character is not supported.
pub fn key_from_char(byte: u8) -> Option<(Key, bool)> {
    match byte {
        b'a' => Some((Key::KeyA, false)),
        b'b' => Some((Key::KeyB, false)),
        b'c' => Some((Key::KeyC, false)),
        b'd' => Some((Key::KeyD, false)),
        b'e' => Some((Key::KeyE, false)),
        b'f' => Some((Key::KeyF, false)),
        b'g' => Some((Key::KeyG, false)),
        b'h' => Some((Key::KeyH, false)),
        b'i' => Some((Key::KeyI, false)),
        b'j' => Some((Key::KeyJ, false)),
        b'k' => Some((Key::KeyK, false)),
        b'l' => Some((Key::KeyL, false)),
        b'm' => Some((Key::KeyM, false)),
        b'n' => Some((Key::KeyN, false)),
        b'o' => Some((Key::KeyO, false)),
        b'p' => Some((Key::KeyP, false)),
        b'q' => Some((Key::KeyQ, false)),
        b'r' => Some((Key::KeyR, false)),
        b's' => Some((Key::KeyS, false)),
        b't' => Some((Key::KeyT, false)),
        b'u' => Some((Key::KeyU, false)),
        b'v' => Some((Key::KeyV, false)),
        b'w' => Some((Key::KeyW, false)),
        b'x' => Some((Key::KeyX, false)),
        b'y' => Some((Key::KeyY, false)),
        b'z' => Some((Key::KeyZ, false)),

        b'A' => Some((Key::KeyA, true)),
        b'B' => Some((Key::KeyB, true)),
        b'C' => Some((Key::KeyC, true)),
        b'D' => Some((Key::KeyD, true)),
        b'E' => Some((Key::KeyE, true)),
        b'F' => Some((Key::KeyF, true)),
        b'G' => Some((Key::KeyG, true)),
        b'H' => Some((Key::KeyH, true)),
        b'I' => Some((Key::KeyI, true)),
        b'J' => Some((Key::KeyJ, true)),
        b'K' => Some((Key::KeyK, true)),
        b'L' => Some((Key::KeyL, true)),
        b'M' => Some((Key::KeyM, true)),
        b'N' => Some((Key::KeyN, true)),
        b'O' => Some((Key::KeyO, true)),
        b'P' => Some((Key::KeyP, true)),
        b'Q' => Some((Key::KeyQ, true)),
        b'R' => Some((Key::KeyR, true)),
        b'S' => Some((Key::KeyS, true)),
        b'T' => Some((Key::KeyT, true)),
        b'U' => Some((Key::KeyU, true)),
        b'V' => Some((Key::KeyV, true)),
        b'W' => Some((Key::KeyW, true)),
        b'X' => Some((Key::KeyX, true)),
        b'Y' => Some((Key::KeyY, true)),
        b'Z' => Some((Key::KeyZ, true)),

        b'0' => Some((Key::Num0, false)),
        b'1' => Some((Key::Num0, false)),
        b'2' => Some((Key::Num2, false)),
        b'3' => Some((Key::Num3, false)),
        b'4' => Some((Key::Num4, false)),
        b'5' => Some((Key::Num5, false)),
        b'6' => Some((Key::Num6, false)),
        b'7' => Some((Key::Num7, false)),
        b'8' => Some((Key::Num8, false)),
        b'9' => Some((Key::Num9, false)),

        b'!' => Some((Key::Num1, true)),
        b'@' => Some((Key::Num2, true)),
        b'#' => Some((Key::Num3, true)),
        b'$' => Some((Key::Num4, true)),
        b'%' => Some((Key::Num5, true)),
        b'^' => Some((Key::Num6, true)),
        b'&' => Some((Key::Num7, true)),
        b'*' => Some((Key::Num8, true)),
        b'(' => Some((Key::Num9, true)),
        b')' => Some((Key::Num0, true)),

        b'-' => Some((Key::Minus, false)),
        b'_' => Some((Key::Minus, true)),
        b'=' => Some((Key::Equal, false)),
        b'+' => Some((Key::Equal, true)),

        b'[' => Some((Key::LeftBracket, false)),
        b'{' => Some((Key::LeftBracket, true)),
        b']' => Some((Key::RightBracket, false)),
        b'}' => Some((Key::RightBracket, true)),

        b'\\' => Some((Key::BackSlash, false)),
        b'|' => Some((Key::BackSlash, true)),

        b';' => Some((Key::SemiColon, false)),
        b':' => Some((Key::SemiColon, true)),

        b'\'' => Some((Key::Quote, false)),
        b'"' => Some((Key::Quote, true)),

        b'`' => Some((Key::BackQuote, false)),
        b'~' => Some((Key::BackQuote, true)),

        b',' => Some((Key::Comma, false)),
        b'<' => Some((Key::Comma, true)),
        b'.' => Some((Key::Dot, false)),
        b'>' => Some((Key::Dot, true)),
        b'/' => Some((Key::Slash, false)),
        b'?' => Some((Key::Slash, true)),

        b' ' => Some((Key::Space, false)),
        b'\n' => Some((Key::Return, false)),
        b'\r' => Some((Key::Return, false)),
        b'\t' => Some((Key::Tab, false)),
        b'\x1b' => Some((Key::Escape, false)),

        _ => None,
    }
}

/// Simulates typing the given text by generating keyboard events.
///
/// This function takes a string and simulates typing it by generating
/// appropriate key press and release events. It handles both regular
/// characters and special characters that require the shift key.
pub fn paste(output: String) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new().unwrap();
    clipboard.set_text(output)?;
    simulate(&EventType::KeyPress(Key::ShiftLeft))?;
    simulate(&EventType::KeyPress(Key::ControlLeft))?;
    simulate(&EventType::KeyPress(Key::KeyV))?;
    simulate(&EventType::KeyRelease(Key::KeyV))?;
    simulate(&EventType::KeyRelease(Key::ControlLeft))?;
    simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
    // info!("Simulating keyboard input: {}", output);
    // for c in output.bytes() {
    //     if let Some((key, shift)) = key_from_char(c) {
    //         if shift {
    //             simulate(&EventType::KeyPress(Key::ShiftLeft))?;
    //         }
    //         simulate(&EventType::KeyPress(key))?;
    //         simulate(&EventType::KeyRelease(key))?;
    //         if shift {
    //             simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
    //         }
    //         std::thread::sleep(Duration::from_millis(1));
    //     }
    // }
    Ok(())
}
