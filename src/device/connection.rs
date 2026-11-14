use std::thread::sleep;
use std::time::Duration;

use crate::config::device_settings::DeviceSettings;
use crate::device::model::DeviceModel;
use crate::error::{Result, SimpadError};
use crate::protocol::packet::{PACKET_SIZE, Packet};

/// Abstraction for communicating with a SimPad device over HID or in tests.
pub trait DeviceConnection: Send {
    /// Send an 8-byte packet to the device.
    fn send_packet(&mut self, packet: &Packet) -> Result<()>;

    /// Read an 8-byte packet from the device with a timeout in milliseconds.
    fn read_packet(&mut self, timeout_ms: i32) -> Result<Packet>;

    /// Drain any stale buffered packets from the HID receive queue.
    fn flush_input(&mut self) {}

    /// Query and read a specific setting pointer (1..11) with retry logic.
    fn read_setting(&mut self, pointer: u8, retries: usize) -> Result<[u8; 4]> {
        let query = Packet::query_setting(pointer);

        for _ in 0..retries {
            self.flush_input();
            self.send_packet(&query)?;
            sleep(Duration::from_millis(15));

            for _ in 0..2 {
                if let Ok(packet) = self.read_packet(100)
                    && let Ok((resp_pointer, data)) = packet.parse_setting_response()
                    && resp_pointer == pointer
                {
                    return Ok(data);
                }
            }
        }

        Err(SimpadError::Timeout { pointer })
    }

    /// Write 4 bytes to a specific setting pointer with automatic checksum.
    fn write_setting(&mut self, pointer: u8, data: [u8; 4]) -> Result<()> {
        let packet = Packet::write_setting(pointer, data);
        self.send_packet(&packet)?;
        sleep(Duration::from_millis(25));
        Ok(())
    }

    /// Query the firmware version number (e.g. 20180926).
    fn get_firmware_version(&mut self) -> Result<u32> {
        self.flush_input();
        let query = Packet::query_version();
        self.send_packet(&query)?;
        sleep(Duration::from_millis(15));

        let response = self.read_packet(150)?;
        Ok(response.parse_version_response())
    }

    /// Query the microcontroller chip ID (4-byte uppercase hex string).
    fn get_chip_id(&mut self) -> Result<String> {
        self.flush_input();
        let query = Packet::query_chip_id();
        self.send_packet(&query)?;
        sleep(Duration::from_millis(15));

        let response = self.read_packet(150)?;
        Ok(response.parse_chip_id_response())
    }

    /// Read all supported configuration settings for a specific `DeviceModel`.
    fn read_all_settings_for_model(&mut self, model: DeviceModel) -> Result<DeviceSettings> {
        let mut settings = DeviceSettings::default_for_model(model);
        if model.is_bootloader() {
            return Ok(settings);
        }

        for pointer in 1..=11 {
            if settings.get_raw_setting(pointer).is_none() {
                continue;
            }
            if let Ok(data) = self.read_setting(pointer, 3) {
                settings.apply_raw_setting(pointer, data);
            }
        }

        Ok(settings)
    }

    /// Read all configuration settings from the device into a DeviceSettings struct.
    fn read_all_settings(&mut self, key_count: usize, _led_count: usize) -> Result<DeviceSettings> {
        let model = match key_count {
            3 => DeviceModel::SimpadNano,
            6 => DeviceModel::SimpadV2Ex,
            8 => DeviceModel::SimpadSpE,
            0 => DeviceModel::Bootloader,
            _ => DeviceModel::SimpadV2,
        };
        self.read_all_settings_for_model(model)
    }

    /// Send a series of changed settings to the device.
    fn apply_changes(&mut self, diff: &[(u8, [u8; 4])]) -> Result<()> {
        for (pointer, data) in diff {
            self.write_setting(*pointer, *data)?;
        }
        Ok(())
    }
}

/// Physical HID connection wrapping a `hidapi::HidDevice`.
pub struct HidConnection {
    device: hidapi::HidDevice,
}

impl HidConnection {
    pub fn new(device: hidapi::HidDevice) -> Self {
        Self { device }
    }
}

impl DeviceConnection for HidConnection {
    fn send_packet(&mut self, packet: &Packet) -> Result<()> {
        // hidapi write() requires the Report ID at byte 0.
        // For unnumbered HID reports, Report ID is 0x00, followed by the 8-byte payload.
        let mut buf = [0u8; PACKET_SIZE + 1];
        buf[0] = 0x00;
        buf[1..].copy_from_slice(packet.as_bytes());

        let written = self.device.write(&buf)?;
        if written < PACKET_SIZE {
            return Err(SimpadError::InvalidPacketLength(written));
        }
        Ok(())
    }

    fn read_packet(&mut self, timeout_ms: i32) -> Result<Packet> {
        let mut buf = [0u8; 64];
        let bytes_read = self.device.read_timeout(&mut buf, timeout_ms)?;
        if bytes_read == 0 {
            return Err(SimpadError::Timeout { pointer: 0 });
        }
        Packet::try_from(&buf[..bytes_read])
    }

    fn flush_input(&mut self) {
        let mut buf = [0u8; 64];
        for _ in 0..16 {
            match self.device.read_timeout(&mut buf, 1) {
                Ok(0) | Err(_) => break,
                Ok(_) => continue,
            }
        }
    }
}

/// Mock connection used for unit tests and offline testing.
#[derive(Debug, Clone)]
pub struct MockConnection {
    pub settings: DeviceSettings,
    pub version: u32,
    pub chip_id: [u8; 4],
    pub sent_packets: Vec<Packet>,
}

impl Default for MockConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl MockConnection {
    pub fn new() -> Self {
        Self::for_model(DeviceModel::SimpadV2)
    }

    pub fn for_model(model: DeviceModel) -> Self {
        Self {
            settings: DeviceSettings::default_for_model(model),
            version: 20261112,
            chip_id: [0x20, 0x20, 0xab, 0xcd],
            sent_packets: Vec::new(),
        }
    }
}

impl DeviceConnection for MockConnection {
    fn send_packet(&mut self, packet: &Packet) -> Result<()> {
        self.sent_packets.push(*packet);

        // If it's a write setting packet (pointer 1..11 at index 0)
        let ptr = packet.0[0];
        if (1..=11).contains(&ptr) {
            let data = [packet.0[1], packet.0[2], packet.0[3], packet.0[4]];
            self.settings.apply_raw_setting(ptr, data);
        }

        Ok(())
    }

    fn read_packet(&mut self, _timeout_ms: i32) -> Result<Packet> {
        let last = self
            .sent_packets
            .last()
            .ok_or(SimpadError::Timeout { pointer: 0 })?;

        // Query setting command: [0x00, 0x01, pointer, ...]
        if last.0[0] == 0x00 && last.0[1] == 0x01 {
            let pointer = last.0[2];
            let data = self.settings.get_raw_setting(pointer).unwrap_or([0; 4]);
            let cs = Packet::calculate_checksum(data[0], data[1], data[2], data[3]);
            return Ok(Packet::new([
                data[0], data[1], data[2], data[3], pointer, cs, 0, 0,
            ]));
        }

        // Query version / chip id command: [0x00, 0x02, subcmd, ...]
        if last.0[0] == 0x00 && last.0[1] == 0x02 {
            if last.0[2] == 0x01 {
                return Ok(Packet::new([
                    self.chip_id[0],
                    self.chip_id[1],
                    self.chip_id[2],
                    self.chip_id[3],
                    0,
                    0,
                    0,
                    0,
                ]));
            }
            let y_hi = (self.version / 1000000) as u8;
            let y_lo = ((self.version / 10000) % 100) as u8;
            let mm = ((self.version / 100) % 100) as u8;
            let dd = (self.version % 100) as u8;
            return Ok(Packet::new([y_hi, y_lo, mm, dd, 0, 0, 0, 0]));
        }

        Ok(Packet::zero())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KeyBinding, KeyCode, LightMode};

    #[test]
    fn test_mock_read_and_write_setting() {
        let mut mock = MockConnection::new();
        let val = mock.read_setting(1, 1).expect("read setting 1");
        assert_eq!(val[1], 0x1d); // Default 'Z' key

        // Write new key: 'A' (0x04)
        mock.write_setting(1, [0x00, 0x04, 0x00, 0x00])
            .expect("write setting 1");
        let updated = mock.read_setting(1, 1).expect("read updated setting");
        assert_eq!(updated[1], 0x04);
    }

    #[test]
    fn test_mock_get_version_and_chip_id() {
        let mut mock = MockConnection::for_model(DeviceModel::SimpadNanoYearEdition);
        mock.version = 20200702;
        mock.chip_id = [0xde, 0xad, 0xbe, 0xef];
        assert_eq!(mock.get_firmware_version().expect("get version"), 20200702);
        assert_eq!(mock.get_chip_id().expect("get chip id"), "DEADBEEF");
    }

    #[test]
    fn test_mock_nano_and_v2ex_roundtrip() {
        let mut nano = MockConnection::for_model(DeviceModel::SimpadNanoYearEdition);
        let mut target = nano.settings.clone();
        target.keys[2] = KeyBinding::from_key(KeyCode::SPACE);
        target.light_mode = LightMode::RainbowEaseOut;
        target.night_light = false;

        let diff = nano.settings.diff(&target);
        nano.apply_changes(&diff).expect("apply nano diff");

        let reloaded = nano
            .read_all_settings_for_model(DeviceModel::SimpadNanoYearEdition)
            .expect("reload nano");
        assert_eq!(reloaded.keys.len(), 3);
        assert_eq!(reloaded.keys[2].key, KeyCode::SPACE);
        assert_eq!(reloaded.light_mode, LightMode::RainbowEaseOut);
        assert!(!reloaded.night_light);

        let mut v2ex = MockConnection::for_model(DeviceModel::SimpadV2Ex);
        let mut v2ex_target = v2ex.settings.clone();
        v2ex_target.keys[5] = KeyBinding::from_key(KeyCode::F12);
        let diff = v2ex.settings.diff(&v2ex_target);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].0, 11);
        v2ex.apply_changes(&diff).expect("apply v2ex diff");

        let reloaded_v2ex = v2ex
            .read_all_settings_for_model(DeviceModel::SimpadV2Ex)
            .expect("reload v2ex");
        assert_eq!(reloaded_v2ex.keys.len(), 6);
        assert_eq!(reloaded_v2ex.keys[5].key, KeyCode::F12);
    }

    #[test]
    fn test_mock_apply_changes() {
        let mut mock = MockConnection::new();
        let diff = vec![
            (1, [0x01, 0x15, 0x00, 0x00]), // Ctrl+R
            (8, [0x02, 0x00, 0x00, 0x00]), // KeepOn mode
        ];
        mock.apply_changes(&diff).expect("apply changes");
        assert_eq!(mock.read_setting(8, 1).unwrap()[0], 0x02);
    }
}
