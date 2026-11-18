//! Shortcut identity: which physical key (numpad aware) plus modifiers.
//!
//! Keys are matched by **scan code**, so a bound numpad key works with NumLock
//! on or off (`Numpad1` and `End` are the same key on a numpad).

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    Digit(u8),
    Divide,
    Multiply,
    Subtract,
    Add,
    Dot,
    Enter,
    /// Any other physical key (scan code + extended flag). Reserved for the
    /// "bind any key" feature; parsed as `Key(scan)` for now.
    Other(u16, bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Shortcut {
    pub key: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

impl Shortcut {
    pub fn new(key: KeyCode) -> Self {
        Self {
            key,
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
        }
    }

    #[allow(dead_code)] // convenient for the shortcuts editor landing in a later phase
    pub fn with_modifiers(mut self, ctrl: bool, alt: bool, shift: bool, win: bool) -> Self {
        self.ctrl = ctrl;
        self.alt = alt;
        self.shift = shift;
        self.win = win;
        self
    }

    /// `1..9` → `Numpad1..Numpad9`, `10` → `Numpad/`, `11` → `Numpad*`.
    pub fn from_legacy_key(key: i64) -> Option<Self> {
        let code = match key {
            1..=9 => KeyCode::Digit(key as u8),
            10 => KeyCode::Divide,
            11 => KeyCode::Multiply,
            _ => return None,
        };
        Some(Self::new(code))
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        let text = input.trim();
        if text.is_empty() {
            return Err("Empty shortcut".to_string());
        }

        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut win = false;
        let mut key_part: Option<&str> = None;

        for part in text.split('+').map(str::trim).filter(|p| !p.is_empty()) {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" | "menu" => alt = true,
                "shift" => shift = true,
                "win" | "meta" | "super" => win = true,
                _ => {
                    if key_part.is_some() {
                        return Err(format!("\"{}\" has more than one key", input));
                    }
                    key_part = Some(part);
                }
            }
        }

        let key_part = key_part.ok_or_else(|| format!("\"{}\" has no key", input))?;
        let key = KeyCode::parse(key_part)?;

        Ok(Self {
            key,
            ctrl,
            alt,
            shift,
            win,
        })
    }

    /// Maps a low-level keyboard hook event to a shortcut identity.
    #[allow(dead_code)] // retained for diagnostics and future calibration UI
    pub fn from_scan(scan: u16, extended: bool, ctrl: bool, alt: bool, shift: bool, win: bool) -> Self {
        Self {
            key: KeyCode::from_scan(scan, extended),
            ctrl,
            alt,
            shift,
            win,
        }
    }
}

impl KeyCode {
    pub fn parse(input: &str) -> Result<Self, String> {
        let lowered = input.trim().to_ascii_lowercase();
        let name = lowered.strip_prefix("numpad").unwrap_or(&lowered);

        let digit = match name {
            "0" | "ins" | "insert" => Some(0),
            "1" | "end" => Some(1),
            "2" | "down" => Some(2),
            "3" | "pgdn" | "pagedown" => Some(3),
            "4" | "left" => Some(4),
            "5" | "clear" => Some(5),
            "6" | "right" => Some(6),
            "7" | "home" => Some(7),
            "8" | "up" => Some(8),
            "9" | "pgup" | "pageup" => Some(9),
            _ => None,
        };

        if let Some(digit) = digit {
            if lowered.starts_with("numpad") || matches!(name, "end" | "down" | "pgdn" | "left" | "clear" | "right" | "home" | "up" | "pgup" | "ins") {
                return Ok(KeyCode::Digit(digit));
            }
        }

        let code = match name {
            "/" | "div" | "divide" | "slash" => KeyCode::Divide,
            "*" | "mult" | "multiply" => KeyCode::Multiply,
            "-" | "sub" | "subtract" => KeyCode::Subtract,
            "+" | "add" => KeyCode::Add,
            "." | "dot" | "decimal" | "del" => KeyCode::Dot,
            "enter" | "return" => KeyCode::Enter,
            other => {
                if let Some(scan) = other.strip_prefix("key") {
                    if let Ok(scan) = scan.parse::<u16>() {
                        return Ok(KeyCode::Other(scan, false));
                    }
                }
                return Err(format!("Unknown key \"{}\"", input));
            }
        };
        Ok(code)
    }

    #[allow(dead_code)] // retained for diagnostics and future calibration UI
    pub fn from_scan(scan: u16, extended: bool) -> Self {
        match (scan, extended) {
            (0x52, _) => KeyCode::Digit(0),
            (0x4F, _) => KeyCode::Digit(1),
            (0x50, _) => KeyCode::Digit(2),
            (0x51, _) => KeyCode::Digit(3),
            (0x4B, _) => KeyCode::Digit(4),
            (0x4C, _) => KeyCode::Digit(5),
            (0x4D, _) => KeyCode::Digit(6),
            (0x47, _) => KeyCode::Digit(7),
            (0x48, _) => KeyCode::Digit(8),
            (0x49, _) => KeyCode::Digit(9),
            (0x35, true) => KeyCode::Divide,
            (0x37, _) => KeyCode::Multiply,
            (0x4A, _) => KeyCode::Subtract,
            (0x4E, _) => KeyCode::Add,
            (0x53, _) => KeyCode::Dot,
            (0x1C, true) => KeyCode::Enter,
            _ => KeyCode::Other(scan, extended),
        }
    }

    #[allow(dead_code)] // used by the keyboard tests and by future SendInput mapping
    pub fn scan(&self) -> Option<(u16, bool)> {
        Some(match self {
            KeyCode::Digit(0) => (0x52, false),
            KeyCode::Digit(d @ 1..=9) => {
                let scans = [0x4F, 0x50, 0x51, 0x4B, 0x4C, 0x4D, 0x47, 0x48, 0x49];
                (scans[(d - 1) as usize], false)
            }
            KeyCode::Digit(_) => return None,
            KeyCode::Divide => (0x35, true),
            KeyCode::Multiply => (0x37, false),
            KeyCode::Subtract => (0x4A, false),
            KeyCode::Add => (0x4E, false),
            KeyCode::Dot => (0x53, false),
            KeyCode::Enter => (0x1C, true),
            KeyCode::Other(scan, extended) => (*scan, *extended),
        })
    }

    pub fn label(&self) -> String {
        match self {
            KeyCode::Digit(d) => format!("Numpad{}", d),
            KeyCode::Divide => "Numpad/".to_string(),
            KeyCode::Multiply => "Numpad*".to_string(),
            KeyCode::Subtract => "Numpad-".to_string(),
            KeyCode::Add => "Numpad+".to_string(),
            KeyCode::Dot => "Numpad.".to_string(),
            KeyCode::Enter => "NumpadEnter".to_string(),
            KeyCode::Other(scan, _) => format!("Key{}", scan),
        }
    }
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.win {
            parts.push("Win".to_string());
        }
        parts.push(self.key.label());
        write!(f, "{}", parts.join("+"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_keys_map_to_numpad() {
        assert_eq!(
            Shortcut::from_legacy_key(1).unwrap().to_string(),
            "Numpad1"
        );
        assert_eq!(
            Shortcut::from_legacy_key(10).unwrap().to_string(),
            "Numpad/"
        );
        assert_eq!(
            Shortcut::from_legacy_key(11).unwrap().to_string(),
            "Numpad*"
        );
        assert!(Shortcut::from_legacy_key(12).is_none());
    }

    #[test]
    fn parses_modifier_combinations() {
        let shortcut = Shortcut::parse("Ctrl+Shift+Numpad4").unwrap();
        assert!(shortcut.ctrl && shortcut.shift && !shortcut.alt && !shortcut.win);
        assert_eq!(shortcut.key, KeyCode::Digit(4));
        assert_eq!(shortcut.to_string(), "Ctrl+Shift+Numpad4");

        let parse_round_trip = Shortcut::parse(&shortcut.to_string()).unwrap();
        assert_eq!(shortcut, parse_round_trip);
    }

    #[test]
    fn parsing_is_order_independent_and_case_insensitive() {
        let a = Shortcut::parse("alt+numpad1").unwrap();
        let b = Shortcut::parse("Numpad1+Alt").unwrap();
        assert_eq!(a, b);
        assert!(a.alt);
    }

    #[test]
    fn rejects_invalid_shortcuts() {
        assert!(Shortcut::parse("").is_err());
        assert!(Shortcut::parse("Ctrl").is_err());
        assert!(Shortcut::parse("Ctrl+Numpad1+Numpad2").is_err());
        assert!(Shortcut::parse("Ctrl+Banana").is_err());
    }

    #[test]
    fn numpad_keys_survive_numlock_off() {
        // With NumLock off the same physical numpad key reports Home/End VKs but
        // the same scan code, so the shortcut identity must not change.
        assert_eq!(KeyCode::from_scan(0x4F, false), KeyCode::Digit(1));
        assert_eq!(KeyCode::from_scan(0x49, false), KeyCode::Digit(9));
        assert_eq!(KeyCode::from_scan(0x35, true), KeyCode::Divide);
        assert_eq!(KeyCode::from_scan(0x35, false), KeyCode::Other(0x35, false));
        assert_eq!(KeyCode::from_scan(0x37, false), KeyCode::Multiply);
    }

    #[test]
    fn scan_codes_round_trip() {
        for digit in 0..=9u8 {
            let key = KeyCode::Digit(digit);
            let (scan, extended) = key.scan().unwrap();
            assert_eq!(KeyCode::from_scan(scan, extended), key);
        }
    }
}
