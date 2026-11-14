use serde::{Deserialize, Serialize};

use crate::config::key_binding::KeyBinding;
use crate::config::key_code::KeyCode;
use crate::config::lighting::{Brightness, LedConfig, LightMode, LightSpeed, RgbColor};
use crate::config::modifier::ModifierFlags;
use crate::device::DeviceModel;

fn default_model() -> DeviceModel {
    DeviceModel::SimpadV2
}

/// Full configuration snapshot for a SimPad device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSettings {
    /// Device model these settings are scoped to.
    #[serde(default = "default_model")]
    pub model: DeviceModel,
    /// Keyboard key shortcuts mapped by physical key index.
    pub keys: Vec<KeyBinding>,
    /// LED lighting configurations (typically 2 for LED 0 and LED 1).
    pub leds: Vec<LedConfig>,
    /// Active dynamic lighting effect mode.
    pub light_mode: LightMode,
    /// Anti-jitter input debounce delay ticks.
    pub debounce_delay: u32,
    /// Full speed mode (turns off lighting for lowest possible input latency).
    pub full_speed_mode: bool,
    /// Ambient night light status (`true` = keep lighting on when host sleeps/off).
    pub night_light: bool,
    /// Preserved untouched bytes `[1]` and `[3]` from pointer 10.
    #[serde(default)]
    pub raw_p10_extra: [u8; 2],
    /// Lighting animation speed delays.
    pub light_speed: LightSpeed,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self::default_for_model(DeviceModel::SimpadV2)
    }
}

impl DeviceSettings {
    /// Factory default settings tailored to the exact capabilities and key count of `model`.
    pub fn default_for_model(model: DeviceModel) -> Self {
        let caps = model.capabilities();
        let keys = match model {
            DeviceModel::Bootloader => Vec::new(),
            DeviceModel::SimpadNano | DeviceModel::SimpadNanoYearEdition => vec![
                KeyBinding::from_key(KeyCode::Z),
                KeyBinding::from_key(KeyCode::X),
                KeyBinding::from_key(KeyCode::ESCAPE),
            ],
            DeviceModel::SimpadSpE | DeviceModel::SimpadSpE2 => vec![
                KeyBinding::from_key(KeyCode::KEY_1),
                KeyBinding::from_key(KeyCode::KEY_2),
                KeyBinding::from_key(KeyCode::KEY_3),
                KeyBinding::from_key(KeyCode::KEY_4),
                KeyBinding::from_key(KeyCode::KEY_5),
                KeyBinding::from_key(KeyCode::KEY_6),
                KeyBinding::from_key(KeyCode::KEY_7),
                KeyBinding::from_key(KeyCode::KEY_8),
            ],
            _ => {
                let mut base = vec![
                    KeyBinding::from_key(KeyCode::Z),
                    KeyBinding::from_key(KeyCode::X),
                    KeyBinding {
                        modifiers: ModifierFlags {
                            left_ctrl: true,
                            ..Default::default()
                        },
                        key: KeyCode::R,
                        ..Default::default()
                    },
                    KeyBinding::from_key(KeyCode::ESCAPE),
                    KeyBinding::from_key(KeyCode::F3),
                ];
                if caps.key_count == 6 {
                    base.push(KeyBinding::from_key(KeyCode::ENTER));
                }
                base
            }
        };

        let leds = match caps.led_count {
            0 => Vec::new(),
            1 => vec![LedConfig::new(RgbColor::GREEN, Brightness::MAX)],
            _ => vec![
                LedConfig::new(RgbColor::GREEN, Brightness::MAX),
                LedConfig::new(RgbColor::BLUE, Brightness::MAX),
            ],
        };

        let light_mode = if caps.supported_light_modes.is_empty() {
            LightMode::KeepOff
        } else {
            LightMode::RainbowColor
        };

        Self {
            model,
            keys,
            leds,
            light_mode,
            debounce_delay: if caps.key_count == 8 { 0x80 } else { 0x60 },
            full_speed_mode: caps.key_count == 8,
            night_light: caps.has_night_light,
            raw_p10_extra: [0x00, 0x00],
            light_speed: LightSpeed::default(),
        }
    }

    /// Default settings for a 5-key SimPad v2.
    pub fn default_5_keys() -> Self {
        Self::default_for_model(DeviceModel::SimpadV2)
    }

    /// Default settings for an 8-key SimPad (SP-E, SP-E2).
    pub fn default_8_keys() -> Self {
        Self::default_for_model(DeviceModel::SimpadSpE)
    }

    /// Convert a setting pointer into raw 4-byte payload according to `self.model`.
    pub fn get_raw_setting(&self, pointer: u8) -> Option<[u8; 4]> {
        if let Some(key_idx) =
            (0..self.keys.len()).find(|&idx| self.model.key_pointer(idx) == Some(pointer))
        {
            return Some(self.keys[key_idx].to_raw_bytes());
        }

        let caps = self.model.capabilities();
        match pointer {
            6 if caps.led_count >= 1 && !self.leds.is_empty() => Some(self.leds[0].to_raw_bytes()),
            7 if caps.led_count >= 2 && self.leds.len() >= 2 => Some(self.leds[1].to_raw_bytes()),
            8 if !caps.supported_light_modes.is_empty() => {
                Some([self.light_mode.to_u8(), 0x00, 0x00, 0x00])
            }
            9 if caps.has_debounce => Some(self.debounce_delay.to_be_bytes()),
            10 if caps.has_full_speed || caps.has_night_light => Some([
                if caps.has_full_speed && self.full_speed_mode {
                    0x01
                } else {
                    0x00
                },
                self.raw_p10_extra[0],
                if caps.has_night_light && !self.night_light {
                    0x01
                } else {
                    0x00
                },
                self.raw_p10_extra[1],
            ]),
            11 if caps.has_light_speed => Some(self.light_speed.to_raw_bytes()),
            _ => None,
        }
    }

    /// Update internal state from a received raw setting packet according to `self.model`.
    pub fn apply_raw_setting(&mut self, pointer: u8, data: [u8; 4]) {
        if let Some(key_idx) =
            (0..self.keys.len()).find(|&idx| self.model.key_pointer(idx) == Some(pointer))
        {
            self.keys[key_idx] = KeyBinding::from_raw_bytes(data);
            return;
        }

        let caps = self.model.capabilities();
        match pointer {
            6 if caps.led_count >= 1 && !self.leds.is_empty() => {
                self.leds[0] = LedConfig::from_raw_bytes(data);
            }
            7 if caps.led_count >= 2 && self.leds.len() >= 2 => {
                self.leds[1] = LedConfig::from_raw_bytes(data);
            }
            8 if !caps.supported_light_modes.is_empty() => {
                if let Ok(mode) = LightMode::from_u8(data[0]) {
                    self.light_mode = mode;
                }
            }
            9 if caps.has_debounce => {
                self.debounce_delay = u32::from_be_bytes(data);
            }
            10 if caps.has_full_speed || caps.has_night_light => {
                if caps.has_full_speed {
                    self.full_speed_mode = data[0] == 0x01;
                }
                self.raw_p10_extra = [data[1], data[3]];
                if caps.has_night_light {
                    self.night_light = data[2] == 0x00;
                }
            }
            11 if caps.has_light_speed => {
                self.light_speed = LightSpeed::from_raw_bytes(data);
            }
            _ => {}
        }
    }

    /// Compare with another settings struct and return the list of pointers and data
    /// that need to be updated.
    pub fn diff(&self, other: &DeviceSettings) -> Vec<(u8, [u8; 4])> {
        let mut changes = Vec::new();
        for pointer in 1..=11 {
            if let (Some(c), Some(t)) = (
                self.get_raw_setting(pointer),
                other.get_raw_setting(pointer),
            ) && c != t
            {
                changes.push((pointer, t));
            }
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_diff_5_keys() {
        let original = DeviceSettings::default_for_model(DeviceModel::SimpadV2);
        let mut modified = original.clone();
        modified.keys[0] = KeyBinding::from_key(KeyCode::A);
        modified.light_mode = LightMode::KeepOn;

        let diff = original.diff(&modified);
        assert_eq!(diff.len(), 2);
        assert_eq!(diff[0].0, 1);
        assert_eq!(diff[1].0, 8);
    }

    #[test]
    fn test_nano_3_keys_ignores_pointers_4_5() {
        let nano = DeviceSettings::default_for_model(DeviceModel::SimpadNanoYearEdition);
        assert_eq!(nano.keys.len(), 3);
        assert!(nano.get_raw_setting(1).is_some());
        assert!(nano.get_raw_setting(3).is_some());
        assert_eq!(nano.get_raw_setting(4), None);
        assert_eq!(nano.get_raw_setting(5), None);
        assert!(nano.get_raw_setting(6).is_some()); // LED 0
        assert!(nano.get_raw_setting(11).is_some()); // LightSpeed
    }

    #[test]
    fn test_v2ex_6_keys_maps_bt6_to_pointer_11() {
        let mut v2ex = DeviceSettings::default_for_model(DeviceModel::SimpadV2Ex);
        assert_eq!(v2ex.keys.len(), 6);
        v2ex.apply_raw_setting(11, KeyBinding::from_key(KeyCode::SPACE).to_raw_bytes());
        assert_eq!(v2ex.keys[5].key, KeyCode::SPACE);
        assert_eq!(
            v2ex.get_raw_setting(11),
            Some(KeyBinding::from_key(KeyCode::SPACE).to_raw_bytes())
        );
    }

    #[test]
    fn test_night_light_encoding_and_p10_extra_preservation() {
        let mut settings = DeviceSettings::default_for_model(DeviceModel::SimpadNanoYearEdition);
        // Hardware data: full_speed=0, extra[0]=0x40, night_light=ON(0x00), extra[1]=0x00
        settings.apply_raw_setting(10, [0x00, 0x40, 0x00, 0x12]);
        assert!(!settings.full_speed_mode);
        assert!(settings.night_light);
        assert_eq!(settings.get_raw_setting(10), Some([0x00, 0x40, 0x00, 0x12]));

        // Turning night_light OFF writes 0x01 into byte 2 while preserving extra bytes
        settings.night_light = false;
        assert_eq!(settings.get_raw_setting(10), Some([0x00, 0x40, 0x01, 0x12]));

        // Reading 0x01 sets night_light = false
        settings.apply_raw_setting(10, [0x01, 0x00, 0x01, 0x00]);
        assert!(settings.full_speed_mode);
        assert!(!settings.night_light);
    }

    #[test]
    fn test_sayo_o2c_capabilities_pointers() {
        let o2c = DeviceSettings::default_for_model(DeviceModel::SayoO2C);
        assert_eq!(o2c.keys.len(), 5);
        assert_eq!(o2c.leds.len(), 2);
        assert!(o2c.get_raw_setting(6).is_some());
        assert!(o2c.get_raw_setting(7).is_some());
        assert_eq!(o2c.get_raw_setting(8), None);
        assert_eq!(o2c.get_raw_setting(9), None);
        assert_eq!(o2c.get_raw_setting(10), None);
        assert_eq!(o2c.get_raw_setting(11), None);
    }

    #[test]
    fn test_debounce_raw_conversion() {
        let mut settings = DeviceSettings::default_5_keys();
        settings.debounce_delay = 0x01020304;
        let raw = settings.get_raw_setting(9).unwrap();
        assert_eq!(raw, [0x01, 0x02, 0x03, 0x04]);

        let mut fresh = DeviceSettings::default_5_keys();
        fresh.apply_raw_setting(9, [0x00, 0x00, 0x00, 0x60]);
        assert_eq!(fresh.debounce_delay, 96);
    }
}
