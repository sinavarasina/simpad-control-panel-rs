use relm4::adw;
use relm4::adw::prelude::*;
use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque};
use relm4::gtk::glib;
use relm4::prelude::*;

use crate::config::key_binding::KeyBinding;
use crate::config::key_code::KeyCode;
use crate::config::modifier::ModifierFlags;
use crate::config::mouse::MouseButtonFlags;
use crate::ui::key_capture::translate_key_event;

const MODIFIER_OPTIONS_LEFT: [(u8, &str); 4] = [
    (ModifierFlags::LEFT_CTRL, "L Ctrl"),
    (ModifierFlags::LEFT_SHIFT, "L Shift"),
    (ModifierFlags::LEFT_ALT, "L Alt"),
    (ModifierFlags::LEFT_WIN, "L Super/Win"),
];

const MODIFIER_OPTIONS_RIGHT: [(u8, &str); 4] = [
    (ModifierFlags::RIGHT_CTRL, "R Ctrl"),
    (ModifierFlags::RIGHT_SHIFT, "R Shift"),
    (ModifierFlags::RIGHT_ALT, "R Alt"),
    (ModifierFlags::RIGHT_WIN, "R Super/Win"),
];

const MOUSE_OPTIONS: [(u8, &str); 3] = [
    (MouseButtonFlags::LEFT, "Left Click"),
    (MouseButtonFlags::MIDDLE, "Middle Click"),
    (MouseButtonFlags::RIGHT, "Right Click"),
];

#[derive(Debug, Clone, Copy)]
struct KeySlotInit {
    binding: KeyBinding,
    selected: bool,
}

#[derive(Debug)]
struct KeySlotButton {
    label: String,
    selected: bool,
}

#[derive(Debug)]
enum KeySlotOutput {
    Selected(usize),
}

impl KeySlotButton {
    fn format_label(index: usize, binding: KeyBinding) -> String {
        format!("BT{} · {}", index + 1, binding)
    }

    fn apply_sync(&mut self, index: usize, binding: KeyBinding, selected: bool) {
        self.label = Self::format_label(index, binding);
        self.selected = selected;
    }
}

#[relm4::factory]
impl FactoryComponent for KeySlotButton {
    type Init = KeySlotInit;
    type Input = ();
    type Output = KeySlotOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    view! {
        #[name = "button"]
        gtk::Button {
            #[watch]
            set_label: &self.label,
            add_css_class?: self.selected.then_some("suggested-action"),

            connect_clicked[sender, index] => move |_| {
                let _ = sender.output(KeySlotOutput::Selected(index.current_index()));
            },
        }
    }

    fn post_view() {
        if self.selected {
            button.add_css_class("suggested-action");
        } else {
            button.remove_css_class("suggested-action");
        }
    }

    fn init_model(init: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            label: Self::format_label(index.current_index(), init.binding),
            selected: init.selected,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FlagCheckInit {
    mask: u8,
    label: &'static str,
    active: bool,
}

#[derive(Debug)]
struct FlagCheckItem {
    mask: u8,
    label: &'static str,
    active: bool,
}

#[derive(Debug)]
enum FlagCheckMsg {
    Toggle(bool),
}

#[derive(Debug)]
struct FlagCheckOutput {
    mask: u8,
    active: bool,
}

#[relm4::factory]
impl FactoryComponent for FlagCheckItem {
    type Init = FlagCheckInit;
    type Input = FlagCheckMsg;
    type Output = FlagCheckOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::CheckButton {
            set_label: Some(self.label),
            set_active: self.active,
            #[watch]
            #[block_signal(check_handler)]
            set_active: self.active,

            connect_toggled[sender] => move |btn| {
                sender.input(FlagCheckMsg::Toggle(btn.is_active()));
            } @check_handler,
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            mask: init.mask,
            label: init.label,
            active: init.active,
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            FlagCheckMsg::Toggle(active) => {
                if self.active != active {
                    self.active = active;
                    let _ = sender.output(FlagCheckOutput {
                        mask: self.mask,
                        active,
                    });
                }
            }
        }
    }
}

pub struct KeysViewState {
    pub keys: Vec<KeyBinding>,
    pub selected_key_idx: usize,
    pub is_capturing: bool,
    key_slots: FactoryVecDeque<KeySlotButton>,
    mod_left_checks: FactoryVecDeque<FlagCheckItem>,
    mod_right_checks: FactoryVecDeque<FlagCheckItem>,
    mouse_checks: FactoryVecDeque<FlagCheckItem>,
}

#[derive(Debug)]
pub enum KeysViewMsg {
    SyncKeys(Vec<KeyBinding>),
    SelectKey(usize),
    SetKeyCode(u8),
    ToggleCapture(bool),
    CapturedKey {
        modifiers: ModifierFlags,
        key: KeyCode,
    },
    ClearSelected,
    ToggleModifier {
        mask: u8,
        active: bool,
    },
    ToggleMouse {
        mask: u8,
        active: bool,
    },
}

#[derive(Debug)]
pub enum KeysViewOutput {
    KeyUpdated { index: usize, binding: KeyBinding },
}

impl KeysViewState {
    fn all_key_names() -> Vec<&'static str> {
        KeyCode::all_keys().iter().map(|k| k.name()).collect()
    }

    fn key_code_dropdown_index(code: KeyCode) -> u32 {
        KeyCode::all_keys()
            .iter()
            .position(|&k| k == code)
            .unwrap_or(0) as u32
    }

    fn selected_binding(&self) -> KeyBinding {
        self.keys
            .get(self.selected_key_idx)
            .copied()
            .unwrap_or_default()
    }

    fn overview_summary(&self) -> String {
        self.keys
            .iter()
            .enumerate()
            .map(|(i, k)| format!("BT{}: {}", i + 1, k))
            .collect::<Vec<_>>()
            .join("   •   ")
    }

    fn sync_factories(&mut self) {
        let mut slot_guard = self.key_slots.guard();
        while slot_guard.len() > self.keys.len() {
            slot_guard.pop_back();
        }
        for (i, &binding) in self.keys.iter().enumerate() {
            let selected = i == self.selected_key_idx;
            if let Some(slot) = slot_guard.get_mut(i) {
                slot.apply_sync(i, binding, selected);
            } else {
                slot_guard.push_back(KeySlotInit { binding, selected });
            }
        }
        drop(slot_guard);

        let binding = self.selected_binding();
        let mod_bits = binding.modifiers.to_bits();
        let mouse_bits = binding.mouse.to_bits();

        Self::sync_flag_factory(&mut self.mod_left_checks, mod_bits);
        Self::sync_flag_factory(&mut self.mod_right_checks, mod_bits);
        Self::sync_flag_factory(&mut self.mouse_checks, mouse_bits);
    }

    fn sync_flag_factory(factory: &mut FactoryVecDeque<FlagCheckItem>, active_bits: u8) {
        let mut guard = factory.guard();
        for i in 0..guard.len() {
            if let Some(item) = guard.get_mut(i) {
                item.active = (active_bits & item.mask) != 0;
            }
        }
    }

    fn build_flag_factory(
        options: &[(u8, &'static str)],
        active_bits: u8,
        on_change: fn(u8, bool) -> KeysViewMsg,
        sender: &ComponentSender<Self>,
    ) -> FactoryVecDeque<FlagCheckItem> {
        let mut factory = FactoryVecDeque::<FlagCheckItem>::builder()
            .launch_default()
            .forward(sender.input_sender(), move |out| {
                on_change(out.mask, out.active)
            });
        let mut guard = factory.guard();
        for &(mask, label) in options {
            guard.push_back(FlagCheckInit {
                mask,
                label,
                active: (active_bits & mask) != 0,
            });
        }
        drop(guard);
        factory
    }

    fn apply_bit_mask(bits: u8, mask: u8, active: bool) -> Option<u8> {
        let is_set = (bits & mask) != 0;
        if is_set == active {
            None
        } else if active {
            Some(bits | mask)
        } else {
            Some(bits & !mask)
        }
    }

    fn emit_current(&self, sender: &ComponentSender<Self>) {
        if let Some(binding) = self.keys.get(self.selected_key_idx).copied() {
            let _ = sender.output(KeysViewOutput::KeyUpdated {
                index: self.selected_key_idx,
                binding,
            });
        }
    }
}

#[relm4::component(pub)]
impl Component for KeysViewState {
    type Init = Vec<KeyBinding>;
    type Input = KeysViewMsg;
    type Output = KeysViewOutput;
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_margin_all: 16,

            adw::StatusPage {
                #[watch]
                set_visible: model.keys.is_empty(),
                set_vexpand: true,
                set_icon_name: Some("input-keyboard-symbolic"),
                set_title: "No Configurable Keys",
                set_description: Some("This device is in Bootloader / IAP mode and exposes no macro key slots."),
            },

            gtk::Box {
                #[watch]
                set_visible: !model.keys.is_empty(),
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 14,

                gtk::Label {
                    set_halign: gtk::Align::Start,
                    add_css_class: "title-4",
                    set_text: "Key Shortcut Configuration",
                },

                gtk::Label {
                    set_halign: gtk::Align::Start,
                    add_css_class: "dim-label",
                    set_text: "Choose a physical button below, then pick a key from the list or click Capture Key.",
                },

                // Dynamic per-model physical button chips (3, 5, 6, or 8 buttons)
                #[local_ref]
                key_buttons_flow -> gtk::FlowBox {
                    set_selection_mode: gtk::SelectionMode::None,
                    set_homogeneous: true,
                    set_min_children_per_line: 3,
                    set_max_children_per_line: 4,
                    set_row_spacing: 8,
                    set_column_spacing: 8,
                },

                // Selected Key Details Card
                gtk::Frame {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 14,
                        set_margin_all: 14,

                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            add_css_class: "heading",
                            #[watch]
                            set_label: &format!(
                                "Button {}  —  Current Binding: {}",
                                model.selected_key_idx + 1,
                                model.selected_binding()
                            ),
                        },

                        // Primary key dropdown + interactive capture + clear
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 10,

                            gtk::Label {
                                set_text: "Keyboard Key:",
                                set_width_chars: 14,
                                set_halign: gtk::Align::Start,
                            },

                            #[name = "key_dropdown"]
                            gtk::DropDown {
                                set_hexpand: true,
                                set_enable_search: true,
                                set_expression: Some(&gtk::PropertyExpression::new(
                                    gtk::StringObject::static_type(),
                                    gtk::Expression::NONE,
                                    "string",
                                )),
                                set_model: Some(&gtk::StringList::new(&Self::all_key_names())),
                                set_selected: Self::key_code_dropdown_index(model.selected_binding().key),
                                #[track(
                                    skip_init,
                                    key_dropdown.selected()
                                        != Self::key_code_dropdown_index(model.selected_binding().key)
                                )]
                                #[block_signal(key_handler)]
                                set_selected: Self::key_code_dropdown_index(model.selected_binding().key),

                                connect_selected_notify[sender] => move |dd| {
                                    let sel = dd.selected() as usize;
                                    let code = KeyCode::all_keys()
                                        .get(sel)
                                        .map(|k| k.to_u8())
                                        .unwrap_or(0x00);
                                    sender.input(KeysViewMsg::SetKeyCode(code));
                                } @key_handler,
                            },

                            #[name = "capture_btn"]
                            gtk::ToggleButton {
                                set_tooltip_text: Some("Click, then press a keyboard key or shortcut combination"),
                                set_active: model.is_capturing,
                                set_label: "Capture Key",
                                #[watch]
                                #[block_signal(capture_handler)]
                                set_active: model.is_capturing,
                                #[watch]
                                set_label: if model.is_capturing {
                                    "Press any key combo..."
                                } else {
                                    "Capture Key"
                                },

                                connect_toggled[sender] => move |btn| {
                                    sender.input(KeysViewMsg::ToggleCapture(btn.is_active()));
                                } @capture_handler,
                            },

                            gtk::Button {
                                set_label: "Clear",
                                set_tooltip_text: Some("Clear keyboard, modifier, and mouse bindings for this button"),
                                connect_clicked => KeysViewMsg::ClearSelected,
                            },
                        },

                        // Left & Right Modifiers
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 10,
                            set_valign: gtk::Align::Start,

                            gtk::Label {
                                set_text: "Modifier Keys:",
                                set_width_chars: 14,
                                set_halign: gtk::Align::Start,
                                set_valign: gtk::Align::Start,
                                set_margin_top: 4,
                            },

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 6,

                                #[local_ref]
                                mod_left_box -> gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 14,
                                    set_homogeneous: true,
                                },

                                #[local_ref]
                                mod_right_box -> gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 14,
                                    set_homogeneous: true,
                                },
                            },
                        },

                        // Mouse buttons row
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 10,

                            gtk::Label {
                                set_text: "Mouse Click:",
                                set_width_chars: 14,
                                set_halign: gtk::Align::Start,
                            },

                            #[local_ref]
                            mouse_box -> gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 16,
                            },
                        },
                    }
                },

                // Overview summary of all configured buttons
                gtk::Frame {
                    set_label: Some("Configured Shortcuts Overview"),

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 12,

                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_wrap: true,
                            #[watch]
                            set_label: &model.overview_summary(),
                        },
                    }
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let key_slots =
            FactoryVecDeque::builder()
                .launch_default()
                .forward(sender.input_sender(), |output| match output {
                    KeySlotOutput::Selected(idx) => KeysViewMsg::SelectKey(idx),
                });

        let initial_binding = init.first().copied().unwrap_or_default();
        let mod_bits = initial_binding.modifiers.to_bits();
        let mouse_bits = initial_binding.mouse.to_bits();

        let mod_left_checks = Self::build_flag_factory(
            &MODIFIER_OPTIONS_LEFT,
            mod_bits,
            |mask, active| KeysViewMsg::ToggleModifier { mask, active },
            &sender,
        );
        let mod_right_checks = Self::build_flag_factory(
            &MODIFIER_OPTIONS_RIGHT,
            mod_bits,
            |mask, active| KeysViewMsg::ToggleModifier { mask, active },
            &sender,
        );
        let mouse_checks = Self::build_flag_factory(
            &MOUSE_OPTIONS,
            mouse_bits,
            |mask, active| KeysViewMsg::ToggleMouse { mask, active },
            &sender,
        );

        let mut model = KeysViewState {
            keys: init,
            selected_key_idx: 0,
            is_capturing: false,
            key_slots,
            mod_left_checks,
            mod_right_checks,
            mouse_checks,
        };
        model.sync_factories();

        let key_buttons_flow = model.key_slots.widget();
        let mod_left_box = model.mod_left_checks.widget();
        let mod_right_box = model.mod_right_checks.widget();
        let mouse_box = model.mouse_checks.widget();
        let widgets = view_output!();

        // Keyboard capture controller requires intercepting raw GDK key events
        let controller = gtk::EventControllerKey::new();
        let capture_ref = widgets.capture_btn.clone();
        controller.connect_key_pressed(move |_, key, _, state| {
            if !capture_ref.is_active() {
                return glib::Propagation::Proceed;
            }
            if let Some((modifiers, hid_key)) = translate_key_event(key, state) {
                sender.input(KeysViewMsg::CapturedKey {
                    modifiers,
                    key: hid_key,
                });
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        widgets.capture_btn.add_controller(controller);

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            KeysViewMsg::SyncKeys(keys) => {
                self.keys = keys;
                self.is_capturing = false;
                if self.selected_key_idx >= self.keys.len() {
                    self.selected_key_idx = 0;
                }
                self.sync_factories();
            }
            KeysViewMsg::SelectKey(idx) => {
                if idx < self.keys.len() && (self.selected_key_idx != idx || self.is_capturing) {
                    self.selected_key_idx = idx;
                    self.is_capturing = false;
                    self.sync_factories();
                }
            }
            KeysViewMsg::SetKeyCode(code) => {
                if let Some(binding) = self.keys.get_mut(self.selected_key_idx) {
                    let new_key = KeyCode::from_u8(code);
                    if binding.key != new_key {
                        binding.key = new_key;
                        self.sync_factories();
                        self.emit_current(&sender);
                    }
                }
            }
            KeysViewMsg::ToggleCapture(active) => {
                if self.is_capturing != active {
                    self.is_capturing = active;
                    if active {
                        widgets.capture_btn.grab_focus();
                    }
                }
            }
            KeysViewMsg::CapturedKey { modifiers, key } => {
                self.is_capturing = false;
                if let Some(binding) = self.keys.get_mut(self.selected_key_idx) {
                    binding.modifiers = modifiers;
                    binding.key = key;
                    self.sync_factories();
                    self.emit_current(&sender);
                }
            }
            KeysViewMsg::ClearSelected => {
                self.is_capturing = false;
                if let Some(binding) = self.keys.get_mut(self.selected_key_idx) {
                    let empty = KeyBinding::default();
                    if *binding != empty {
                        *binding = empty;
                        self.sync_factories();
                        self.emit_current(&sender);
                    }
                }
            }
            KeysViewMsg::ToggleModifier { mask, active } => {
                if let Some(binding) = self.keys.get_mut(self.selected_key_idx)
                    && let Some(bits) =
                        Self::apply_bit_mask(binding.modifiers.to_bits(), mask, active)
                {
                    binding.modifiers = ModifierFlags::from_bits(bits);
                    self.sync_factories();
                    self.emit_current(&sender);
                }
            }
            KeysViewMsg::ToggleMouse { mask, active } => {
                if let Some(binding) = self.keys.get_mut(self.selected_key_idx)
                    && let Some(bits) = Self::apply_bit_mask(binding.mouse.to_bits(), mask, active)
                {
                    binding.mouse = MouseButtonFlags::from_bits(bits);
                    self.sync_factories();
                    self.emit_current(&sender);
                }
            }
        }

        self.update_view(widgets, sender);
    }
}
