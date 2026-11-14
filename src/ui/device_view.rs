use relm4::gtk::{gio, prelude::*};
use relm4::prelude::*;

use crate::device::DeviceModel;

/// State for the top device selector and hardware summary bar.
#[derive(Debug, Clone)]
pub struct DeviceViewState {
    pub model: DeviceModel,
    pub model_name: String,
    pub firmware_version: Option<u32>,
    pub chip_id: Option<String>,
    pub is_connected: bool,
    pub is_mock: bool,
    pub device_labels: Vec<String>,
    pub selected_device_idx: usize,
    pub mock_model_idx: usize,
    device_list_model: gtk::StringList,
}

impl Default for DeviceViewState {
    fn default() -> Self {
        Self {
            model: DeviceModel::SimpadNanoYearEdition,
            model_name: "No device connected".to_string(),
            firmware_version: None,
            chip_id: None,
            is_connected: false,
            is_mock: false,
            device_labels: Vec::new(),
            selected_device_idx: 0,
            mock_model_idx: Self::index_of_model(DeviceModel::SimpadNanoYearEdition),
            device_list_model: gtk::StringList::new(&[]),
        }
    }
}

impl DeviceViewState {
    fn index_of_model(target: DeviceModel) -> usize {
        DeviceModel::all_models()
            .iter()
            .position(|&m| m == target)
            .unwrap_or(0)
    }

    fn all_model_names() -> Vec<&'static str> {
        DeviceModel::all_models()
            .iter()
            .map(DeviceModel::display_name)
            .collect()
    }

    fn status_subtitle(&self) -> String {
        if !self.is_connected {
            return "Connect a SimPad keypad or enable Virtual Device".to_string();
        }

        let mut parts = Vec::new();
        if let Some(ver) = self.firmware_version {
            parts.push(format!("Firmware: {ver}"));
        }
        if let Some(ref chip) = self.chip_id {
            parts.push(format!("Chip ID: {chip}"));
        }

        let caps = self.model.capabilities();
        let hw_desc = if self.model.is_bootloader() {
            "Bootloader / IAP Mode".to_string()
        } else if caps.led_count > 0 {
            format!("{} keys, {} LEDs", caps.key_count, caps.led_count)
        } else {
            format!("{} keys", caps.key_count)
        };
        parts.push(hw_desc);

        parts.push(if self.is_mock {
            "Virtual Device".to_string()
        } else {
            "USB HID".to_string()
        });

        parts.join("  •  ")
    }
}

#[derive(Debug)]
pub enum DeviceViewMsg {
    UpdateState {
        model: DeviceModel,
        model_name: String,
        firmware_version: Option<u32>,
        chip_id: Option<String>,
        is_connected: bool,
        is_mock: bool,
        device_labels: Vec<String>,
        selected_device_idx: usize,
    },
    RefreshRequested,
    ToggleMockRequested(bool),
    DeviceSelected(u32),
    MockModelSelected(u32),
}

#[derive(Debug)]
pub enum DeviceViewOutput {
    Refresh,
    ToggleMock(bool),
    SelectDevice(usize),
    SelectMockModel(DeviceModel),
}

#[relm4::component(pub)]
impl SimpleComponent for DeviceViewState {
    type Init = DeviceViewState;
    type Input = DeviceViewMsg;
    type Output = DeviceViewOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 12,
            set_margin_all: 8,

            gtk::Image {
                set_icon_name: Some("input-keyboard-symbolic"),
                set_pixel_size: 24,
                set_valign: gtk::Align::Center,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                set_spacing: 2,

                gtk::Label {
                    set_halign: gtk::Align::Start,
                    add_css_class: "heading",
                    #[watch]
                    set_label: &model.model_name,
                },

                gtk::Label {
                    set_halign: gtk::Align::Start,
                    add_css_class: "dim-label",
                    add_css_class: "caption",
                    #[watch]
                    set_label: &model.status_subtitle(),
                },
            },

            #[name = "device_dropdown"]
            gtk::DropDown {
                set_tooltip_text: Some("Select active physical SimPad device"),
                set_model: Some(&model.device_list_model),
                #[watch]
                set_visible: !model.is_mock && model.device_labels.len() > 1,
                #[track(
                    skip_init,
                    device_dropdown.model().as_ref()
                        != Some(model.device_list_model.upcast_ref::<gio::ListModel>())
                )]
                #[block_signal(device_handler)]
                set_model: Some(&model.device_list_model),
                #[track(
                    skip_init,
                    model.selected_device_idx < model.device_labels.len()
                        && device_dropdown.selected() != model.selected_device_idx as u32
                )]
                #[block_signal(device_handler)]
                set_selected: model.selected_device_idx as u32,

                connect_selected_notify[sender] => move |dd| {
                    let idx = dd.selected();
                    if idx != gtk::INVALID_LIST_POSITION {
                        sender.input(DeviceViewMsg::DeviceSelected(idx));
                    }
                } @device_handler,
            },

            gtk::DropDown {
                set_tooltip_text: Some("Virtual Device model to simulate"),
                set_model: Some(&gtk::StringList::new(&Self::all_model_names())),
                set_selected: model.mock_model_idx as u32,
                #[watch]
                set_visible: model.is_mock,
                #[watch]
                #[block_signal(mock_model_handler)]
                set_selected: model.mock_model_idx as u32,

                connect_selected_notify[sender] => move |dd| {
                    let idx = dd.selected();
                    if idx != gtk::INVALID_LIST_POSITION {
                        sender.input(DeviceViewMsg::MockModelSelected(idx));
                    }
                } @mock_model_handler,
            },

            gtk::CheckButton {
                set_label: Some("Virtual Device"),
                set_tooltip_text: Some("Enable simulated SimPad for offline testing"),
                set_active: model.is_mock,
                #[watch]
                #[block_signal(mock_check_handler)]
                set_active: model.is_mock,

                connect_toggled[sender] => move |btn| {
                    sender.input(DeviceViewMsg::ToggleMockRequested(btn.is_active()));
                } @mock_check_handler,
            },

            gtk::Button {
                set_icon_name: "view-refresh-symbolic",
                set_tooltip_text: Some("Scan for connected SimPad hardware"),
                connect_clicked => DeviceViewMsg::RefreshRequested,
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = init;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            DeviceViewMsg::UpdateState {
                model,
                model_name,
                firmware_version,
                chip_id,
                is_connected,
                is_mock,
                device_labels,
                selected_device_idx,
            } => {
                self.model = model;
                self.model_name = model_name;
                self.firmware_version = firmware_version;
                self.chip_id = chip_id;
                self.is_connected = is_connected;
                self.is_mock = is_mock;
                self.selected_device_idx = selected_device_idx;
                self.mock_model_idx = Self::index_of_model(model);

                if self.device_labels != device_labels {
                    let strs: Vec<&str> = device_labels.iter().map(String::as_str).collect();
                    self.device_list_model = gtk::StringList::new(&strs);
                    self.device_labels = device_labels;
                }
            }
            DeviceViewMsg::RefreshRequested => {
                let _ = sender.output(DeviceViewOutput::Refresh);
            }
            DeviceViewMsg::ToggleMockRequested(active) => {
                if self.is_mock != active {
                    self.is_mock = active;
                    let _ = sender.output(DeviceViewOutput::ToggleMock(active));
                }
            }
            DeviceViewMsg::DeviceSelected(idx) => {
                let idx = idx as usize;
                if !self.is_mock
                    && idx < self.device_labels.len()
                    && self.selected_device_idx != idx
                {
                    self.selected_device_idx = idx;
                    let _ = sender.output(DeviceViewOutput::SelectDevice(idx));
                }
            }
            DeviceViewMsg::MockModelSelected(idx) => {
                let idx = idx as usize;
                let models = DeviceModel::all_models();
                if self.is_mock && idx < models.len() && self.mock_model_idx != idx {
                    self.mock_model_idx = idx;
                    let _ = sender.output(DeviceViewOutput::SelectMockModel(models[idx]));
                }
            }
        }
    }
}
