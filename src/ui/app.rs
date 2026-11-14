use std::thread::sleep;
use std::time::Duration;

use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use tracing::{error, info};

use crate::config::device_settings::DeviceSettings;
use crate::config::key_binding::KeyBinding;
use crate::config::lighting::{LedConfig, LightMode, LightSpeed};
use crate::device::connection::{DeviceConnection, MockConnection};
use crate::device::detector::{DeviceInfo, enumerate_devices};
use crate::device::model::DeviceModel;
use crate::protocol::packet::Packet;
use crate::ui::about_view::AboutViewState;
use crate::ui::advanced_view::{AdvancedViewMsg, AdvancedViewOutput, AdvancedViewState};
use crate::ui::device_view::{DeviceViewMsg, DeviceViewOutput, DeviceViewState};
use crate::ui::keys_view::{KeysViewMsg, KeysViewOutput, KeysViewState};
use crate::ui::lighting_view::{LightingViewMsg, LightingViewOutput, LightingViewState};
use crate::ui::style::apply_platform_theme;

/// Main application model managing device state, hardware I/O, and active views.
pub struct AppModel {
    pub hid_api: Option<hidapi::HidApi>,
    pub detected_devices: Vec<DeviceInfo>,
    pub selected_device_idx: usize,
    pub connection: Option<Box<dyn DeviceConnection>>,
    pub is_mock: bool,
    pub mock_model: DeviceModel,
    pub active_model: DeviceModel,
    pub original_settings: DeviceSettings,
    pub current_settings: DeviceSettings,
    pub pending_changes_count: usize,
    pub firmware_version: Option<u32>,
    pub chip_id: Option<String>,

    // Subcomponents
    pub device_view: Controller<DeviceViewState>,
    pub keys_view: Controller<KeysViewState>,
    pub lighting_view: Controller<LightingViewState>,
    pub advanced_view: Controller<AdvancedViewState>,
    pub about_view: Controller<AboutViewState>,
    toast_overlay: adw::ToastOverlay,
}

#[derive(Debug)]
pub enum AppMsg {
    RefreshDevices,
    SelectDevice(usize),
    ToggleMock(bool),
    SelectMockModel(DeviceModel),
    ApplyChanges,
    DiscardChanges,
    FactoryReset,
    KeyChanged { index: usize, binding: KeyBinding },
    LightModeChanged(LightMode),
    LedChanged { index: usize, led: LedConfig },
    LightSpeedChanged(LightSpeed),
    NightLightChanged(bool),
    DebounceChanged(u32),
    FullSpeedChanged(bool),
    SendRawPacket([u8; 8]),
    EnterBootloader,
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        gtk::Window {
            set_title: Some("SimPad Control Panel"),
            set_default_size: (880, 680),

            #[name = "toast_overlay"]
            adw::ToastOverlay {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    adw::HeaderBar {
                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            set_title: "SimPad Control Panel",
                            #[watch]
                            set_subtitle: &if model.pending_changes_count > 0 {
                                format!("{} pending change(s)", model.pending_changes_count)
                            } else {
                                "All changes applied".to_string()
                            },
                        },

                        pack_end = &gtk::Button {
                            set_label: "Apply to Device",
                            add_css_class: "suggested-action",
                            #[watch]
                            set_sensitive: model.pending_changes_count > 0 && model.connection.is_some(),
                            connect_clicked => AppMsg::ApplyChanges,
                        },

                        pack_end = &gtk::Button {
                            set_label: "Discard",
                            #[watch]
                            set_sensitive: model.pending_changes_count > 0,
                            connect_clicked => AppMsg::DiscardChanges,
                        },
                    },

                    model.device_view.widget(),

                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },

                    #[name = "switcher"]
                    adw::ViewSwitcher {
                        set_policy: adw::ViewSwitcherPolicy::Wide,
                    },

                    gtk::Separator {
                        set_orientation: gtk::Orientation::Horizontal,
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),

                        #[name = "stack"]
                        adw::ViewStack {
                            set_vexpand: true,
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        apply_platform_theme();

        let hid_api = hidapi::HidApi::new().ok();
        let initial_model = DeviceModel::SimpadNanoYearEdition;
        let default_settings = DeviceSettings::default_for_model(initial_model);
        let initial_caps = initial_model.capabilities();

        let device_view = DeviceViewState::builder()
            .launch(DeviceViewState::default())
            .forward(sender.input_sender(), |msg| match msg {
                DeviceViewOutput::Refresh => AppMsg::RefreshDevices,
                DeviceViewOutput::ToggleMock(active) => AppMsg::ToggleMock(active),
                DeviceViewOutput::SelectDevice(idx) => AppMsg::SelectDevice(idx),
                DeviceViewOutput::SelectMockModel(m) => AppMsg::SelectMockModel(m),
            });

        let keys_view = KeysViewState::builder()
            .launch(default_settings.keys.clone())
            .forward(sender.input_sender(), |msg| match msg {
                KeysViewOutput::KeyUpdated { index, binding } => {
                    AppMsg::KeyChanged { index, binding }
                }
            });

        let lighting_view = LightingViewState::builder()
            .launch((
                initial_caps,
                default_settings.light_mode,
                default_settings.leds.clone(),
                default_settings.light_speed,
                default_settings.night_light,
                default_settings.full_speed_mode,
            ))
            .forward(sender.input_sender(), |msg| match msg {
                LightingViewOutput::ModeChanged(mode) => AppMsg::LightModeChanged(mode),
                LightingViewOutput::LedChanged { index, led } => AppMsg::LedChanged { index, led },
                LightingViewOutput::SpeedChanged(spd) => AppMsg::LightSpeedChanged(spd),
                LightingViewOutput::NightLightChanged(n) => AppMsg::NightLightChanged(n),
                LightingViewOutput::DisableFullSpeed => AppMsg::FullSpeedChanged(false),
            });

        let advanced_view = AdvancedViewState::builder()
            .launch((
                initial_caps,
                default_settings.debounce_delay,
                default_settings.full_speed_mode,
            ))
            .forward(sender.input_sender(), |msg| match msg {
                AdvancedViewOutput::DebounceChanged(d) => AppMsg::DebounceChanged(d),
                AdvancedViewOutput::FullSpeedChanged(f) => AppMsg::FullSpeedChanged(f),
                AdvancedViewOutput::SendRaw(raw) => AppMsg::SendRawPacket(raw),
                AdvancedViewOutput::FactoryResetRequested => AppMsg::FactoryReset,
                AdvancedViewOutput::BootloaderRequested => AppMsg::EnterBootloader,
            });

        let about_view = AboutViewState::builder().launch(()).detach();

        // Build widgets before populating `toast_overlay` in model
        let mut model = AppModel {
            hid_api,
            detected_devices: Vec::new(),
            selected_device_idx: 0,
            connection: None,
            is_mock: false,
            mock_model: initial_model,
            active_model: initial_model,
            original_settings: default_settings.clone(),
            current_settings: default_settings,
            pending_changes_count: 0,
            firmware_version: None,
            chip_id: None,
            device_view,
            keys_view,
            lighting_view,
            advanced_view,
            about_view,
            toast_overlay: adw::ToastOverlay::new(),
        };

        let widgets = view_output!();
        model.toast_overlay = widgets.toast_overlay.clone();

        widgets.switcher.set_stack(Some(&widgets.stack));

        let page1 = widgets
            .stack
            .add_titled(model.keys_view.widget(), Some("keys"), "Shortcuts");
        page1.set_icon_name(Some("input-keyboard-symbolic"));

        let page2 =
            widgets
                .stack
                .add_titled(model.lighting_view.widget(), Some("lighting"), "Lighting");
        page2.set_icon_name(Some("display-brightness-symbolic"));

        let page3 =
            widgets
                .stack
                .add_titled(model.advanced_view.widget(), Some("advanced"), "Advanced");
        page3.set_icon_name(Some("emblem-system-symbolic"));

        let page4 = widgets
            .stack
            .add_titled(model.about_view.widget(), Some("about"), "About");
        page4.set_icon_name(Some("help-about-symbolic"));

        widgets.stack.set_visible_child_name("keys");
        sender.input(AppMsg::RefreshDevices);
        root.present();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::RefreshDevices => {
                self.refresh_connected_devices();
            }
            AppMsg::SelectDevice(idx) => {
                if !self.is_mock && idx < self.detected_devices.len() {
                    self.selected_device_idx = idx;
                    self.connect_selected_device();
                }
            }
            AppMsg::ToggleMock(active) => {
                self.is_mock = active;
                if active {
                    self.activate_mock(self.mock_model);
                } else {
                    self.refresh_connected_devices();
                }
            }
            AppMsg::SelectMockModel(model) => {
                self.mock_model = model;
                if self.is_mock {
                    self.activate_mock(model);
                }
            }
            AppMsg::ApplyChanges => {
                let diff = self.original_settings.diff(&self.current_settings);
                if diff.is_empty() {
                    return;
                }
                if let Some(conn) = &mut self.connection {
                    match conn.apply_changes(&diff) {
                        Ok(()) => {
                            let count = diff.len();
                            info!("Successfully applied {count} setting changes to device.");
                            self.original_settings = self.current_settings.clone();
                            self.pending_changes_count = 0;
                            self.show_toast(&format!(
                                "Applied {count} setting change(s) to device"
                            ));
                        }
                        Err(e) => {
                            error!("Failed to apply settings to device: {e}");
                            self.show_toast(&format!("Failed to apply settings: {e}"));
                        }
                    }
                }
            }
            AppMsg::DiscardChanges => {
                self.current_settings = self.original_settings.clone();
                self.sync_editor_views();
                self.show_toast("Discarded pending changes");
            }
            AppMsg::FactoryReset => {
                let defaults = DeviceSettings::default_for_model(self.active_model);
                let packets: Vec<(u8, [u8; 4])> = (1..=11)
                    .filter_map(|ptr| defaults.get_raw_setting(ptr).map(|data| (ptr, data)))
                    .collect();
                if let Some(conn) = &mut self.connection {
                    match conn.apply_changes(&packets) {
                        Ok(()) => {
                            self.original_settings = defaults.clone();
                            self.current_settings = defaults;
                            self.sync_editor_views();
                            self.show_toast(&format!(
                                "Restored factory defaults for {}",
                                self.active_model.display_name()
                            ));
                        }
                        Err(e) => {
                            error!("Failed to restore factory defaults: {e}");
                            self.show_toast(&format!("Factory reset failed: {e}"));
                        }
                    }
                } else {
                    self.show_toast("No connected device to reset");
                }
            }
            AppMsg::KeyChanged { index, binding } => {
                if let Some(key) = self.current_settings.keys.get_mut(index) {
                    *key = binding;
                    self.recount_pending_changes();
                }
            }
            AppMsg::LightModeChanged(mode) => {
                self.current_settings.light_mode = mode;
                self.recount_pending_changes();
            }
            AppMsg::LedChanged { index, led } => {
                if let Some(l) = self.current_settings.leds.get_mut(index) {
                    *l = led;
                    self.recount_pending_changes();
                }
            }
            AppMsg::LightSpeedChanged(speed) => {
                self.current_settings.light_speed = speed;
                self.recount_pending_changes();
            }
            AppMsg::NightLightChanged(val) => {
                self.current_settings.night_light = val;
                self.recount_pending_changes();
            }
            AppMsg::DebounceChanged(val) => {
                self.current_settings.debounce_delay = val;
                self.recount_pending_changes();
            }
            AppMsg::FullSpeedChanged(val) => {
                if self.current_settings.full_speed_mode != val {
                    self.current_settings.full_speed_mode = val;
                    self.lighting_view.emit(LightingViewMsg::SetFullSpeed(val));
                    self.advanced_view.emit(AdvancedViewMsg::Sync {
                        capabilities: self.active_model.capabilities(),
                        debounce_delay: self.current_settings.debounce_delay,
                        full_speed_mode: val,
                    });
                    self.recount_pending_changes();
                }
            }
            AppMsg::SendRawPacket(raw) => {
                let summary = if let Some(conn) = &mut self.connection {
                    conn.flush_input();
                    let pkt = Packet::new(raw);
                    match conn.send_packet(&pkt) {
                        Ok(()) => {
                            sleep(Duration::from_millis(20));
                            match conn.read_packet(150) {
                                Ok(resp) => format!(
                                    "TX: {}  |  RX: {}",
                                    format_packet(&raw),
                                    format_packet(&resp.0)
                                ),
                                Err(_) => {
                                    format!(
                                        "TX: {}  |  Sent (no response packet)",
                                        format_packet(&raw)
                                    )
                                }
                            }
                        }
                        Err(e) => format!("TX failed: {e}"),
                    }
                } else {
                    "Error: No device connected.".to_string()
                };
                self.advanced_view
                    .emit(AdvancedViewMsg::RawPacketResult(summary));
            }
            AppMsg::EnterBootloader => {
                if let Some(conn) = &mut self.connection {
                    let pkt = Packet::enter_bootloader();
                    match conn.send_packet(&pkt) {
                        Ok(()) => {
                            self.show_toast(
                                "Bootloader command sent — keypad rebooting to IAP mode",
                            );
                        }
                        Err(e) => {
                            self.show_toast(&format!("Failed to enter bootloader: {e}"));
                        }
                    }
                } else {
                    self.show_toast("No connected device");
                }
            }
        }
    }
}

fn format_packet(bytes: &[u8; 8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

impl AppModel {
    fn show_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        toast.set_timeout(3);
        self.toast_overlay.add_toast(toast);
    }

    fn recount_pending_changes(&mut self) {
        self.pending_changes_count = self.original_settings.diff(&self.current_settings).len();
    }

    fn activate_mock(&mut self, model: DeviceModel) {
        let mock = MockConnection::for_model(model);
        self.firmware_version = Some(mock.version);
        self.chip_id = if model.capabilities().has_chip_id {
            Some(
                Packet::new([
                    mock.chip_id[0],
                    mock.chip_id[1],
                    mock.chip_id[2],
                    mock.chip_id[3],
                    0,
                    0,
                    0,
                    0,
                ])
                .parse_chip_id_response(),
            )
        } else {
            None
        };
        self.active_model = model;
        self.original_settings = mock.settings.clone();
        self.current_settings = mock.settings.clone();
        self.connection = Some(Box::new(mock));
        self.update_all_views(true);
    }

    fn refresh_connected_devices(&mut self) {
        if self.is_mock {
            return;
        }

        if let Some(api) = &self.hid_api {
            self.detected_devices = enumerate_devices(api);
            if self.selected_device_idx >= self.detected_devices.len() {
                self.selected_device_idx = 0;
            }
            if !self.detected_devices.is_empty() {
                self.connect_selected_device();
                return;
            }
        } else {
            self.detected_devices.clear();
        }

        self.connection = None;
        self.firmware_version = None;
        self.chip_id = None;
        self.update_all_views(false);
    }

    fn connect_selected_device(&mut self) {
        let Some(api) = &self.hid_api else {
            return;
        };
        let Some(dev_info) = self.detected_devices.get(self.selected_device_idx) else {
            return;
        };

        match dev_info.open(api) {
            Ok(mut conn) => {
                let ver = conn.get_firmware_version().ok();
                let chip_id = if dev_info.model.capabilities().has_chip_id {
                    conn.get_chip_id().ok()
                } else {
                    None
                };
                let settings = conn
                    .read_all_settings_for_model(dev_info.model)
                    .unwrap_or_else(|_| DeviceSettings::default_for_model(dev_info.model));

                self.active_model = dev_info.model;
                self.firmware_version = ver;
                self.chip_id = chip_id;
                self.original_settings = settings.clone();
                self.current_settings = settings;
                self.connection = Some(Box::new(conn));
                self.update_all_views(true);
            }
            Err(e) => {
                error!("Failed to open HID device {}: {e}", dev_info.path);
                self.connection = None;
                self.firmware_version = None;
                self.chip_id = None;
                self.update_all_views(false);
                self.show_toast(&format!("Could not open device: {e}"));
            }
        }
    }

    fn device_labels(&self) -> Vec<String> {
        self.detected_devices
            .iter()
            .enumerate()
            .map(|(i, d)| {
                format!(
                    "#{} {} ({:04X}:{:04X})",
                    i + 1,
                    d.model.display_name(),
                    d.vendor_id,
                    d.product_id
                )
            })
            .collect()
    }

    fn update_all_views(&mut self, connected: bool) {
        let labels = self.device_labels();
        self.device_view.emit(DeviceViewMsg::UpdateState {
            model: self.active_model,
            model_name: if connected {
                self.active_model.display_name().to_string()
            } else {
                "No SimPad device connected".to_string()
            },
            firmware_version: self.firmware_version,
            chip_id: self.chip_id.clone(),
            is_connected: connected,
            is_mock: self.is_mock,
            device_labels: labels,
            selected_device_idx: self.selected_device_idx,
        });

        self.sync_editor_views();
    }

    fn sync_editor_views(&mut self) {
        let caps = self.active_model.capabilities();

        self.keys_view
            .emit(KeysViewMsg::SyncKeys(self.current_settings.keys.clone()));

        self.lighting_view.emit(LightingViewMsg::Sync {
            capabilities: caps,
            mode: self.current_settings.light_mode,
            leds: self.current_settings.leds.clone(),
            speed: self.current_settings.light_speed,
            night_light: self.current_settings.night_light,
            full_speed_mode: self.current_settings.full_speed_mode,
        });

        self.advanced_view.emit(AdvancedViewMsg::Sync {
            capabilities: caps,
            debounce_delay: self.current_settings.debounce_delay,
            full_speed_mode: self.current_settings.full_speed_mode,
        });

        self.recount_pending_changes();
    }
}
