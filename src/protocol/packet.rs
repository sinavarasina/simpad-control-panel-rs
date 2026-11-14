use crate::error::{Result, SimpadError};

/// The fixed size of a SimPad HID report packet.
pub const PACKET_SIZE: usize = 8;

/// A raw 8-byte SimPad packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packet(pub [u8; PACKET_SIZE]);

impl Packet {
    /// Create a new packet from a raw 8-byte array.
    pub const fn new(bytes: [u8; PACKET_SIZE]) -> Self {
        Self(bytes)
    }

    /// Create a zero-filled packet.
    pub const fn zero() -> Self {
        Self([0u8; PACKET_SIZE])
    }

    /// Returns a slice of the packet's bytes.
    pub fn as_bytes(&self) -> &[u8; PACKET_SIZE] {
        &self.0
    }

    /// Returns the underlying byte array.
    pub const fn to_bytes(self) -> [u8; PACKET_SIZE] {
        self.0
    }

    /// Calculate the XOR checksum of 4 bytes.
    pub const fn calculate_checksum(b0: u8, b1: u8, b2: u8, b3: u8) -> u8 {
        b0 ^ b1 ^ b2 ^ b3
    }

    /// Build a Write Setting packet.
    ///
    /// Layout:
    /// - `[0]`: pointer (setting index 1..11)
    /// - `[1..5]`: data bytes (4 bytes)
    /// - `[5]`: XOR checksum of data bytes
    /// - `[6..8]`: 0x00, 0x00
    pub fn write_setting(pointer: u8, data: [u8; 4]) -> Self {
        let checksum = Self::calculate_checksum(data[0], data[1], data[2], data[3]);
        Self([
            pointer, data[0], data[1], data[2], data[3], checksum, 0x00, 0x00,
        ])
    }

    /// Build a Query Setting packet.
    ///
    /// Layout: `[0x00, 0x01, pointer, 0x00, 0x00, 0x00, 0x00, 0x00]`
    pub fn query_setting(pointer: u8) -> Self {
        Self([0x00, 0x01, pointer, 0x00, 0x00, 0x00, 0x00, 0x00])
    }

    /// Build a Query Version packet.
    ///
    /// Layout: `[0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
    pub fn query_version() -> Self {
        Self([0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
    }

    /// Build a Query Chip ID packet.
    ///
    /// Layout: `[0x00, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]`
    pub fn query_chip_id() -> Self {
        Self([0x00, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00])
    }

    /// Build an Enter Bootloader packet.
    ///
    /// Layout: `[0x00, 0x0b, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00]`
    pub fn enter_bootloader() -> Self {
        Self([0x00, 0x0b, 0x00, 0x00, 0x00, 0x0b, 0x00, 0x00])
    }

    /// Parse a received response for a setting query.
    ///
    /// Response layout:
    /// - `[0..4]`: 4 data bytes
    /// - `[4]`: pointer
    /// - `[5..8]`: padding (zeros)
    pub fn parse_setting_response(&self) -> Result<(u8, [u8; 4])> {
        let data = [self.0[0], self.0[1], self.0[2], self.0[3]];
        let pointer = self.0[4];
        Ok((pointer, data))
    }

    /// Parse a received response for firmware version.
    ///
    /// Version layout: 4 bytes: `[yy_hi, yy_lo, mm, dd]` e.g. `[20, 18, 9, 26]` -> 20180926.
    pub fn parse_version_response(&self) -> u32 {
        let y_hi = self.0[0] as u32;
        let y_lo = self.0[1] as u32;
        let mm = self.0[2] as u32;
        let dd = self.0[3] as u32;

        let year = y_hi * 100 + y_lo;
        year * 10000 + mm * 100 + dd
    }

    /// Parse the 4-byte Chip ID response into an uppercase hex string.
    pub fn parse_chip_id_response(&self) -> String {
        format!(
            "{:02X}{:02X}{:02X}{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = SimpadError;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        if bytes.len() == PACKET_SIZE + 1 && bytes[0] == 0x00 {
            let mut arr = [0u8; PACKET_SIZE];
            arr.copy_from_slice(&bytes[1..PACKET_SIZE + 1]);
            return Ok(Self(arr));
        }
        if bytes.len() < PACKET_SIZE {
            return Err(SimpadError::InvalidPacketLength(bytes.len()));
        }
        let mut arr = [0u8; PACKET_SIZE];
        arr.copy_from_slice(&bytes[..PACKET_SIZE]);
        Ok(Self(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_calculation() {
        let cs = Packet::calculate_checksum(0x01, 0x02, 0x04, 0x08);
        assert_eq!(cs, 0x01 ^ 0x02 ^ 0x04 ^ 0x08);
    }

    #[test]
    fn test_write_setting_packet() {
        let p = Packet::write_setting(1, [0x00, 0x1d, 0x00, 0x00]);
        assert_eq!(p.0[0], 1);
        assert_eq!(p.0[1], 0x00);
        assert_eq!(p.0[2], 0x1d);
        assert_eq!(p.0[3], 0x00);
        assert_eq!(p.0[4], 0x00);
        assert_eq!(p.0[5], 0x1d); // 0 ^ 0x1d ^ 0 ^ 0
        assert_eq!(p.0[6], 0x00);
        assert_eq!(p.0[7], 0x00);
    }

    #[test]
    fn test_query_setting_packet() {
        let p = Packet::query_setting(6);
        assert_eq!(p.0, [0x00, 0x01, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_parse_setting_response() {
        let raw = [0x00, 0xff, 0x00, 0x04, 0x06, 0x00, 0x00, 0x00];
        let p = Packet::new(raw);
        let (pointer, data) = p.parse_setting_response().expect("parse success");
        assert_eq!(pointer, 6);
        assert_eq!(data, [0x00, 0xff, 0x00, 0x04]);
    }

    #[test]
    fn test_parse_version() {
        let raw = [20, 18, 9, 26, 0, 0, 0, 0];
        let p = Packet::new(raw);
        assert_eq!(p.parse_version_response(), 20180926);
    }

    #[test]
    fn test_query_and_parse_chip_id() {
        let q = Packet::query_chip_id();
        assert_eq!(q.0, [0x00, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let resp = Packet::new([0x12, 0xab, 0x00, 0xef, 0, 0, 0, 0]);
        assert_eq!(resp.parse_chip_id_response(), "12AB00EF");
    }

    #[test]
    fn test_try_from_8_byte_and_9_byte_windows_report() {
        let raw_8 = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let p8 = Packet::try_from(raw_8.as_slice()).expect("8-byte packet");
        assert_eq!(p8.0, raw_8);

        let raw_9 = [0x00, 0x11, 0x22, 0x33, 0x44, 0x06, 0x00, 0x00, 0x00];
        let p9 = Packet::try_from(raw_9.as_slice()).expect("9-byte prefixed packet");
        assert_eq!(p9.0, [0x11, 0x22, 0x33, 0x44, 0x06, 0x00, 0x00, 0x00]);
    }
}
