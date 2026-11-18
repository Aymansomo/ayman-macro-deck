//! Stable physical key identities derived from Raw Input scan codes.
//!
//! A `KeyId` describes *which physical key* was pressed, independent of the
//! displayed label, NumLock state or the originating device. Numpad keys and
//! navigation keys share scan codes on many keyboards, so the `E0`/`E1`
//! extended flags are part of the identity.

use serde::{Deserialize, Serialize};

use crate::shortcut::{KeyCode, Shortcut};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyId {
    Numpad(u8),
    NumpadDiv,
    NumpadMul,
    NumpadSub,
    NumpadAdd,
    NumpadDot,
    NumpadEnter,
    NavInsert,
    NavHome,
    NavPageUp,
    NavDelete,
    NavEnd,
    NavPageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Function(u8),
    Letter(char),
    Digit(char),
    Other(u16, bool),
}

impl KeyId {
    /// Maps a Raw Input `MakeCode` plus `E0`/`E1` flags to a physical key.
    pub fn from_raw(scan: u16, extended: bool, vkey: u16) -> Self {
        match (scan, extended) {
            // Navigation cluster and arrows (E0-prefixed versions of the
            // numpad scan codes).
            (0x52, true) => Self::NavInsert,
            (0x47, true) => Self::NavHome,
            (0x49, true) => Self::NavPageUp,
            (0x53, true) => Self::NavDelete,
            (0x4F, true) => Self::NavEnd,
            (0x51, true) => Self::NavPageDown,
            (0x48, true) => Self::ArrowUp,
            (0x50, true) => Self::ArrowDown,
            (0x4B, true) => Self::ArrowLeft,
            (0x4D, true) => Self::ArrowRight,

            // Numpad digits (no E0; works with NumLock on or off).
            (0x52, false) => Self::Numpad(0),
            (0x4F, false) => Self::Numpad(1),
            (0x50, false) => Self::Numpad(2),
            (0x51, false) => Self::Numpad(3),
            (0x4B, false) => Self::Numpad(4),
            (0x4C, false) => Self::Numpad(5),
            (0x4D, false) => Self::Numpad(6),
            (0x47, false) => Self::Numpad(7),
            (0x48, false) => Self::Numpad(8),
            (0x49, false) => Self::Numpad(9),

            (0x35, true) => Self::NumpadDiv,
            (0x37, false) => Self::NumpadMul,
            (0x4A, false) => Self::NumpadSub,
            (0x4E, false) => Self::NumpadAdd,
            (0x53, false) => Self::NumpadDot,
            (0x1C, true) => Self::NumpadEnter,

            (0x3B..=0x44, false) => Self::Function((scan - 0x3A) as u8),
            (0x57, false) => Self::Function(11),
            (0x58, false) => Self::Function(12),

            // Main alphanumeric block.
            (0x1E, false) => Self::Letter('A'),
            (0x30, false) => Self::Letter('B'),
            (0x2E, false) => Self::Letter('C'),
            (0x20, false) => Self::Letter('D'),
            (0x12, false) => Self::Letter('E'),
            (0x21, false) => Self::Letter('F'),
            (0x22, false) => Self::Letter('G'),
            (0x23, false) => Self::Letter('H'),
            (0x17, false) => Self::Letter('I'),
            (0x24, false) => Self::Letter('J'),
            (0x25, false) => Self::Letter('K'),
            (0x26, false) => Self::Letter('L'),
            (0x32, false) => Self::Letter('M'),
            (0x31, false) => Self::Letter('N'),
            (0x18, false) => Self::Letter('O'),
            (0x19, false) => Self::Letter('P'),
            (0x10, false) => Self::Letter('Q'),
            (0x13, false) => Self::Letter('R'),
            (0x1F, false) => Self::Letter('S'),
            (0x14, false) => Self::Letter('T'),
            (0x16, false) => Self::Letter('U'),
            (0x2F, false) => Self::Letter('V'),
            (0x11, false) => Self::Letter('W'),
            (0x2D, false) => Self::Letter('X'),
            (0x15, false) => Self::Letter('Y'),
            (0x2C, false) => Self::Letter('Z'),
            (0x0B, false) => Self::Digit('0'),
            (0x02, false) => Self::Digit('1'),
            (0x03, false) => Self::Digit('2'),
            (0x04, false) => Self::Digit('3'),
            (0x05, false) => Self::Digit('4'),
            (0x06, false) => Self::Digit('5'),
            (0x07, false) => Self::Digit('6'),
            (0x08, false) => Self::Digit('7'),
            (0x09, false) => Self::Digit('8'),
            (0x0A, false) => Self::Digit('9'),

            // Unknown key: keep the raw identity plus the VK as a hint so it
            // can be calibrated later.
            _ => {
                let _ = vkey;
                Self::Other(scan, extended)
            }
        }
    }

    /// Stable string used in config files and events.
    pub fn id(&self) -> String {
        match self {
            Self::Numpad(d) => format!("Numpad{}", d),
            Self::NumpadDiv => "NumpadDivide".into(),
            Self::NumpadMul => "NumpadMultiply".into(),
            Self::NumpadSub => "NumpadSubtract".into(),
            Self::NumpadAdd => "NumpadAdd".into(),
            Self::NumpadDot => "NumpadDecimal".into(),
            Self::NumpadEnter => "NumpadEnter".into(),
            Self::NavInsert => "Insert".into(),
            Self::NavHome => "Home".into(),
            Self::NavPageUp => "PageUp".into(),
            Self::NavDelete => "Delete".into(),
            Self::NavEnd => "End".into(),
            Self::NavPageDown => "PageDown".into(),
            Self::ArrowUp => "ArrowUp".into(),
            Self::ArrowDown => "ArrowDown".into(),
            Self::ArrowLeft => "ArrowLeft".into(),
            Self::ArrowRight => "ArrowRight".into(),
            Self::Function(n) => format!("F{}", n),
            Self::Letter(c) => format!("Key{}", c),
            Self::Digit(c) => format!("Digit{}", c),
            Self::Other(scan, ext) => format!("Scan{:03X}{}", scan, if *ext { "E0" } else { "" }),
        }
    }

    /// Inverse of `id()` so persisted identities can be resolved again.
    pub fn parse(id: &str) -> Option<Self> {
        let text = id.trim();
        let key = match text {
            "NumpadDivide" => Self::NumpadDiv,
            "NumpadMultiply" => Self::NumpadMul,
            "NumpadSubtract" => Self::NumpadSub,
            "NumpadAdd" => Self::NumpadAdd,
            "NumpadDecimal" => Self::NumpadDot,
            "NumpadEnter" => Self::NumpadEnter,
            "Insert" => Self::NavInsert,
            "Home" => Self::NavHome,
            "PageUp" => Self::NavPageUp,
            "Delete" => Self::NavDelete,
            "End" => Self::NavEnd,
            "PageDown" => Self::NavPageDown,
            "ArrowUp" => Self::ArrowUp,
            "ArrowDown" => Self::ArrowDown,
            "ArrowLeft" => Self::ArrowLeft,
            "ArrowRight" => Self::ArrowRight,
            other => {
                if let Some(digit) = other.strip_prefix("Numpad") {
                    let digit: u8 = digit.parse().ok()?;
                    if digit > 9 {
                        return None;
                    }
                    Self::Numpad(digit)
                } else if let Some(number) = other.strip_prefix("F") {
                    Self::Function(number.parse().ok()?)
                } else if let Some(letter) = other.strip_prefix("Key") {
                    let mut chars = letter.chars();
                    let c = chars.next()?;
                    if chars.next().is_some() || !c.is_ascii_alphabetic() {
                        return None;
                    }
                    Self::Letter(c.to_ascii_uppercase())
                } else if let Some(digit) = other.strip_prefix("Digit") {
                    let mut chars = digit.chars();
                    let c = chars.next()?;
                    if chars.next().is_some() || !c.is_ascii_digit() {
                        return None;
                    }
                    Self::Digit(c)
                } else if let Some(raw) = other.strip_prefix("Scan") {
                    let (scan, ext) = raw.strip_suffix("E0").map(|s| (s, true)).unwrap_or((raw, false));
                    Self::Other(u16::from_str_radix(scan, 16).ok()?, ext)
                } else {
                    return None;
                }
            }
        };
        // Reject strings that did not round-trip (e.g. "Numpad42").
        if key.id() != text {
            return None;
        }
        Some(key)
    }

    pub fn label(&self) -> String {
        match self {
            Self::Numpad(d) => d.to_string(),
            Self::NumpadDiv => "/".into(),
            Self::NumpadMul => "*".into(),
            Self::NumpadSub => "-".into(),
            Self::NumpadAdd => "+".into(),
            Self::NumpadDot => ".".into(),
            Self::NumpadEnter => "Enter".into(),
            Self::NavInsert => "Ins".into(),
            Self::NavHome => "Home".into(),
            Self::NavPageUp => "PgUp".into(),
            Self::NavDelete => "Del".into(),
            Self::NavEnd => "End".into(),
            Self::NavPageDown => "PgDn".into(),
            Self::ArrowUp => "↑".into(),
            Self::ArrowDown => "↓".into(),
            Self::ArrowLeft => "←".into(),
            Self::ArrowRight => "→".into(),
            other => other.id(),
        }
    }

    /// True when this physical key satisfies a legacy `trigger` string.
    pub fn matches_shortcut(&self, shortcut: &Shortcut) -> bool {
        let key = match self {
            Self::Numpad(d) => KeyCode::Digit(*d),
            Self::NumpadDiv => KeyCode::Divide,
            Self::NumpadMul => KeyCode::Multiply,
            Self::NumpadSub => KeyCode::Subtract,
            Self::NumpadAdd => KeyCode::Add,
            Self::NumpadDot => KeyCode::Dot,
            Self::NumpadEnter => KeyCode::Enter,
            Self::NavInsert => KeyCode::Other(0x52, true),
            Self::NavHome => KeyCode::Other(0x47, true),
            Self::NavPageUp => KeyCode::Other(0x49, true),
            Self::NavDelete => KeyCode::Other(0x53, true),
            Self::NavEnd => KeyCode::Other(0x4F, true),
            Self::NavPageDown => KeyCode::Other(0x51, true),
            Self::ArrowUp => KeyCode::Other(0x48, true),
            Self::ArrowDown => KeyCode::Other(0x50, true),
            Self::ArrowLeft => KeyCode::Other(0x4B, true),
            Self::ArrowRight => KeyCode::Other(0x4D, true),
            Self::Other(scan, extended) => KeyCode::Other(*scan, *extended),
            Self::Function(_) | Self::Letter(_) | Self::Digit(_) => return false,
        };
        shortcut.key == key && !shortcut.ctrl && !shortcut.alt && !shortcut.shift && !shortcut.win
    }

    /// Legacy `key` slot (1-9, 10 = `/`, 11 = `*`) for configs written by the
    /// old editor. Other keys have no legacy slot.
    pub fn legacy_key(&self) -> Option<i64> {
        match self {
            Self::Numpad(d @ 1..=9) => Some(*d as i64),
            Self::NumpadDiv => Some(10),
            Self::NumpadMul => Some(11),
            _ => None,
        }
    }

    /// Inverse of `legacy_key()` used when migrating old config slots.
    pub fn from_legacy_key(key: i64) -> Option<Self> {
        match key {
            1..=9 => Some(Self::Numpad(key as u8)),
            10 => Some(Self::NumpadDiv),
            11 => Some(Self::NumpadMul),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numpad_digits_ignore_numlock() {
        assert_eq!(KeyId::from_raw(0x4F, false, 0x21), KeyId::Numpad(1));
        assert_eq!(KeyId::from_raw(0x4F, false, 0x01).id(), "Numpad1");
    }

    #[test]
    fn extended_flag_separates_navigation_from_numpad() {
        assert_eq!(KeyId::from_raw(0x47, false, 0x24), KeyId::Numpad(7));
        assert_eq!(KeyId::from_raw(0x47, true, 0x24), KeyId::NavHome);
        assert_eq!(KeyId::from_raw(0x35, true, 0x6F), KeyId::NumpadDiv);
        assert_eq!(KeyId::from_raw(0x35, false, 0xBF), KeyId::Other(0x35, false));
    }

    #[test]
    fn legacy_slots_match_old_editor() {
        assert_eq!(KeyId::Numpad(1).legacy_key(), Some(1));
        assert_eq!(KeyId::NumpadDiv.legacy_key(), Some(10));
        assert_eq!(KeyId::NumpadMul.legacy_key(), Some(11));
        assert_eq!(KeyId::NavHome.legacy_key(), None);
        assert_eq!(KeyId::from_legacy_key(1), Some(KeyId::Numpad(1)));
        assert_eq!(KeyId::from_legacy_key(10), Some(KeyId::NumpadDiv));
        assert_eq!(KeyId::from_legacy_key(11), Some(KeyId::NumpadMul));
        assert_eq!(KeyId::from_legacy_key(42), None);
    }

    #[test]
    fn identities_round_trip() {
        for key in [
            KeyId::Numpad(1),
            KeyId::NumpadDiv,
            KeyId::NavHome,
            KeyId::ArrowUp,
            KeyId::Function(12),
            KeyId::Letter('A'),
            KeyId::Other(0x35, false),
        ] {
            assert_eq!(KeyId::parse(&key.id()), Some(key));
        }
        assert_eq!(KeyId::parse("Numpad42"), None);
    }

    #[test]
    fn main_number_row_does_not_match_numpad_trigger() {
        let trigger = Shortcut::parse("Numpad1").unwrap();
        assert!(KeyId::Numpad(1).matches_shortcut(&trigger));
        assert!(!KeyId::Digit('1').matches_shortcut(&trigger));
    }
}
