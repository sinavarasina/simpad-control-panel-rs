# SimPad Control Panel

Cross-platform Linux and Windows USB HID configurator for SimPad keypads, written in Rust with Relm4, GTK4, and Libadwaita.

## Supported Devices

| Model | VID:PID | Keys | LEDs |
|---|---|---|---|
| SimPad v2 | `8088:0001` | 5 | 2 |
| SimPad v2 EX | `8088:0002` | 6 | 2 |
| SimPad v2 Lite | `8088:0003` | 5 | 0 |
| SimPad Nano | `8088:0004` | 3 | 2 |
| SimPad v2 Anniversary Edition | `8088:0006` | 5 | 2 |
| SimPad Nano Year Edition | `8088:0007` | 3 | 2 |
| SimPad Special Edition 8 Keys (SP-E2) | `8088:0102` | 8 | 0 |
| SimPad Special Edition 8 Keys (SP-E) | `8088:0302` | 8 | 0 |
| Sayo's KeyPad O2C | `8089:0003` | 5 | 2 |
| SimPad Bootloader / IAP | `8088:00FF` | 0 | 0 |

## Requirements

### Linux

```bash
# Arch Linux
sudo pacman -S gtk4 libadwaita

# Fedora
sudo dnf install gtk4-devel libadwaita-devel

# Ubuntu / Debian
sudo apt install libgtk-4-dev libadwaita-1-dev
```

Install udev rules for non-root HID access:

```bash
sudo cp udev/99-simpad.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Windows (MSYS2 UCRT64)

```bash
pacman -S mingw-w64-ucrt-x86_64-gtk4 mingw-w64-ucrt-x86_64-libadwaita mingw-w64-ucrt-x86_64-pkgconf mingw-w64-ucrt-x86_64-rust
```

## Build & Run

```bash
cargo test
cargo run
```

Build a release binary:

```bash
cargo build --release
```

## Project Layout

```text
├── udev/99-simpad.rules
└── src/
    ├── lib.rs
    ├── main.rs
    ├── error.rs
    ├── config.rs        # Keycodes, modifiers, mouse, lighting, settings diff
    ├── device.rs        # Models, capabilities, HID detection & connection
    ├── protocol.rs      # 8-byte HID packets, checksum & commands
    └── ui.rs            # Relm4 views & platform styling
```

## License

[Apache-2.0](LICENSE)
