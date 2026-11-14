use serde::{Deserialize, Serialize};

/// Mouse button flags encoded as a bitmask in data byte 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MouseButtonFlags {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

impl MouseButtonFlags {
    pub const LEFT: u8 = 0b0000_0001;
    pub const RIGHT: u8 = 0b0000_0010;
    pub const MIDDLE: u8 = 0b0000_0100;

    /// Construct flags from a raw byte.
    pub const fn from_bits(bits: u8) -> Self {
        Self {
            left: (bits & Self::LEFT) != 0,
            right: (bits & Self::RIGHT) != 0,
            middle: (bits & Self::MIDDLE) != 0,
        }
    }

    /// Encode flags into a raw byte.
    pub const fn to_bits(&self) -> u8 {
        let mut bits = 0u8;
        if self.left {
            bits |= Self::LEFT;
        }
        if self.right {
            bits |= Self::RIGHT;
        }
        if self.middle {
            bits |= Self::MIDDLE;
        }
        bits
    }

    /// Whether any mouse button is active.
    pub const fn is_empty(&self) -> bool {
        self.to_bits() == 0
    }

    /// Human-readable list of active mouse buttons.
    pub fn names(&self) -> Vec<&'static str> {
        let mut list = Vec::new();
        if self.left {
            list.push("Mouse Left");
        }
        if self.middle {
            list.push("Mouse Middle");
        }
        if self.right {
            list.push("Mouse Right");
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_bits_roundtrip() {
        let flags = MouseButtonFlags {
            left: true,
            right: false,
            middle: true,
        };
        let bits = flags.to_bits();
        assert_eq!(bits, 0x01 | 0x04);
        let restored = MouseButtonFlags::from_bits(bits);
        assert_eq!(flags, restored);
    }
}
