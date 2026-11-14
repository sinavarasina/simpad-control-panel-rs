use thiserror::Error;

/// Errors that can occur during SimPad operations.
#[derive(Debug, Error)]
pub enum SimpadError {
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    #[error("Invalid packet length: expected 8 bytes, got {0}")]
    InvalidPacketLength(usize),

    #[error("Checksum mismatch: expected {expected:#04x}, got {actual:#04x}")]
    ChecksumMismatch { expected: u8, actual: u8 },

    #[error("Device not found: VID {vendor_id:#06x}, PID {product_id:#06x}")]
    DeviceNotFound { vendor_id: u16, product_id: u16 },

    #[error("No SimPad device connected")]
    NoDeviceConnected,

    #[error("Timeout waiting for response for pointer {pointer}")]
    Timeout { pointer: u8 },

    #[error("Invalid keycode: {0}")]
    InvalidKeyCode(u8),

    #[error("Invalid light mode index: {0}")]
    InvalidLightMode(u8),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SimpadError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SimpadError::ChecksumMismatch {
            expected: 0xaa,
            actual: 0xbb,
        };
        assert_eq!(
            err.to_string(),
            "Checksum mismatch: expected 0xaa, got 0xbb"
        );
    }
}
