pub mod connection;
pub mod detector;
pub mod layout;
pub mod model;

pub use connection::{DeviceConnection, HidConnection, MockConnection};
pub use detector::{DeviceInfo, enumerate_devices};
pub use layout::{CanvasTransform, DeviceLayout, ElementShape, KeySlotLayout, LedZoneLayout};
pub use model::{DeviceCapabilities, DeviceModel};
