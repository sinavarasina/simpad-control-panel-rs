use crate::device::connection::HidConnection;
use crate::device::model::DeviceModel;
use crate::error::{Result, SimpadError};

/// Information about a detected SimPad hardware device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub path: String,
    pub model: DeviceModel,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
    pub interface_number: i32,
}

impl DeviceInfo {
    /// Open a connection to this physical HID device.
    pub fn open(&self, api: &hidapi::HidApi) -> Result<HidConnection> {
        let c_path = std::ffi::CString::new(self.path.as_bytes())
            .map_err(|_| SimpadError::NoDeviceConnected)?;
        let device = api.open_path(&c_path)?;
        Ok(HidConnection::new(device))
    }
}

/// Determine whether a HID device descriptor corresponds to the SimPad
/// vendor control endpoint (`Interface 1` / Windows `&mi_01`).
pub fn is_control_interface(interface_number: i32, path: &str, usage: u16) -> bool {
    if interface_number >= 0 {
        return interface_number == 1;
    }

    let lower = path.to_ascii_lowercase();
    if lower.contains("&mi_01") {
        return true;
    }
    if lower.contains("&mi_00") || lower.contains("&mi_02") || lower.contains("&mi_03") {
        return false;
    }

    // Fallback (e.g., macOS/platforms where interface_number == -1 and no MI_xx tag):
    // exclude standard USB HID keyboard (0x0006) and mouse (0x0002) usages.
    usage != 0x0006 && usage != 0x0002
}

/// Supported USB Vendor IDs for all recognized SimPad and compatible devices.
pub const KNOWN_VENDOR_IDS: &[u16] = &[0x8088, 0x8089];

/// Outcome of reconciling a cached device list against a fresh HID scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceSelectionPlan {
    /// Index in the freshly scanned list to select (`0` when empty).
    pub selected_idx: usize,
    /// Whether the enumerated list changed compared to the previous snapshot.
    pub list_changed: bool,
    /// Whether the active HID connection should be closed, switched, or opened.
    pub connection_changed: bool,
}

/// Detect and enumerate all connected SimPad devices currently indexed in `api`.
pub fn enumerate_devices(api: &hidapi::HidApi) -> Vec<DeviceInfo> {
    let mut list = Vec::new();

    for dev in api.device_list() {
        if let Some(model) = DeviceModel::from_vid_pid(dev.vendor_id(), dev.product_id()) {
            let path = dev.path().to_string_lossy().into_owned();
            let accepted = is_control_interface(dev.interface_number(), &path, dev.usage())
                || (model.is_bootloader() && dev.interface_number() <= 0);
            if accepted {
                list.push(DeviceInfo {
                    path,
                    model,
                    vendor_id: dev.vendor_id(),
                    product_id: dev.product_id(),
                    serial_number: dev.serial_number().map(String::from),
                    interface_number: dev.interface_number(),
                });
            }
        }
    }

    list.sort_by(|a, b| a.path.cmp(&b.path));
    list
}

/// Refresh `api`'s OS HID cache for all known SimPad vendor IDs and return
/// the currently connected control interfaces.
pub fn refresh_and_enumerate_devices(api: &mut hidapi::HidApi) -> Result<Vec<DeviceInfo>> {
    // Re-query only SimPad/compatible VIDs so unrelated peripherals do not have
    // their descriptors probed during periodic hotplug checks.
    api.reset_devices()?;
    for &vid in KNOWN_VENDOR_IDS {
        api.add_devices(vid, 0)?;
    }
    Ok(enumerate_devices(api))
}

/// Determine how to update device selection and connection state after a fresh scan.
///
/// Preserves the active device by endpoint identity while it stays attached so
/// unrelated hotplug events do not interrupt an open session or drop pending edits.
pub fn plan_device_selection(
    prev: &[DeviceInfo],
    prev_idx: usize,
    is_connected: bool,
    fresh: &[DeviceInfo],
) -> DeviceSelectionPlan {
    let list_changed = prev != fresh;
    if fresh.is_empty() {
        return DeviceSelectionPlan {
            selected_idx: 0,
            list_changed,
            connection_changed: is_connected,
        };
    }

    let retained_idx = prev
        .get(prev_idx)
        .and_then(|active| fresh.iter().position(|candidate| candidate == active));

    if is_connected {
        match retained_idx {
            Some(idx) => DeviceSelectionPlan {
                selected_idx: idx,
                list_changed,
                connection_changed: false,
            },
            None => DeviceSelectionPlan {
                selected_idx: 0,
                list_changed,
                connection_changed: true,
            },
        }
    } else {
        DeviceSelectionPlan {
            selected_idx: retained_idx.unwrap_or(0),
            list_changed,
            connection_changed: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_info(path: &str, model: DeviceModel) -> DeviceInfo {
        let (vendor_id, product_id) = model.vid_pid();
        DeviceInfo {
            path: path.to_string(),
            model,
            vendor_id,
            product_id,
            serial_number: None,
            interface_number: 1,
        }
    }

    #[test]
    fn test_enumerate_does_not_panic() {
        if let Ok(mut api) = hidapi::HidApi::new() {
            let _ = enumerate_devices(&api);
            let _ = refresh_and_enumerate_devices(&mut api);
        }
    }

    #[test]
    fn test_known_vendor_ids_cover_all_models() {
        let all_models = [
            DeviceModel::SimpadV2,
            DeviceModel::SimpadV2Ex,
            DeviceModel::SimpadV2Lite,
            DeviceModel::SimpadNano,
            DeviceModel::SimpadV2Anniversary,
            DeviceModel::SimpadNanoYearEdition,
            DeviceModel::SimpadSpE,
            DeviceModel::SimpadSpE2,
            DeviceModel::SayoO2C,
            DeviceModel::Bootloader,
        ];
        for model in all_models {
            let (vid, _) = model.vid_pid();
            assert!(
                KNOWN_VENDOR_IDS.contains(&vid),
                "Model {model:?} uses VID {vid:#06x} missing from KNOWN_VENDOR_IDS"
            );
        }
    }

    #[test]
    fn test_plan_device_selection_hotplug_transitions() {
        let d1 = mock_info("/dev/hidraw3", DeviceModel::SimpadNano);
        let d1_replugged = mock_info("/dev/hidraw4", DeviceModel::SimpadNano);
        let d2 = mock_info("/dev/hidraw1", DeviceModel::SimpadV2);

        // No device -> still no device: pure no-op
        assert_eq!(
            plan_device_selection(&[], 0, false, &[]),
            DeviceSelectionPlan {
                selected_idx: 0,
                list_changed: false,
                connection_changed: false,
            }
        );

        // Started empty -> plugged in D1: connects to D1
        assert_eq!(
            plan_device_selection(&[], 0, false, std::slice::from_ref(&d1)),
            DeviceSelectionPlan {
                selected_idx: 0,
                list_changed: true,
                connection_changed: true,
            }
        );

        // Connected to D1, secondary D2 plugged ahead in sort order: keep D1 open at idx 1
        assert_eq!(
            plan_device_selection(
                std::slice::from_ref(&d1),
                0,
                true,
                &[d2.clone(), d1.clone()]
            ),
            DeviceSelectionPlan {
                selected_idx: 1,
                list_changed: true,
                connection_changed: false,
            }
        );

        // Connected to D1, unplugged: drop connection and clear state
        assert_eq!(
            plan_device_selection(std::slice::from_ref(&d1), 0, true, &[]),
            DeviceSelectionPlan {
                selected_idx: 0,
                list_changed: true,
                connection_changed: true,
            }
        );

        // Connected on /dev/hidraw3, quick replug moves node to /dev/hidraw4: reconnect cleanly
        assert_eq!(
            plan_device_selection(&[d1], 0, true, &[d1_replugged]),
            DeviceSelectionPlan {
                selected_idx: 0,
                list_changed: true,
                connection_changed: true,
            }
        );
    }

    #[test]
    fn test_is_control_interface_linux_and_windows() {
        // Linux USB HID interface numbers
        assert!(is_control_interface(1, "/dev/hidraw8", 0));
        assert!(!is_control_interface(0, "/dev/hidraw7", 0x0006));
        assert!(!is_control_interface(2, "/dev/hidraw9", 0x0002));

        // Windows HID paths when interface_number is -1
        assert!(is_control_interface(
            -1,
            r"\\?\HID#VID_8088&PID_0007&MI_01#7&1234",
            0x0001
        ));
        assert!(!is_control_interface(
            -1,
            r"\\?\HID#VID_8088&PID_0007&MI_00#7&1234",
            0x0006
        ));
        assert!(!is_control_interface(
            -1,
            r"\\?\HID#VID_8088&PID_0007&MI_02#7&1234",
            0x0002
        ));

        // Generic platform fallback excluding keyboard/mouse usages
        assert!(is_control_interface(-1, "IOService:/custom", 0x0000));
        assert!(!is_control_interface(-1, "IOService:/kbd", 0x0006));
        assert!(!is_control_interface(-1, "IOService:/mouse", 0x0002));
    }
}
