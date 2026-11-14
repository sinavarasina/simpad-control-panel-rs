use serde::{Deserialize, Serialize};

use crate::error::{Result, SimpadError};

/// 24-bit RGB Color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
    pub const GREEN: Self = Self { r: 0, g: 255, b: 0 };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Format color as a hex string `#RRGGBB`.
    pub fn to_hex_string(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Parse color from a hex string `#RRGGBB` or `RRGGBB`.
    pub fn from_hex_str(s: &str) -> std::result::Result<Self, String> {
        let clean = s.trim().trim_start_matches('#');
        if clean.len() != 6 {
            return Err("Hex color must be 6 hex characters".to_string());
        }
        let r = u8::from_str_radix(&clean[0..2], 16).map_err(|e| e.to_string())?;
        let g = u8::from_str_radix(&clean[2..4], 16).map_err(|e| e.to_string())?;
        let b = u8::from_str_radix(&clean[4..6], 16).map_err(|e| e.to_string())?;
        Ok(Self { r, g, b })
    }
}

/// Brightness level from 1 to 4 (25%..100%; legacy firmware may report 0 for 100%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Brightness(pub u8);

impl Brightness {
    pub const OFF: Self = Self(0);
    pub const MAX: Self = Self(4);

    pub const fn new(level: u8) -> Self {
        if level > 4 { Self(4) } else { Self(level) }
    }

    /// Returns the effective brightness level in `1..=4`, treating legacy firmware `0` as `4` (100%).
    pub const fn effective_level(&self) -> u8 {
        if self.0 == 0 || self.0 > 4 { 4 } else { self.0 }
    }

    pub const fn to_percentage(&self) -> u8 {
        self.effective_level() * 25
    }
}

/// LED configuration (color and brightness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LedConfig {
    pub color: RgbColor,
    pub brightness: Brightness,
}

impl LedConfig {
    pub const fn new(color: RgbColor, brightness: Brightness) -> Self {
        Self { color, brightness }
    }

    pub const fn from_raw_bytes(data: [u8; 4]) -> Self {
        Self {
            color: RgbColor::new(data[0], data[1], data[2]),
            brightness: Brightness::new(data[3]),
        }
    }

    pub const fn to_raw_bytes(&self) -> [u8; 4] {
        [self.color.r, self.color.g, self.color.b, self.brightness.0]
    }
}

/// Dynamic lighting effects supported by SimPad firmware.
///
/// The numeric values correspond directly to the firmware mode codes at pointer 8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum LightMode {
    EaseOut = 0,
    EaseIn = 1,
    KeepOn = 2,
    KeepOff = 3,
    KeyPress = 5,
    #[default]
    RainbowColor = 6,
    RainbowEaseOut = 7,
    KeySwitch = 8,
    RainbowSwitch = 9,
}

impl LightMode {
    pub const fn from_u8(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::EaseOut),
            1 => Ok(Self::EaseIn),
            2 => Ok(Self::KeepOn),
            3 => Ok(Self::KeepOff),
            5 => Ok(Self::KeyPress),
            6 => Ok(Self::RainbowColor),
            7 => Ok(Self::RainbowEaseOut),
            8 => Ok(Self::KeySwitch),
            9 => Ok(Self::RainbowSwitch),
            other => Err(SimpadError::InvalidLightMode(other)),
        }
    }

    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::EaseOut => "Fade Out (Ease Out)",
            Self::EaseIn => "Fade In (Ease In)",
            Self::KeepOn => "Always On",
            Self::KeepOff => "Off",
            Self::KeyPress => "On Key Press",
            Self::RainbowColor => "Rainbow Cycle",
            Self::RainbowEaseOut => "Rainbow Fade Out",
            Self::KeySwitch => "Toggle On/Off",
            Self::RainbowSwitch => "Toggle Rainbow",
        }
    }

    /// Human-readable explanation of the lighting effect shown in the UI.
    pub const fn description(self) -> &'static str {
        match self {
            Self::EaseOut => {
                "Lights flash on key press and smoothly fade out in your selected colors."
            }
            Self::EaseIn => "Lights dim on key press and smoothly fade back in (SimPad v2).",
            Self::KeepOn => "Keeps LEDs continuously lit in your selected colors.",
            Self::KeepOff => "Turns off all LEDs.",
            Self::KeyPress => "Lights up in your selected colors while keys are held down.",
            Self::RainbowColor => "Smoothly cycles LEDs through rainbow spectrum colors.",
            Self::RainbowEaseOut => "Fades out on key press while cycling through rainbow colors.",
            Self::KeySwitch => "Toggles your selected LED colors on or off with each key press.",
            Self::RainbowSwitch => "Cycles through rainbow colors with key-switch response.",
        }
    }

    /// Whether this mode uses per-LED static RGB colors rather than rainbow/off.
    pub const fn uses_static_color(self) -> bool {
        matches!(
            self,
            Self::EaseOut | Self::EaseIn | Self::KeepOn | Self::KeyPress | Self::KeySwitch
        )
    }

    /// Whether LED brightness levels apply to this mode.
    pub const fn uses_brightness(self) -> bool {
        !matches!(self, Self::KeepOff)
    }

    /// Whether the fade/ease delay timing setting affects this mode.
    pub const fn uses_fade_delay(self) -> bool {
        matches!(self, Self::EaseOut | Self::EaseIn | Self::RainbowEaseOut)
    }

    /// Whether the rainbow cycle delay timing setting affects this mode.
    pub const fn uses_rainbow_delay(self) -> bool {
        matches!(
            self,
            Self::RainbowColor | Self::RainbowEaseOut | Self::RainbowSwitch
        )
    }

    pub const fn all_modes() -> &'static [LightMode] {
        &[
            Self::EaseOut,
            Self::EaseIn,
            Self::KeepOn,
            Self::KeyPress,
            Self::KeySwitch,
            Self::KeepOff,
            Self::RainbowColor,
            Self::RainbowEaseOut,
            Self::RainbowSwitch,
        ]
    }

    /// Lighting modes supported by SimPad v2 and SimPad v2 EX.
    pub const fn legacy_v2_modes() -> &'static [LightMode] {
        &[
            Self::EaseOut,
            Self::EaseIn,
            Self::KeepOn,
            Self::KeyPress,
            Self::KeepOff,
            Self::RainbowColor,
        ]
    }

    /// Lighting modes supported by SimPad Nano, Nano Year Edition, and v2 Anniversary.
    pub const fn modern_modes() -> &'static [LightMode] {
        &[
            Self::EaseOut,
            Self::KeepOn,
            Self::KeyPress,
            Self::KeySwitch,
            Self::KeepOff,
            Self::RainbowColor,
            Self::RainbowEaseOut,
            Self::RainbowSwitch,
        ]
    }
}

/// Transition speed delays for lighting animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightSpeed {
    /// Delay for fade in/out transitions (10..=65535, default: 0x0A00 = 2560).
    pub fade_delay: u16,
    /// Delay for rainbow transitions (10..=65535, default: 0x0A00 = 2560).
    pub rainbow_delay: u16,
}

impl Default for LightSpeed {
    fn default() -> Self {
        Self {
            fade_delay: 0x0a00,
            rainbow_delay: 0x0a00,
        }
    }
}

impl LightSpeed {
    pub const fn from_raw_bytes(data: [u8; 4]) -> Self {
        let fade_delay = ((data[0] as u16) << 8) | (data[1] as u16);
        let rainbow_delay = ((data[2] as u16) << 8) | (data[3] as u16);
        Self {
            fade_delay: if fade_delay < 0x0a {
                0x0a00
            } else {
                fade_delay
            },
            rainbow_delay: if rainbow_delay < 0x0a {
                0x0a00
            } else {
                rainbow_delay
            },
        }
    }

    pub const fn to_raw_bytes(&self) -> [u8; 4] {
        [
            (self.fade_delay >> 8) as u8,
            (self.fade_delay & 0xff) as u8,
            (self.rainbow_delay >> 8) as u8,
            (self.rainbow_delay & 0xff) as u8,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_hex_conversion() {
        let color = RgbColor::new(0x12, 0xab, 0xef);
        assert_eq!(color.to_hex_string(), "#12abef");
        let parsed = RgbColor::from_hex_str("#12abef").expect("valid hex");
        assert_eq!(color, parsed);
    }

    #[test]
    fn test_led_config_raw_roundtrip() {
        let led = LedConfig::new(RgbColor::new(0, 255, 0), Brightness::new(4));
        let raw = led.to_raw_bytes();
        assert_eq!(raw, [0, 255, 0, 4]);
        assert_eq!(LedConfig::from_raw_bytes(raw), led);
    }

    #[test]
    fn test_light_mode_roundtrip_and_firmware_ids() {
        for mode in LightMode::all_modes() {
            let u = mode.to_u8();
            assert_eq!(LightMode::from_u8(u).unwrap(), *mode);
        }
        assert_eq!(LightMode::EaseOut.to_u8(), 0);
        assert_eq!(LightMode::KeepOn.to_u8(), 2);
        assert_eq!(LightMode::KeepOff.to_u8(), 3);
        assert_eq!(LightMode::KeyPress.to_u8(), 5);
        assert_eq!(LightMode::RainbowColor.to_u8(), 6);
        assert_eq!(LightMode::RainbowEaseOut.to_u8(), 7);
        assert_eq!(LightMode::KeySwitch.to_u8(), 8);
        assert_eq!(LightMode::RainbowSwitch.to_u8(), 9);
        assert!(LightMode::from_u8(4).is_err());
    }

    #[test]
    fn test_light_speed_roundtrip() {
        let speed = LightSpeed {
            fade_delay: 1000,
            rainbow_delay: 2000,
        };
        let raw = speed.to_raw_bytes();
        let restored = LightSpeed::from_raw_bytes(raw);
        assert_eq!(speed, restored);
    }

    #[test]
    fn test_brightness_effective_level_and_percentage() {
        // Legacy firmware value 0 maps to effective level 4 (100%) without mutating raw byte 0.
        let legacy_zero = Brightness::new(0);
        assert_eq!(legacy_zero.0, 0);
        assert_eq!(legacy_zero.effective_level(), 4);
        assert_eq!(legacy_zero.to_percentage(), 100);

        for (level, pct) in [(1, 25), (2, 50), (3, 75), (4, 100)] {
            let b = Brightness::new(level);
            assert_eq!(b.effective_level(), level);
            assert_eq!(b.to_percentage(), pct);
        }

        // Clamping above 4 returns level 4.
        assert_eq!(Brightness::new(9).effective_level(), 4);
    }

    #[test]
    fn test_light_mode_feature_predicates() {
        for mode in LightMode::all_modes() {
            assert!(!mode.description().is_empty());
        }

        // Static color modes with fade delay
        assert!(LightMode::EaseOut.uses_static_color());
        assert!(LightMode::EaseOut.uses_brightness());
        assert!(LightMode::EaseOut.uses_fade_delay());
        assert!(!LightMode::EaseOut.uses_rainbow_delay());

        assert!(LightMode::EaseIn.uses_static_color());
        assert!(LightMode::EaseIn.uses_brightness());
        assert!(LightMode::EaseIn.uses_fade_delay());
        assert!(!LightMode::EaseIn.uses_rainbow_delay());

        // Static color modes without timing delays
        for mode in [LightMode::KeepOn, LightMode::KeyPress, LightMode::KeySwitch] {
            assert!(mode.uses_static_color());
            assert!(mode.uses_brightness());
            assert!(!mode.uses_fade_delay());
            assert!(!mode.uses_rainbow_delay());
        }

        // Off mode hides static color, brightness, and both timing delays
        assert!(!LightMode::KeepOff.uses_static_color());
        assert!(!LightMode::KeepOff.uses_brightness());
        assert!(!LightMode::KeepOff.uses_fade_delay());
        assert!(!LightMode::KeepOff.uses_rainbow_delay());

        // Rainbow modes hide static color pickers but keep brightness and relevant delays
        assert!(!LightMode::RainbowColor.uses_static_color());
        assert!(LightMode::RainbowColor.uses_brightness());
        assert!(!LightMode::RainbowColor.uses_fade_delay());
        assert!(LightMode::RainbowColor.uses_rainbow_delay());

        assert!(!LightMode::RainbowEaseOut.uses_static_color());
        assert!(LightMode::RainbowEaseOut.uses_brightness());
        assert!(LightMode::RainbowEaseOut.uses_fade_delay());
        assert!(LightMode::RainbowEaseOut.uses_rainbow_delay());

        assert!(!LightMode::RainbowSwitch.uses_static_color());
        assert!(LightMode::RainbowSwitch.uses_brightness());
        assert!(!LightMode::RainbowSwitch.uses_fade_delay());
        assert!(LightMode::RainbowSwitch.uses_rainbow_delay());
    }
}
