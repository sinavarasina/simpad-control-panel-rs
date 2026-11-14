use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

use crate::device::DeviceCapabilities;
use crate::protocol::packet::Packet;

pub struct AdvancedViewState {
    pub capabilities: DeviceCapabilities,
    pub debounce_delay: u32,
    pub full_speed_mode: bool,
    pub raw_packet_log: String,
    raw_hex_buffer: gtk::EntryBuffer,
}

#[derive(Debug)]
pub enum AdvancedViewMsg {
    Sync {
        capabilities: DeviceCapabilities,
        debounce_delay: u32,
        full_speed_mode: bool,
    },
    SetDebounce(u32),
    ToggleFullSpeed(bool),
    FixRawChecksum,
    SendRawPacket,
    RawPacketResult(String),
    AskFactoryReset,
    AskEnterBootloader,
    ConfirmFactoryReset,
    ConfirmEnterBootloader,
}

#[derive(Debug)]
pub enum AdvancedViewOutput {
    DebounceChanged(u32),
    FullSpeedChanged(bool),
    SendRaw([u8; 8]),
    FactoryResetRequested,
    BootloaderRequested,
}

impl AdvancedViewState {
    fn parse_hex_bytes(input: &str) -> Result<[u8; 8], &'static str> {
        let cleaned = input.replace([',', ':'], " ");
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        if parts.len() != 8 {
            return Err("Expected exactly 8 space-separated hex bytes.");
        }
        let mut bytes = [0u8; 8];
        for (i, part) in parts.iter().enumerate() {
            let trimmed = part.trim_start_matches("0x").trim_start_matches("0X");
            bytes[i] = u8::from_str_radix(trimmed, 16)
                .map_err(|_| "Invalid hexadecimal byte in packet input.")?;
        }
        Ok(bytes)
    }

    fn format_hex_bytes(bytes: &[u8; 8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn show_confirm_dialog(
        root: &gtk::Box,
        title: &str,
        detail: &str,
        confirm_label: &str,
        on_confirm: AdvancedViewMsg,
        sender: ComponentSender<Self>,
    ) {
        let dialog = gtk::AlertDialog::builder()
            .message(title)
            .detail(detail)
            .buttons(["Cancel", confirm_label])
            .cancel_button(0)
            .default_button(0)
            .modal(true)
            .build();

        let parent = root.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        dialog.choose(parent.as_ref(), gtk::gio::Cancellable::NONE, move |res| {
            if matches!(res, Ok(1)) {
                sender.input(on_confirm);
            }
        });
    }
}

#[relm4::component(pub)]
impl Component for AdvancedViewState {
    type Init = (DeviceCapabilities, u32, bool);
    type Input = AdvancedViewMsg;
    type Output = AdvancedViewOutput;
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_margin_all: 16,

            gtk::Label {
                set_halign: gtk::Align::Start,
                add_css_class: "title-4",
                set_text: "Advanced Hardware Settings",
            },

            gtk::ListBox {
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
                #[watch]
                set_visible: model.capabilities.has_debounce || model.capabilities.has_full_speed,

                adw::ActionRow {
                    set_title: "Input Debounce Delay",
                    set_subtitle: "Anti-jitter filter delay ticks (range 1 – 65535, default 96).",
                    #[watch]
                    set_visible: model.capabilities.has_debounce,

                    #[name = "debounce_spin"]
                    add_suffix = &gtk::SpinButton::with_range(1.0, 65535.0, 1.0) {
                        set_valign: gtk::Align::Center,
                        set_value: model.debounce_delay as f64,
                        #[track(
                            skip_init,
                            (debounce_spin.value().round() as u32) != model.debounce_delay
                        )]
                        #[block_signal(debounce_handler)]
                        set_value: model.debounce_delay as f64,

                        connect_value_changed[sender] => move |spin| {
                            sender.input(AdvancedViewMsg::SetDebounce(spin.value() as u32));
                        } @debounce_handler,
                    },
                },

                adw::ActionRow {
                    set_title: "Full Speed Mode",
                    set_subtitle: "Disables RGB updates on device to minimize key input latency.",
                    #[watch]
                    set_visible: model.capabilities.has_full_speed,

                    add_suffix = &gtk::Switch {
                        set_valign: gtk::Align::Center,
                        set_active: model.full_speed_mode,
                        #[watch]
                        #[block_signal(full_speed_handler)]
                        set_active: model.full_speed_mode,

                        connect_active_notify[sender] => move |sw| {
                            sender.input(AdvancedViewMsg::ToggleFullSpeed(sw.is_active()));
                        } @full_speed_handler,
                    },
                },
            },

            // Raw packet sender
            gtk::Frame {
                set_label: Some("Raw HID Packet Inspector"),

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
                    set_margin_all: 12,

                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        add_css_class: "dim-label",
                        set_text: "Transmit an arbitrary 8-byte hex payload (pointer write commands place XOR checksum at byte 6).",
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,

                        gtk::Entry {
                            set_hexpand: true,
                            set_buffer: &model.raw_hex_buffer,
                        },

                        gtk::Button {
                            set_label: "Fix Checksum",
                            set_tooltip_text: Some("Compute byte[5] = byte[1] ^ byte[2] ^ byte[3] ^ byte[4]"),
                            connect_clicked => AdvancedViewMsg::FixRawChecksum,
                        },

                        gtk::Button {
                            set_label: "Send Packet",
                            add_css_class: "suggested-action",
                            connect_clicked => AdvancedViewMsg::SendRawPacket,
                        },
                    },

                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_selectable: true,
                        add_css_class: "dim-label",
                        #[watch]
                        set_label: &model.raw_packet_log,
                    },
                }
            },

            // Factory reset & Bootloader maintenance actions
            gtk::Frame {
                set_label: Some("Maintenance & Recovery"),

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_all: 12,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_hexpand: true,

                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "heading",
                                set_text: "Restore Factory Defaults",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "dim-label",
                                set_text: "Resets all keys, LED colors, lighting effects, and timings to defaults for this model.",
                            },
                        },

                        gtk::Button {
                            set_label: "Factory Reset",
                            add_css_class: "destructive-action",
                            connect_clicked => AdvancedViewMsg::AskFactoryReset,
                        },
                    },

                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_hexpand: true,

                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "heading",
                                set_text: "Enter IAP Bootloader Mode",
                            },
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "dim-label",
                                set_text: "Reboots the keypad into ISP/IAP bootloader mode for firmware flashing.",
                            },
                        },

                        gtk::Button {
                            set_label: "Reboot to Bootloader",
                            add_css_class: "destructive-action",
                            connect_clicked => AdvancedViewMsg::AskEnterBootloader,
                        },
                    },
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (capabilities, debounce_delay, full_speed_mode) = init;
        let model = AdvancedViewState {
            capabilities,
            debounce_delay,
            full_speed_mode,
            raw_packet_log: "Ready to send raw 8-byte HID reports.".to_string(),
            raw_hex_buffer: gtk::EntryBuffer::new(Some("00 01 06 00 00 00 00 00")),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            AdvancedViewMsg::Sync {
                capabilities,
                debounce_delay,
                full_speed_mode,
            } => {
                self.capabilities = capabilities;
                self.debounce_delay = debounce_delay;
                self.full_speed_mode = full_speed_mode;
            }
            AdvancedViewMsg::SetDebounce(val) => {
                if self.debounce_delay != val {
                    self.debounce_delay = val;
                    let _ = sender.output(AdvancedViewOutput::DebounceChanged(val));
                }
            }
            AdvancedViewMsg::ToggleFullSpeed(val) => {
                if self.full_speed_mode != val {
                    self.full_speed_mode = val;
                    let _ = sender.output(AdvancedViewOutput::FullSpeedChanged(val));
                }
            }
            AdvancedViewMsg::FixRawChecksum => {
                let raw_text = self.raw_hex_buffer.text();
                match Self::parse_hex_bytes(&raw_text) {
                    Ok(mut bytes) => {
                        bytes[5] =
                            Packet::calculate_checksum(bytes[1], bytes[2], bytes[3], bytes[4]);
                        self.raw_hex_buffer.set_text(Self::format_hex_bytes(&bytes));
                        self.raw_packet_log =
                            format!("Updated XOR checksum byte[5] = {:02X}", bytes[5]);
                    }
                    Err(e) => {
                        self.raw_packet_log = format!("Error: {e}");
                    }
                }
            }
            AdvancedViewMsg::SendRawPacket => {
                let raw_text = self.raw_hex_buffer.text();
                match Self::parse_hex_bytes(&raw_text) {
                    Ok(bytes) => {
                        self.raw_packet_log = format!("TX: {}", Self::format_hex_bytes(&bytes));
                        let _ = sender.output(AdvancedViewOutput::SendRaw(bytes));
                    }
                    Err(e) => {
                        self.raw_packet_log = format!("Error: {e}");
                    }
                }
            }
            AdvancedViewMsg::RawPacketResult(summary) => {
                self.raw_packet_log = summary;
            }
            AdvancedViewMsg::AskFactoryReset => {
                Self::show_confirm_dialog(
                    root,
                    "Restore Factory Defaults?",
                    "This will overwrite current keys, LED colors, modes, and timings on the device with model defaults.",
                    "Factory Reset",
                    AdvancedViewMsg::ConfirmFactoryReset,
                    sender,
                );
            }
            AdvancedViewMsg::AskEnterBootloader => {
                Self::show_confirm_dialog(
                    root,
                    "Reboot Keypad to Bootloader?",
                    "Your SimPad will disconnect normal keypad functions and enter IAP flashing mode until reflashed or power-cycled.",
                    "Reboot to Bootloader",
                    AdvancedViewMsg::ConfirmEnterBootloader,
                    sender,
                );
            }
            AdvancedViewMsg::ConfirmFactoryReset => {
                let _ = sender.output(AdvancedViewOutput::FactoryResetRequested);
            }
            AdvancedViewMsg::ConfirmEnterBootloader => {
                let _ = sender.output(AdvancedViewOutput::BootloaderRequested);
            }
        }
    }
}
