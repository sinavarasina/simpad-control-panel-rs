pub mod connection;
pub mod detector;
pub mod model;

pub use connection::{DeviceConnection, HidConnection, MockConnection};
pub use detector::{DeviceInfo, enumerate_devices};
pub use model::{DeviceCapabilities, DeviceModel};
