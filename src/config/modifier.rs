use serde::{Deserialize, Serialize};

/// Keyboard modifier flags encoded as a bitmask in data byte 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModifierFlags {
    pub left_ctrl: bool,
    pub left_shift: bool,
    pub left_alt: bool,
    pub left_win: bool,
    pub right_ctrl: bool,
    pub right_shift: bool,
    pub right_alt: bool,
    pub right_win: bool,
}

impl ModifierFlags {
    pub const LEFT_CTRL: u8 = 0b0000_0001;
    pub const LEFT_SHIFT: u8 = 0b0000_0010;
    pub const LEFT_ALT: u8 = 0b0000_0100;
    pub const LEFT_WIN: u8 = 0b0000_1000;
    pub const RIGHT_CTRL: u8 = 0b0001_0000;
    pub const RIGHT_SHIFT: u8 = 0b0010_0000;
    pub const RIGHT_ALT: u8 = 0b0100_0000;
    pub const RIGHT_WIN: u8 = 0b1000_0000;

    /// Construct flags from a raw byte.
    pub const fn from_bits(bits: u8) -> Self {
        Self {
            left_ctrl: (bits & Self::LEFT_CTRL) != 0,
            left_shift: (bits & Self::LEFT_SHIFT) != 0,
            left_alt: (bits & Self::LEFT_ALT) != 0,
            left_win: (bits & Self::LEFT_WIN) != 0,
            right_ctrl: (bits & Self::RIGHT_CTRL) != 0,
            right_shift: (bits & Self::RIGHT_SHIFT) != 0,
            right_alt: (bits & Self::RIGHT_ALT) != 0,
            right_win: (bits & Self::RIGHT_WIN) != 0,
        }
    }

    /// Encode flags into a raw byte.
    pub const fn to_bits(&self) -> u8 {
        let mut bits = 0u8;
        if self.left_ctrl {
            bits |= Self::LEFT_CTRL;
        }
        if self.left_shift {
            bits |= Self::LEFT_SHIFT;
        }
        if self.left_alt {
            bits |= Self::LEFT_ALT;
        }
        if self.left_win {
            bits |= Self::LEFT_WIN;
        }
        if self.right_ctrl {
            bits |= Self::RIGHT_CTRL;
        }
        if self.right_shift {
            bits |= Self::RIGHT_SHIFT;
        }
        if self.right_alt {
            bits |= Self::RIGHT_ALT;
        }
        if self.right_win {
            bits |= Self::RIGHT_WIN;
        }
        bits
    }

    /// Whether any modifier is active.
    pub const fn is_empty(&self) -> bool {
        self.to_bits() == 0
    }

    /// Human-readable list of active modifiers.
    pub fn names(&self) -> Vec<&'static str> {
        let mut list = Vec::new();
        if self.left_ctrl {
            list.push("LCtrl");
        }
        if self.left_shift {
            list.push("LShift");
        }
        if self.left_alt {
            list.push("LAlt");
        }
        if self.left_win {
            list.push("LWin");
        }
        if self.right_ctrl {
            list.push("RCtrl");
        }
        if self.right_shift {
            list.push("RShift");
        }
        if self.right_alt {
            list.push("RAlt");
        }
        if self.right_win {
            list.push("RWin");
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_bits_roundtrip() {
        let flags = ModifierFlags {
            left_ctrl: true,
            left_alt: true,
            right_shift: true,
            ..Default::default()
        };
        let bits = flags.to_bits();
        assert_eq!(bits, 0x01 | 0x04 | 0x20);
        let restored = ModifierFlags::from_bits(bits);
        assert_eq!(flags, restored);
    }
}
