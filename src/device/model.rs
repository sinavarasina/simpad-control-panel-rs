use serde::{Deserialize, Serialize};

use crate::config::LightMode;

/// Hardware capabilities and feature flags for a specific device model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceCapabilities {
    pub key_count: usize,
    pub led_count: usize,
    pub supported_light_modes: &'static [LightMode],
    pub has_brightness: bool,
    pub has_light_speed: bool,
    pub has_night_light: bool,
    pub has_debounce: bool,
    pub has_full_speed: bool,
    pub has_chip_id: bool,
}

/// Supported SimPad and compatible device models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceModel {
    SimpadV2,
    SimpadV2Ex,
    SimpadV2Lite,
    SimpadNano,
    SimpadV2Anniversary,
    SimpadNanoYearEdition,
    SimpadSpE,
    SimpadSpE2,
    SayoO2C,
    Bootloader,
}

impl DeviceModel {
    /// Match a device model by USB Vendor ID and Product ID.
    pub fn from_vid_pid(vendor_id: u16, product_id: u16) -> Option<Self> {
        match (vendor_id, product_id) {
            (0x8088, 0x0001) => Some(Self::SimpadV2),
            (0x8088, 0x0002) => Some(Self::SimpadV2Ex),
            (0x8088, 0x0003) => Some(Self::SimpadV2Lite),
            (0x8088, 0x0004) => Some(Self::SimpadNano),
            (0x8088, 0x0006) => Some(Self::SimpadV2Anniversary),
            (0x8088, 0x0007) => Some(Self::SimpadNanoYearEdition),
            (0x8088, 0x0102) => Some(Self::SimpadSpE2),
            (0x8088, 0x0302) => Some(Self::SimpadSpE),
            (0x8089, 0x0003) => Some(Self::SayoO2C),
            (0x8088, 0x00ff) => Some(Self::Bootloader),
            _ => None,
        }
    }

    /// Primary Vendor ID and Product ID pair for this model.
    pub const fn vid_pid(&self) -> (u16, u16) {
        match self {
            Self::SimpadV2 => (0x8088, 0x0001),
            Self::SimpadV2Ex => (0x8088, 0x0002),
            Self::SimpadV2Lite => (0x8088, 0x0003),
            Self::SimpadNano => (0x8088, 0x0004),
            Self::SimpadV2Anniversary => (0x8088, 0x0006),
            Self::SimpadNanoYearEdition => (0x8088, 0x0007),
            Self::SimpadSpE2 => (0x8088, 0x0102),
            Self::SimpadSpE => (0x8088, 0x0302),
            Self::SayoO2C => (0x8089, 0x0003),
            Self::Bootloader => (0x8088, 0x00ff),
        }
    }

    /// User-friendly display name.
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::SimpadV2 => "SimPad v2",
            Self::SimpadV2Ex => "SimPad v2 EX",
            Self::SimpadV2Lite => "SimPad v2 Lite",
            Self::SimpadNano => "SimPad Nano",
            Self::SimpadV2Anniversary => "SimPad v2 Anniversary Edition",
            Self::SimpadNanoYearEdition => "SimPad Nano Year Edition",
            Self::SimpadSpE => "SimPad Special Edition 8 Keys (SP-E)",
            Self::SimpadSpE2 => "SimPad Special Edition 8 Keys (SP-E2)",
            Self::SayoO2C => "Sayo's KeyPad O2C",
            Self::Bootloader => "SimPad Bootloader / IAP",
        }
    }

    /// Full hardware capability descriptor for this model.
    pub const fn capabilities(&self) -> DeviceCapabilities {
        match self {
            Self::SimpadNano | Self::SimpadNanoYearEdition => DeviceCapabilities {
                key_count: 3,
                led_count: 2,
                supported_light_modes: LightMode::modern_modes(),
                has_brightness: true,
                has_light_speed: true,
                has_night_light: true,
                has_debounce: true,
                has_full_speed: true,
                has_chip_id: true,
            },
            Self::SimpadV2 => DeviceCapabilities {
                key_count: 5,
                led_count: 2,
                supported_light_modes: LightMode::legacy_v2_modes(),
                has_brightness: false,
                has_light_speed: true,
                has_night_light: true,
                has_debounce: true,
                has_full_speed: true,
                has_chip_id: false,
            },
            Self::SimpadV2Anniversary => DeviceCapabilities {
                key_count: 5,
                led_count: 2,
                supported_light_modes: LightMode::modern_modes(),
                has_brightness: true,
                has_light_speed: true,
                has_night_light: true,
                has_debounce: true,
                has_full_speed: true,
                has_chip_id: true,
            },
            Self::SimpadV2Ex => DeviceCapabilities {
                key_count: 6,
                led_count: 2,
                supported_light_modes: LightMode::legacy_v2_modes(),
                has_brightness: false,
                has_light_speed: false, // Pointer 11 is occupied by key 6 (BT6)
                has_night_light: false,
                has_debounce: true,
                has_full_speed: true,
                has_chip_id: false,
            },
            Self::SimpadV2Lite => DeviceCapabilities {
                key_count: 5,
                led_count: 0,
                supported_light_modes: &[],
                has_brightness: false,
                has_light_speed: false,
                has_night_light: false,
                has_debounce: true,
                has_full_speed: true,
                has_chip_id: false,
            },
            Self::SimpadSpE | Self::SimpadSpE2 => DeviceCapabilities {
                key_count: 8,
                led_count: 0,
                supported_light_modes: &[],
                has_brightness: false,
                has_light_speed: false,
                has_night_light: false,
                has_debounce: true,
                has_full_speed: true,
                has_chip_id: false,
            },
            Self::SayoO2C => DeviceCapabilities {
                key_count: 5,
                led_count: 2,
                supported_light_modes: &[],
                has_brightness: false,
                has_light_speed: false,
                has_night_light: false,
                has_debounce: false,
                has_full_speed: false,
                has_chip_id: false,
            },
            Self::Bootloader => DeviceCapabilities {
                key_count: 0,
                led_count: 0,
                supported_light_modes: &[],
                has_brightness: false,
                has_light_speed: false,
                has_night_light: false,
                has_debounce: false,
                has_full_speed: false,
                has_chip_id: false,
            },
        }
    }

    /// Number of physical keys on the device.
    pub const fn key_count(&self) -> usize {
        self.capabilities().key_count
    }

    /// HID protocol pointer for a zero-based key index.
    pub const fn key_pointer(&self, index: usize) -> Option<u8> {
        if index >= self.key_count() {
            return None;
        }
        match (self, index) {
            (Self::SimpadV2Ex, 5) => Some(11),
            _ => Some((index + 1) as u8),
        }
    }

    /// Number of configurable RGB LEDs.
    pub const fn led_count(&self) -> usize {
        self.capabilities().led_count
    }

    /// Whether the device has configurable RGB LEDs.
    pub const fn has_rgb_lighting(&self) -> bool {
        self.led_count() > 0
    }

    /// Whether the device is currently in Bootloader / IAP mode.
    pub const fn is_bootloader(&self) -> bool {
        matches!(self, Self::Bootloader)
    }

    /// All known SimPad models.
    pub const fn all_models() -> &'static [DeviceModel] {
        &[
            Self::SimpadV2,
            Self::SimpadV2Ex,
            Self::SimpadV2Lite,
            Self::SimpadNano,
            Self::SimpadV2Anniversary,
            Self::SimpadNanoYearEdition,
            Self::SimpadSpE,
            Self::SimpadSpE2,
            Self::SayoO2C,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vid_pid_matching() {
        assert_eq!(
            DeviceModel::from_vid_pid(0x8088, 0x0001),
            Some(DeviceModel::SimpadV2)
        );
        assert_eq!(
            DeviceModel::from_vid_pid(0x8088, 0x0302),
            Some(DeviceModel::SimpadSpE)
        );
        assert_eq!(
            DeviceModel::from_vid_pid(0x8089, 0x0003),
            Some(DeviceModel::SayoO2C)
        );
        assert_eq!(DeviceModel::from_vid_pid(0x1234, 0x5678), None);
    }

    #[test]
    fn test_key_and_led_counts_and_pointers_all_models() {
        assert_eq!(DeviceModel::SimpadNano.key_count(), 3);
        assert_eq!(DeviceModel::SimpadNanoYearEdition.key_count(), 3);
        assert_eq!(DeviceModel::SimpadNanoYearEdition.key_pointer(0), Some(1));
        assert_eq!(DeviceModel::SimpadNanoYearEdition.key_pointer(2), Some(3));
        assert_eq!(DeviceModel::SimpadNanoYearEdition.key_pointer(3), None);

        assert_eq!(DeviceModel::SimpadV2.key_count(), 5);
        assert_eq!(DeviceModel::SimpadV2.led_count(), 2);
        assert_eq!(DeviceModel::SimpadV2Anniversary.key_count(), 5);

        assert_eq!(DeviceModel::SimpadV2Ex.key_count(), 6);
        assert_eq!(DeviceModel::SimpadV2Ex.key_pointer(4), Some(5));
        assert_eq!(DeviceModel::SimpadV2Ex.key_pointer(5), Some(11));
        assert_eq!(DeviceModel::SimpadV2Ex.key_pointer(6), None);

        assert_eq!(DeviceModel::SimpadV2Lite.key_count(), 5);
        assert_eq!(DeviceModel::SimpadV2Lite.led_count(), 0);

        assert_eq!(DeviceModel::SimpadSpE.key_count(), 8);
        assert_eq!(DeviceModel::SimpadSpE.led_count(), 0);
        assert_eq!(DeviceModel::SimpadSpE2.key_count(), 8);
        assert_eq!(DeviceModel::SimpadSpE2.key_pointer(7), Some(8));

        assert_eq!(DeviceModel::SayoO2C.key_count(), 5);
        assert_eq!(DeviceModel::SayoO2C.led_count(), 2);

        assert_eq!(DeviceModel::Bootloader.key_count(), 0);
        assert!(DeviceModel::Bootloader.is_bootloader());
    }

    #[test]
    fn test_capabilities_across_models() {
        let nano = DeviceModel::SimpadNanoYearEdition.capabilities();
        assert!(nano.has_brightness);
        assert!(nano.has_light_speed);
        assert!(nano.has_night_light);
        assert!(nano.has_chip_id);
        assert_eq!(nano.supported_light_modes, LightMode::modern_modes());

        let v2 = DeviceModel::SimpadV2.capabilities();
        assert!(!v2.has_brightness);
        assert!(v2.has_light_speed);
        assert!(v2.has_night_light);
        assert!(!v2.has_chip_id);
        assert_eq!(v2.supported_light_modes, LightMode::legacy_v2_modes());

        let v2ex = DeviceModel::SimpadV2Ex.capabilities();
        assert!(!v2ex.has_light_speed);
        assert!(!v2ex.has_night_light);
        assert!(v2ex.has_debounce);
        assert!(v2ex.has_full_speed);

        let o2c = DeviceModel::SayoO2C.capabilities();
        assert!(DeviceModel::SayoO2C.has_rgb_lighting());
        assert!(o2c.supported_light_modes.is_empty());
        assert!(!o2c.has_debounce);
        assert!(!o2c.has_full_speed);
    }
}
