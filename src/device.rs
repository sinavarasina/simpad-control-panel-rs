pub mod connection;
pub mod detector;
pub mod layout;
pub mod model;

pub use connection::{DeviceConnection, HidConnection, MockConnection};
pub use detector::{
    DeviceInfo, DeviceSelectionPlan, KNOWN_VENDOR_IDS, enumerate_devices, plan_device_selection,
    refresh_and_enumerate_devices,
};
pub use layout::{CanvasTransform, DeviceLayout, ElementShape, KeySlotLayout, LedZoneLayout};
pub use model::{DeviceCapabilities, DeviceModel};
