use crate::device::model::DeviceModel;

/// Visual geometry shape for a physical keycap or LED indicator zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementShape {
    RoundedRect,
    Circle,
    /// D-shaped semi-ellipse rocker oriented rightward (used by SimPad v2 EX BT6).
    RockerRight,
}

/// Physical key slot definition in blueprint coordinates (`[0..body_width] x [0..body_height]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeySlotLayout {
    /// Zero-based index matching `DeviceSettings::keys` (`0` = BT1, ...).
    pub index: usize,
    /// Hardware label matching the physical keypad silkscreen (`"BT1"`, `"3"`, ...).
    pub short_label: &'static str,
    pub cx: f64,
    pub cy: f64,
    pub width: f64,
    pub height: f64,
    pub shape: ElementShape,
    /// Draw UP/DN paddle arrows above and below the slot (`SimpadV2Ex` BT6).
    pub show_rocker_arrows: bool,
}

impl KeySlotLayout {
    /// Check whether a blueprint-space point falls inside this physical key slot.
    pub fn contains(&self, bx: f64, by: f64) -> bool {
        match self.shape {
            ElementShape::RoundedRect => {
                (bx - self.cx).abs() <= self.width * 0.5
                    && (by - self.cy).abs() <= self.height * 0.5
            }
            ElementShape::Circle => {
                let rx = self.width * 0.5;
                let ry = self.height * 0.5;
                if rx <= 0.0 || ry <= 0.0 {
                    return false;
                }
                let dx = (bx - self.cx) / rx;
                let dy = (by - self.cy) / ry;
                dx * dx + dy * dy <= 1.0
            }
            ElementShape::RockerRight => {
                let left = self.cx - self.width * 0.5;
                let ry = self.height * 0.5;
                if bx < left || self.width <= 0.0 || ry <= 0.0 {
                    return false;
                }
                let dx = (bx - left) / self.width;
                let dy = (by - self.cy) / ry;
                dx * dx + dy * dy <= 1.0
            }
        }
    }
}

/// Physical LED illumination zone in blueprint coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LedZoneLayout {
    /// Zero-based LED channel index (`0` = LED1/G1, `1` = LED2/G2).
    pub led_index: usize,
    pub label: &'static str,
    pub cx: f64,
    pub cy: f64,
    pub width: f64,
    pub height: f64,
    pub shape: ElementShape,
    /// Primary under-switch LED (`true`) or secondary edge/underglow accent (`false`).
    pub primary: bool,
}

/// Complete physical keypad schematic blueprint for a [`DeviceModel`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceLayout {
    pub body_width: f64,
    pub body_height: f64,
    pub corner_radius: f64,
    /// Extra blueprint units reserved above `y = 0` (e.g. Nano USB arrow indicator).
    pub top_reserve: f64,
    pub bottom_reserve: f64,
    pub has_usb_indicator: bool,
    pub keys: &'static [KeySlotLayout],
    pub leds: &'static [LedZoneLayout],
}

impl DeviceLayout {
    /// Return the zero-based key index under blueprint coordinate `(bx, by)`, if any.
    pub fn hit_test_key(&self, bx: f64, by: f64) -> Option<usize> {
        self.keys
            .iter()
            .find(|slot| slot.contains(bx, by))
            .map(|slot| slot.index)
    }
}

/// Uniform-aspect transform mapping blueprint space into GTK widget pixel space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasTransform {
    pub origin_x: f64,
    pub origin_y: f64,
    pub scale: f64,
}

impl CanvasTransform {
    /// Fit `layout` cleanly inside `(canvas_w, canvas_h)` while preserving true aspect ratio.
    pub fn fit(canvas_w: f64, canvas_h: f64, layout: &DeviceLayout, padding: f64) -> Self {
        let total_h = (layout.body_height + layout.top_reserve + layout.bottom_reserve).max(1.0);
        let body_w = layout.body_width.max(1.0);
        let avail_w = (canvas_w - 2.0 * padding).max(1.0);
        let avail_h = (canvas_h - 2.0 * padding).max(1.0);
        let scale = (avail_w / body_w).min(avail_h / total_h).max(0.05);

        let origin_x = (canvas_w - body_w * scale) * 0.5;
        let origin_y = (canvas_h - total_h * scale) * 0.5 + layout.top_reserve * scale;
        Self {
            origin_x,
            origin_y,
            scale,
        }
    }

    pub fn to_screen_x(self, bx: f64) -> f64 {
        self.origin_x + bx * self.scale
    }

    pub fn to_screen_y(self, by: f64) -> f64 {
        self.origin_y + by * self.scale
    }

    pub fn len(self, blueprint_len: f64) -> f64 {
        blueprint_len * self.scale
    }

    pub fn to_blueprint(self, sx: f64, sy: f64) -> (f64, f64) {
        (
            (sx - self.origin_x) / self.scale,
            (sy - self.origin_y) / self.scale,
        )
    }
}

const NANO_KEYS: &[KeySlotLayout] = &[
    KeySlotLayout {
        index: 0,
        short_label: "BT1",
        cx: 66.0,
        cy: 50.0,
        width: 64.0,
        height: 64.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 1,
        short_label: "BT2",
        cx: 154.0,
        cy: 50.0,
        width: 64.0,
        height: 64.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 2,
        short_label: "3",
        cx: 110.0,
        cy: 97.0,
        width: 44.0,
        height: 22.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
];

const NANO_LEDS: &[LedZoneLayout] = &[
    LedZoneLayout {
        led_index: 0,
        label: "LED1",
        cx: 66.0,
        cy: 50.0,
        width: 60.0,
        height: 60.0,
        shape: ElementShape::RoundedRect,
        primary: true,
    },
    LedZoneLayout {
        led_index: 1,
        label: "LED2",
        cx: 154.0,
        cy: 50.0,
        width: 60.0,
        height: 60.0,
        shape: ElementShape::RoundedRect,
        primary: true,
    },
];

const V2_KEYS: &[KeySlotLayout] = &[
    KeySlotLayout {
        index: 0,
        short_label: "BT1",
        cx: 64.0,
        cy: 54.0,
        width: 62.0,
        height: 62.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 1,
        short_label: "BT2",
        cx: 146.0,
        cy: 54.0,
        width: 62.0,
        height: 62.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 2,
        short_label: "3",
        cx: 100.0,
        cy: 132.0,
        width: 28.0,
        height: 28.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 3,
        short_label: "4",
        cx: 135.0,
        cy: 123.0,
        width: 28.0,
        height: 28.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 4,
        short_label: "5",
        cx: 170.0,
        cy: 113.0,
        width: 28.0,
        height: 28.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
];

const V2_LEDS: &[LedZoneLayout] = &[
    LedZoneLayout {
        led_index: 0,
        label: "G1",
        cx: 64.0,
        cy: 54.0,
        width: 58.0,
        height: 58.0,
        shape: ElementShape::Circle,
        primary: true,
    },
    LedZoneLayout {
        led_index: 1,
        label: "G2",
        cx: 146.0,
        cy: 54.0,
        width: 58.0,
        height: 58.0,
        shape: ElementShape::Circle,
        primary: true,
    },
    LedZoneLayout {
        led_index: 0,
        label: "G1",
        cx: 26.0,
        cy: 136.0,
        width: 24.0,
        height: 24.0,
        shape: ElementShape::Circle,
        primary: false,
    },
    LedZoneLayout {
        led_index: 1,
        label: "G2",
        cx: 186.0,
        cy: 142.0,
        width: 22.0,
        height: 22.0,
        shape: ElementShape::Circle,
        primary: false,
    },
];

const V2_EX_KEYS: &[KeySlotLayout] = &[
    KeySlotLayout {
        index: 0,
        short_label: "BT1",
        cx: 60.0,
        cy: 54.0,
        width: 60.0,
        height: 60.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 1,
        short_label: "BT2",
        cx: 138.0,
        cy: 54.0,
        width: 60.0,
        height: 60.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 2,
        short_label: "3",
        cx: 95.0,
        cy: 132.0,
        width: 27.0,
        height: 27.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 3,
        short_label: "4",
        cx: 129.0,
        cy: 123.0,
        width: 27.0,
        height: 27.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 4,
        short_label: "5",
        cx: 163.0,
        cy: 113.0,
        width: 27.0,
        height: 27.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 5,
        short_label: "6",
        cx: 210.0,
        cy: 74.0,
        width: 32.0,
        height: 58.0,
        shape: ElementShape::RockerRight,
        show_rocker_arrows: true,
    },
];

const V2_EX_LEDS: &[LedZoneLayout] = &[
    LedZoneLayout {
        led_index: 0,
        label: "G1",
        cx: 60.0,
        cy: 54.0,
        width: 56.0,
        height: 56.0,
        shape: ElementShape::Circle,
        primary: true,
    },
    LedZoneLayout {
        led_index: 1,
        label: "G2",
        cx: 138.0,
        cy: 54.0,
        width: 56.0,
        height: 56.0,
        shape: ElementShape::Circle,
        primary: true,
    },
    LedZoneLayout {
        led_index: 0,
        label: "G1",
        cx: 24.0,
        cy: 140.0,
        width: 22.0,
        height: 22.0,
        shape: ElementShape::Circle,
        primary: false,
    },
    LedZoneLayout {
        led_index: 1,
        label: "G2",
        cx: 224.0,
        cy: 144.0,
        width: 20.0,
        height: 20.0,
        shape: ElementShape::Circle,
        primary: false,
    },
];

const SAYO_O2C_KEYS: &[KeySlotLayout] = &[
    KeySlotLayout {
        index: 0,
        short_label: "BT1",
        cx: 64.0,
        cy: 54.0,
        width: 62.0,
        height: 62.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 1,
        short_label: "BT2",
        cx: 146.0,
        cy: 54.0,
        width: 62.0,
        height: 62.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 2,
        short_label: "3",
        cx: 61.0,
        cy: 124.0,
        width: 28.0,
        height: 28.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 3,
        short_label: "4",
        cx: 105.0,
        cy: 124.0,
        width: 28.0,
        height: 28.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 4,
        short_label: "5",
        cx: 149.0,
        cy: 124.0,
        width: 28.0,
        height: 28.0,
        shape: ElementShape::Circle,
        show_rocker_arrows: false,
    },
];

const SAYO_O2C_LEDS: &[LedZoneLayout] = &[
    LedZoneLayout {
        led_index: 0,
        label: "G1",
        cx: 64.0,
        cy: 54.0,
        width: 58.0,
        height: 58.0,
        shape: ElementShape::Circle,
        primary: true,
    },
    LedZoneLayout {
        led_index: 1,
        label: "G2",
        cx: 146.0,
        cy: 54.0,
        width: 58.0,
        height: 58.0,
        shape: ElementShape::Circle,
        primary: true,
    },
    LedZoneLayout {
        led_index: 0,
        label: "G1",
        cx: 24.0,
        cy: 124.0,
        width: 22.0,
        height: 22.0,
        shape: ElementShape::Circle,
        primary: false,
    },
    LedZoneLayout {
        led_index: 1,
        label: "G2",
        cx: 186.0,
        cy: 124.0,
        width: 22.0,
        height: 22.0,
        shape: ElementShape::Circle,
        primary: false,
    },
];

const SP_E_KEYS: &[KeySlotLayout] = &[
    KeySlotLayout {
        index: 0,
        short_label: "BT1",
        cx: 46.0,
        cy: 46.0,
        width: 50.0,
        height: 50.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 1,
        short_label: "BT2",
        cx: 108.0,
        cy: 46.0,
        width: 50.0,
        height: 50.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 2,
        short_label: "BT3",
        cx: 170.0,
        cy: 46.0,
        width: 50.0,
        height: 50.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 3,
        short_label: "BT4",
        cx: 232.0,
        cy: 46.0,
        width: 50.0,
        height: 50.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 4,
        short_label: "BT5",
        cx: 46.0,
        cy: 108.0,
        width: 50.0,
        height: 50.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 5,
        short_label: "BT6",
        cx: 108.0,
        cy: 108.0,
        width: 50.0,
        height: 50.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 6,
        short_label: "BT7",
        cx: 170.0,
        cy: 108.0,
        width: 50.0,
        height: 50.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
    KeySlotLayout {
        index: 7,
        short_label: "BT8",
        cx: 232.0,
        cy: 108.0,
        width: 50.0,
        height: 50.0,
        shape: ElementShape::RoundedRect,
        show_rocker_arrows: false,
    },
];

pub const NANO_LAYOUT: DeviceLayout = DeviceLayout {
    body_width: 220.0,
    body_height: 112.0,
    corner_radius: 16.0,
    top_reserve: 26.0,
    bottom_reserve: 4.0,
    has_usb_indicator: true,
    keys: NANO_KEYS,
    leds: NANO_LEDS,
};

pub const V2_LAYOUT: DeviceLayout = DeviceLayout {
    body_width: 210.0,
    body_height: 162.0,
    corner_radius: 16.0,
    top_reserve: 4.0,
    bottom_reserve: 4.0,
    has_usb_indicator: false,
    keys: V2_KEYS,
    leds: V2_LEDS,
};

pub const V2_LITE_LAYOUT: DeviceLayout = DeviceLayout {
    body_width: 210.0,
    body_height: 162.0,
    corner_radius: 16.0,
    top_reserve: 4.0,
    bottom_reserve: 4.0,
    has_usb_indicator: false,
    keys: V2_KEYS,
    leds: &[],
};

pub const V2_EX_LAYOUT: DeviceLayout = DeviceLayout {
    body_width: 248.0,
    body_height: 162.0,
    corner_radius: 16.0,
    top_reserve: 4.0,
    bottom_reserve: 4.0,
    has_usb_indicator: false,
    keys: V2_EX_KEYS,
    leds: V2_EX_LEDS,
};

pub const SAYO_O2C_LAYOUT: DeviceLayout = DeviceLayout {
    body_width: 210.0,
    body_height: 158.0,
    corner_radius: 16.0,
    top_reserve: 4.0,
    bottom_reserve: 4.0,
    has_usb_indicator: false,
    keys: SAYO_O2C_KEYS,
    leds: SAYO_O2C_LEDS,
};

pub const SP_E_LAYOUT: DeviceLayout = DeviceLayout {
    body_width: 278.0,
    body_height: 154.0,
    corner_radius: 16.0,
    top_reserve: 4.0,
    bottom_reserve: 4.0,
    has_usb_indicator: false,
    keys: SP_E_KEYS,
    leds: &[],
};

pub const BOOTLOADER_LAYOUT: DeviceLayout = DeviceLayout {
    body_width: 200.0,
    body_height: 100.0,
    corner_radius: 14.0,
    top_reserve: 4.0,
    bottom_reserve: 4.0,
    has_usb_indicator: false,
    keys: &[],
    leds: &[],
};

impl DeviceModel {
    /// Physical vector schematic blueprint for this keypad model.
    pub const fn layout(&self) -> &'static DeviceLayout {
        match self {
            Self::SimpadNano | Self::SimpadNanoYearEdition => &NANO_LAYOUT,
            Self::SimpadV2 | Self::SimpadV2Anniversary => &V2_LAYOUT,
            Self::SimpadV2Lite => &V2_LITE_LAYOUT,
            Self::SimpadV2Ex => &V2_EX_LAYOUT,
            Self::SayoO2C => &SAYO_O2C_LAYOUT,
            Self::SimpadSpE | Self::SimpadSpE2 => &SP_E_LAYOUT,
            Self::Bootloader => &BOOTLOADER_LAYOUT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_models_layout_consistency() {
        let mut models = DeviceModel::all_models().to_vec();
        models.push(DeviceModel::Bootloader);

        for model in models {
            let layout = model.layout();
            assert_eq!(
                layout.keys.len(),
                model.key_count(),
                "Key count mismatch for {model:?}"
            );

            for (expected_idx, slot) in layout.keys.iter().enumerate() {
                assert_eq!(slot.index, expected_idx);
                assert!(slot.width > 0.0 && slot.height > 0.0);
                assert!(slot.cx - slot.width * 0.5 >= 0.0);
                assert!(slot.cx + slot.width * 0.5 <= layout.body_width);
                assert!(slot.cy - slot.height * 0.5 >= 0.0);
                assert!(slot.cy + slot.height * 0.5 <= layout.body_height);
                assert_eq!(
                    layout.hit_test_key(slot.cx, slot.cy),
                    Some(expected_idx),
                    "Center of key {expected_idx} on {model:?} must hit itself"
                );
            }

            // Ensure no two physical key slots overlap
            for (i, a) in layout.keys.iter().enumerate() {
                for b in layout.keys.iter().skip(i + 1) {
                    let dx = (a.cx - b.cx).abs();
                    let dy = (a.cy - b.cy).abs();
                    let sep_x = (a.width + b.width) * 0.5;
                    let sep_y = (a.height + b.height) * 0.5;
                    assert!(
                        dx >= sep_x || dy >= sep_y,
                        "Key slots {} and {} overlap on {model:?}",
                        a.index,
                        b.index
                    );
                }
            }

            if model.led_count() == 0 {
                assert!(
                    layout.leds.is_empty(),
                    "{model:?} has 0 LEDs so led zones must be empty"
                );
            } else {
                assert!(
                    !layout.leds.is_empty(),
                    "{model:?} supports LEDs so led zones cannot be empty"
                );
                for zone in layout.leds {
                    assert!(
                        zone.led_index < model.led_count(),
                        "LED zone index {} out of range for {model:?}",
                        zone.led_index
                    );
                    assert!(zone.width > 0.0 && zone.height > 0.0);
                }
            }
        }
    }

    #[test]
    fn test_canvas_transform_roundtrip_and_hit_testing() {
        let layout = DeviceModel::SimpadV2Ex.layout();
        let tx = CanvasTransform::fit(320.0, 200.0, layout, 12.0);
        assert!(tx.scale > 0.0);

        // Rocker BT6 at index 5
        let bt6 = &layout.keys[5];
        let sx = tx.to_screen_x(bt6.cx);
        let sy = tx.to_screen_y(bt6.cy);
        let (bx, by) = tx.to_blueprint(sx, sy);
        assert_eq!(layout.hit_test_key(bx, by), Some(5));

        // Outside the right-side semi-ellipse corner of BT6
        assert!(!bt6.contains(bt6.cx + bt6.width * 0.48, bt6.cy + bt6.height * 0.48));
        // Outside left edge of rocker flat chord
        assert!(!bt6.contains(bt6.cx - bt6.width * 0.55, bt6.cy));
        // Outside overall keypad bounds
        assert_eq!(layout.hit_test_key(-5.0, -5.0), None);
    }
}
