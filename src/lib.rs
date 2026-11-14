pub mod config;
pub mod device;
pub mod error;
pub mod protocol;
pub mod ui;

pub use config::{
    Brightness, DeviceSettings, KeyBinding, KeyCode, LedConfig, LightMode, LightSpeed,
    ModifierFlags, MouseButtonFlags, RgbColor,
};
pub use device::{
    DeviceConnection, DeviceInfo, DeviceModel, HidConnection, MockConnection, enumerate_devices,
};
pub use error::{Result, SimpadError};
pub use protocol::{Command, PACKET_SIZE, Packet};
