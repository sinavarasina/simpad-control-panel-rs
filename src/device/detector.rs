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

/// Detect and enumerate all connected SimPad devices.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_does_not_panic() {
        if let Ok(api) = hidapi::HidApi::new() {
            let _ = enumerate_devices(&api);
        }
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
