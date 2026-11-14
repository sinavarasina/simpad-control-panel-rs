use crate::protocol::packet::Packet;

/// High-level commands supported by the SimPad protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Read setting value from a pointer (1..11).
    ReadSetting { pointer: u8 },

    /// Write 4 bytes of data to a setting pointer (1..11).
    WriteSetting { pointer: u8, data: [u8; 4] },

    /// Query the firmware version.
    GetVersion,

    /// Query the microcontroller chip ID.
    GetChipId,

    /// Enter the IAP / Bootloader mode.
    EnterBootloader,

    /// Arbitrary raw 8-byte packet (for advanced debugging).
    Raw([u8; 8]),
}

impl Command {
    /// Convert the command into a raw 8-byte packet ready for transmission.
    pub fn to_packet(&self) -> Packet {
        match self {
            Self::ReadSetting { pointer } => Packet::query_setting(*pointer),
            Self::WriteSetting { pointer, data } => Packet::write_setting(*pointer, *data),
            Self::GetVersion => Packet::query_version(),
            Self::GetChipId => Packet::query_chip_id(),
            Self::EnterBootloader => Packet::enter_bootloader(),
            Self::Raw(bytes) => Packet::new(*bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_to_packet() {
        let cmd = Command::ReadSetting { pointer: 3 };
        assert_eq!(
            cmd.to_packet().to_bytes(),
            [0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00]
        );

        let cmd = Command::WriteSetting {
            pointer: 2,
            data: [1, 2, 4, 8],
        };
        assert_eq!(cmd.to_packet().to_bytes()[0], 2);
        assert_eq!(cmd.to_packet().to_bytes()[5], 1 ^ 2 ^ 4 ^ 8);
    }
}
