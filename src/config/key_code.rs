use serde::{Deserialize, Serialize};

/// Standard USB HID Keyboard Keycode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct KeyCode(pub u8);

impl KeyCode {
    pub const NONE: Self = Self(0x00);

    // Letters
    pub const A: Self = Self(0x04);
    pub const B: Self = Self(0x05);
    pub const C: Self = Self(0x06);
    pub const D: Self = Self(0x07);
    pub const E: Self = Self(0x08);
    pub const F: Self = Self(0x09);
    pub const G: Self = Self(0x0a);
    pub const H: Self = Self(0x0b);
    pub const I: Self = Self(0x0c);
    pub const J: Self = Self(0x0d);
    pub const K: Self = Self(0x0e);
    pub const L: Self = Self(0x0f);
    pub const M: Self = Self(0x10);
    pub const N: Self = Self(0x11);
    pub const O: Self = Self(0x12);
    pub const P: Self = Self(0x13);
    pub const Q: Self = Self(0x14);
    pub const R: Self = Self(0x15);
    pub const S: Self = Self(0x16);
    pub const T: Self = Self(0x17);
    pub const U: Self = Self(0x18);
    pub const V: Self = Self(0x19);
    pub const W: Self = Self(0x1a);
    pub const X: Self = Self(0x1b);
    pub const Y: Self = Self(0x1c);
    pub const Z: Self = Self(0x1d);

    // Numbers
    pub const KEY_1: Self = Self(0x1e);
    pub const KEY_2: Self = Self(0x1f);
    pub const KEY_3: Self = Self(0x20);
    pub const KEY_4: Self = Self(0x21);
    pub const KEY_5: Self = Self(0x22);
    pub const KEY_6: Self = Self(0x23);
    pub const KEY_7: Self = Self(0x24);
    pub const KEY_8: Self = Self(0x25);
    pub const KEY_9: Self = Self(0x26);
    pub const KEY_0: Self = Self(0x27);

    // Common Controls
    pub const ENTER: Self = Self(0x28);
    pub const ESCAPE: Self = Self(0x29);
    pub const BACKSPACE: Self = Self(0x2a);
    pub const TAB: Self = Self(0x2b);
    pub const SPACE: Self = Self(0x2c);
    pub const MINUS: Self = Self(0x2d);
    pub const EQUAL: Self = Self(0x2e);
    pub const LEFT_BRACKET: Self = Self(0x2f);
    pub const RIGHT_BRACKET: Self = Self(0x30);
    pub const BACKSLASH: Self = Self(0x31);
    pub const SEMICOLON: Self = Self(0x33);
    pub const QUOTE: Self = Self(0x34);
    pub const GRAVE: Self = Self(0x35);
    pub const COMMA: Self = Self(0x36);
    pub const PERIOD: Self = Self(0x37);
    pub const SLASH: Self = Self(0x38);
    pub const CAPS_LOCK: Self = Self(0x39);

    // Function keys
    pub const F1: Self = Self(0x3a);
    pub const F2: Self = Self(0x3b);
    pub const F3: Self = Self(0x3c);
    pub const F4: Self = Self(0x3d);
    pub const F5: Self = Self(0x3e);
    pub const F6: Self = Self(0x3f);
    pub const F7: Self = Self(0x40);
    pub const F8: Self = Self(0x41);
    pub const F9: Self = Self(0x42);
    pub const F10: Self = Self(0x43);
    pub const F11: Self = Self(0x44);
    pub const F12: Self = Self(0x45);

    // Navigation
    pub const PRINT_SCREEN: Self = Self(0x46);
    pub const SCROLL_LOCK: Self = Self(0x47);
    pub const PAUSE_BREAK: Self = Self(0x48);
    pub const INSERT: Self = Self(0x49);
    pub const HOME: Self = Self(0x4a);
    pub const PAGE_UP: Self = Self(0x4b);
    pub const DELETE: Self = Self(0x4c);
    pub const END: Self = Self(0x4d);
    pub const PAGE_DOWN: Self = Self(0x4e);
    pub const RIGHT_ARROW: Self = Self(0x4f);
    pub const LEFT_ARROW: Self = Self(0x50);
    pub const DOWN_ARROW: Self = Self(0x51);
    pub const UP_ARROW: Self = Self(0x52);

    // Keypad
    pub const NUM_LOCK: Self = Self(0x53);
    pub const KP_SLASH: Self = Self(0x54);
    pub const KP_ASTERISK: Self = Self(0x55);
    pub const KP_MINUS: Self = Self(0x56);
    pub const KP_PLUS: Self = Self(0x57);
    pub const KP_ENTER: Self = Self(0x58);
    pub const KP_1: Self = Self(0x59);
    pub const KP_2: Self = Self(0x5a);
    pub const KP_3: Self = Self(0x5b);
    pub const KP_4: Self = Self(0x5c);
    pub const KP_5: Self = Self(0x5d);
    pub const KP_6: Self = Self(0x5e);
    pub const KP_7: Self = Self(0x5f);
    pub const KP_8: Self = Self(0x60);
    pub const KP_9: Self = Self(0x61);
    pub const KP_0: Self = Self(0x62);
    pub const KP_PERIOD: Self = Self(0x63);
    pub const MENU: Self = Self(0x65);

    /// Convert from raw byte.
    pub const fn from_u8(code: u8) -> Self {
        Self(code)
    }

    /// Convert to raw byte.
    pub const fn to_u8(self) -> u8 {
        self.0
    }

    /// Whether this keycode represents no key.
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// Human-readable name of the keycode.
    pub fn name(self) -> &'static str {
        match self.0 {
            0x00 => "None",
            0x04 => "A",
            0x05 => "B",
            0x06 => "C",
            0x07 => "D",
            0x08 => "E",
            0x09 => "F",
            0x0a => "G",
            0x0b => "H",
            0x0c => "I",
            0x0d => "J",
            0x0e => "K",
            0x0f => "L",
            0x10 => "M",
            0x11 => "N",
            0x12 => "O",
            0x13 => "P",
            0x14 => "Q",
            0x15 => "R",
            0x16 => "S",
            0x17 => "T",
            0x18 => "U",
            0x19 => "V",
            0x1a => "W",
            0x1b => "X",
            0x1c => "Y",
            0x1d => "Z",
            0x1e => "1",
            0x1f => "2",
            0x20 => "3",
            0x21 => "4",
            0x22 => "5",
            0x23 => "6",
            0x24 => "7",
            0x25 => "8",
            0x26 => "9",
            0x27 => "0",
            0x28 => "Enter",
            0x29 => "Esc",
            0x2a => "Backspace",
            0x2b => "Tab",
            0x2c => "Space",
            0x2d => "-",
            0x2e => "=",
            0x2f => "[",
            0x30 => "]",
            0x31 => "\\",
            0x33 => ";",
            0x34 => "'",
            0x35 => "~",
            0x36 => ",",
            0x37 => ".",
            0x38 => "/",
            0x39 => "CapsLock",
            0x3a => "F1",
            0x3b => "F2",
            0x3c => "F3",
            0x3d => "F4",
            0x3e => "F5",
            0x3f => "F6",
            0x40 => "F7",
            0x41 => "F8",
            0x42 => "F9",
            0x43 => "F10",
            0x44 => "F11",
            0x45 => "F12",
            0x46 => "PrintScreen",
            0x47 => "ScrollLock",
            0x48 => "PauseBreak",
            0x49 => "Insert",
            0x4a => "Home",
            0x4b => "PageUp",
            0x4c => "Delete",
            0x4d => "End",
            0x4e => "PageDown",
            0x4f => "Right",
            0x50 => "Left",
            0x51 => "Down",
            0x52 => "Up",
            0x53 => "NumLock",
            0x54 => "Keypad /",
            0x55 => "Keypad *",
            0x56 => "Keypad -",
            0x57 => "Keypad +",
            0x58 => "Keypad Enter",
            0x59 => "Keypad 1",
            0x5a => "Keypad 2",
            0x5b => "Keypad 3",
            0x5c => "Keypad 4",
            0x5d => "Keypad 5",
            0x5e => "Keypad 6",
            0x5f => "Keypad 7",
            0x60 => "Keypad 8",
            0x61 => "Keypad 9",
            0x62 => "Keypad 0",
            0x63 => "Keypad .",
            0x65 => "Menu",
            _ => "Unknown",
        }
    }

    /// Complete list of standard USB HID key options (`None` plus all 96 keypad-mappable keys).
    pub const fn all_keys() -> &'static [KeyCode] {
        &[
            Self::NONE,
            // Letters
            Self::A,
            Self::B,
            Self::C,
            Self::D,
            Self::E,
            Self::F,
            Self::G,
            Self::H,
            Self::I,
            Self::J,
            Self::K,
            Self::L,
            Self::M,
            Self::N,
            Self::O,
            Self::P,
            Self::Q,
            Self::R,
            Self::S,
            Self::T,
            Self::U,
            Self::V,
            Self::W,
            Self::X,
            Self::Y,
            Self::Z,
            // Digits
            Self::KEY_1,
            Self::KEY_2,
            Self::KEY_3,
            Self::KEY_4,
            Self::KEY_5,
            Self::KEY_6,
            Self::KEY_7,
            Self::KEY_8,
            Self::KEY_9,
            Self::KEY_0,
            // Common controls & arrows
            Self::SPACE,
            Self::ENTER,
            Self::ESCAPE,
            Self::TAB,
            Self::BACKSPACE,
            Self::DELETE,
            Self::INSERT,
            Self::HOME,
            Self::END,
            Self::PAGE_UP,
            Self::PAGE_DOWN,
            Self::UP_ARROW,
            Self::DOWN_ARROW,
            Self::LEFT_ARROW,
            Self::RIGHT_ARROW,
            // Function keys
            Self::F1,
            Self::F2,
            Self::F3,
            Self::F4,
            Self::F5,
            Self::F6,
            Self::F7,
            Self::F8,
            Self::F9,
            Self::F10,
            Self::F11,
            Self::F12,
            // Symbols & punctuation
            Self::GRAVE,
            Self::MINUS,
            Self::EQUAL,
            Self::LEFT_BRACKET,
            Self::RIGHT_BRACKET,
            Self::BACKSLASH,
            Self::SEMICOLON,
            Self::QUOTE,
            Self::COMMA,
            Self::PERIOD,
            Self::SLASH,
            // System & lock keys
            Self::CAPS_LOCK,
            Self::PRINT_SCREEN,
            Self::SCROLL_LOCK,
            Self::PAUSE_BREAK,
            Self::MENU,
            // Numpad
            Self::NUM_LOCK,
            Self::KP_SLASH,
            Self::KP_ASTERISK,
            Self::KP_MINUS,
            Self::KP_PLUS,
            Self::KP_ENTER,
            Self::KP_1,
            Self::KP_2,
            Self::KP_3,
            Self::KP_4,
            Self::KP_5,
            Self::KP_6,
            Self::KP_7,
            Self::KP_8,
            Self::KP_9,
            Self::KP_0,
            Self::KP_PERIOD,
        ]
    }

    /// List of key options for UI pickers.
    pub const fn common_keys() -> &'static [KeyCode] {
        Self::all_keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_keycode_names() {
        assert_eq!(KeyCode::Z.name(), "Z");
        assert_eq!(KeyCode::ESCAPE.name(), "Esc");
        assert_eq!(KeyCode::F3.name(), "F3");
        assert_eq!(KeyCode::NONE.name(), "None");
    }

    #[test]
    fn test_all_keys_complete_and_unique() {
        let keys = KeyCode::all_keys();
        assert_eq!(keys.len(), 97); // None + 96 standard HID keys
        assert_eq!(keys[0], KeyCode::NONE);

        let unique: HashSet<u8> = keys.iter().map(|k| k.to_u8()).collect();
        assert_eq!(unique.len(), keys.len());

        for key in keys {
            assert_ne!(key.name(), "Unknown", "missing name for {key:?}");
        }
    }
}
