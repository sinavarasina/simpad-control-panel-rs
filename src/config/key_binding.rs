use serde::{Deserialize, Serialize};
use std::fmt;

use crate::config::key_code::KeyCode;
use crate::config::modifier::ModifierFlags;
use crate::config::mouse::MouseButtonFlags;

/// A full key shortcut binding configured for a SimPad key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KeyBinding {
    pub modifiers: ModifierFlags,
    pub key: KeyCode,
    pub mouse: MouseButtonFlags,
}

impl KeyBinding {
    /// Create a new key binding.
    pub const fn new(modifiers: ModifierFlags, key: KeyCode, mouse: MouseButtonFlags) -> Self {
        Self {
            modifiers,
            key,
            mouse,
        }
    }

    /// Construct a binding that triggers a single keyboard key with no modifiers.
    pub const fn from_key(key: KeyCode) -> Self {
        Self {
            modifiers: ModifierFlags::from_bits(0),
            key,
            mouse: MouseButtonFlags::from_bits(0),
        }
    }

    /// Construct from raw 4-byte setting data.
    pub const fn from_raw_bytes(data: [u8; 4]) -> Self {
        Self {
            modifiers: ModifierFlags::from_bits(data[0]),
            key: KeyCode::from_u8(data[1]),
            mouse: MouseButtonFlags::from_bits(data[2]),
        }
    }

    /// Convert into raw 4-byte setting data.
    pub const fn to_raw_bytes(&self) -> [u8; 4] {
        [
            self.modifiers.to_bits(),
            self.key.to_u8(),
            self.mouse.to_bits(),
            0x00,
        ]
    }

    /// Whether this binding has no action assigned.
    pub const fn is_empty(&self) -> bool {
        self.modifiers.is_empty() && self.key.is_none() && self.mouse.is_empty()
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        for mod_name in self.modifiers.names() {
            parts.push(mod_name.to_string());
        }

        if !self.key.is_none() {
            parts.push(self.key.name().to_string());
        }

        for mouse_name in self.mouse.names() {
            parts.push(mouse_name.to_string());
        }

        if parts.is_empty() {
            write!(f, "None")
        } else {
            write!(f, "{}", parts.join(" + "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_display() {
        let binding = KeyBinding {
            modifiers: ModifierFlags {
                left_ctrl: true,
                ..Default::default()
            },
            key: KeyCode::R,
            mouse: MouseButtonFlags::default(),
        };
        assert_eq!(binding.to_string(), "LCtrl + R");

        let raw = binding.to_raw_bytes();
        let restored = KeyBinding::from_raw_bytes(raw);
        assert_eq!(binding, restored);
    }

    #[test]
    fn test_empty_binding() {
        let binding = KeyBinding::default();
        assert!(binding.is_empty());
        assert_eq!(binding.to_string(), "None");
    }
}
