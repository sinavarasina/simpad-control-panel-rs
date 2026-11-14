use relm4::adw;
use relm4::adw::prelude::*;
use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque};
use relm4::gtk::{gdk, gio};
use relm4::prelude::*;

use crate::config::lighting::{Brightness, LedConfig, LightMode, LightSpeed, RgbColor};
use crate::device::DeviceCapabilities;

fn color_to_rgba(color: RgbColor) -> gdk::RGBA {
    gdk::RGBA::new(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        1.0,
    )
}

fn rgba_to_color(rgba: gdk::RGBA) -> RgbColor {
    RgbColor::new(
        (rgba.red() * 255.0).round().clamp(0.0, 255.0) as u8,
        (rgba.green() * 255.0).round().clamp(0.0, 255.0) as u8,
        (rgba.blue() * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

#[derive(Debug, Clone, Copy)]
struct LedCardInit {
    led: LedConfig,
    show_static_color: bool,
    show_brightness: bool,
}

#[derive(Debug)]
struct LedCard {
    slot: usize,
    title: String,
    led: LedConfig,
    show_static_color: bool,
    show_brightness: bool,
}

#[derive(Debug)]
enum LedCardMsg {
    SetColor(RgbColor),
    SetBrightness(u8),
}

#[derive(Debug)]
enum LedCardOutput {
    Changed { index: usize, led: LedConfig },
}

impl LedCard {
    fn card_title(index: usize) -> String {
        match index {
            0 => "LED 1 (Left)".to_string(),
            1 => "LED 2 (Right)".to_string(),
            n => format!("LED {}", n + 1),
        }
    }

    fn apply_sync(&mut self, sync: LedCardInit) {
        self.led = sync.led;
        self.show_static_color = sync.show_static_color;
        self.show_brightness = sync.show_brightness;
    }
}

#[relm4::factory]
impl FactoryComponent for LedCard {
    type Init = LedCardInit;
    type Input = LedCardMsg;
    type Output = LedCardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        gtk::Frame {
            set_label: Some(&self.title),
            set_hexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 12,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                    #[watch]
                    set_visible: self.show_static_color,

                    gtk::Label {
                        set_label: "Color:",
                        set_width_chars: 10,
                        set_halign: gtk::Align::Start,
                    },

                    gtk::ColorDialogButton::new(Some(
                        gtk::ColorDialog::builder()
                            .title(format!("Choose {} Color", self.title))
                            .with_alpha(false)
                            .build(),
                    )) {
                        set_rgba: &color_to_rgba(self.led.color),
                        #[watch]
                        #[block_signal(color_handler)]
                        set_rgba: &color_to_rgba(self.led.color),

                        connect_rgba_notify[sender] => move |btn| {
                            sender.input(LedCardMsg::SetColor(rgba_to_color(btn.rgba())));
                        } @color_handler,
                    },

                    #[name = "hex_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("#00FF00"),
                        set_max_length: 7,
                        set_width_chars: 9,
                        set_hexpand: true,
                        set_text: &self.led.color.to_hex_string(),
                        #[track(
                            skip_init,
                            !hex_entry.has_focus()
                                && hex_entry.text() != self.led.color.to_hex_string()
                        )]
                        #[block_signal(hex_handler)]
                        set_text: &self.led.color.to_hex_string(),

                        connect_changed[sender] => move |entry| {
                            if let Ok(color) = RgbColor::from_hex_str(&entry.text()) {
                                sender.input(LedCardMsg::SetColor(color));
                            }
                        } @hex_handler,
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                    #[watch]
                    set_visible: self.show_brightness,

                    gtk::Label {
                        set_label: "Brightness:",
                        set_width_chars: 10,
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Scale::with_range(gtk::Orientation::Horizontal, 1.0, 4.0, 1.0) {
                        set_hexpand: true,
                        set_draw_value: false,
                        set_round_digits: 0,
                        set_value: self.led.brightness.effective_level() as f64,
                        #[watch]
                        #[block_signal(brightness_handler)]
                        set_value: self.led.brightness.effective_level() as f64,

                        connect_value_changed[sender] => move |scale| {
                            let level = (scale.value().round() as u8).clamp(1, 4);
                            sender.input(LedCardMsg::SetBrightness(level));
                        } @brightness_handler,
                    },

                    gtk::Label {
                        set_width_chars: 6,
                        set_halign: gtk::Align::End,
                        #[watch]
                        set_label: &format!("{}%", self.led.brightness.to_percentage()),
                    },
                },
            }
        }
    }

    fn init_model(init: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let slot = index.current_index();
        Self {
            slot,
            title: Self::card_title(slot),
            led: init.led,
            show_static_color: init.show_static_color,
            show_brightness: init.show_brightness,
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            LedCardMsg::SetColor(color) => {
                if self.led.color != color {
                    self.led.color = color;
                    let _ = sender.output(LedCardOutput::Changed {
                        index: self.slot,
                        led: self.led,
                    });
                }
            }
            LedCardMsg::SetBrightness(level) => {
                let new_b = Brightness::new(level.clamp(1, 4));
                if self.led.brightness != new_b {
                    self.led.brightness = new_b;
                    let _ = sender.output(LedCardOutput::Changed {
                        index: self.slot,
                        led: self.led,
                    });
                }
            }
        }
    }
}

pub struct LightingViewState {
    pub capabilities: DeviceCapabilities,
    pub mode: LightMode,
    pub leds: Vec<LedConfig>,
    pub speed: LightSpeed,
    pub night_light: bool,
    pub full_speed_mode: bool,
    modes_list_model: gtk::StringList,
    last_modes_ptr: *const LightMode,
    last_modes_len: usize,
    led_cards: FactoryVecDeque<LedCard>,
}

#[derive(Debug)]
pub enum LightingViewMsg {
    Sync {
        capabilities: DeviceCapabilities,
        mode: LightMode,
        leds: Vec<LedConfig>,
        speed: LightSpeed,
        night_light: bool,
        full_speed_mode: bool,
    },
    SetFullSpeed(bool),
    DisableFullSpeedClicked,
    SetModeByIndex(u32),
    LedCardChanged {
        index: usize,
        led: LedConfig,
    },
    SetFadeDelay(u16),
    SetRainbowDelay(u16),
    ApplySpeedPreset(u16),
    ToggleNightLight(bool),
}

#[derive(Debug)]
pub enum LightingViewOutput {
    ModeChanged(LightMode),
    LedChanged { index: usize, led: LedConfig },
    SpeedChanged(LightSpeed),
    NightLightChanged(bool),
    DisableFullSpeed,
}

impl LightingViewState {
    fn has_leds(&self) -> bool {
        self.capabilities.led_count > 0 && !self.leds.is_empty()
    }

    fn is_static_only(&self) -> bool {
        self.capabilities.supported_light_modes.is_empty()
    }

    fn show_static_color(&self) -> bool {
        self.is_static_only() || self.mode.uses_static_color()
    }

    fn show_brightness(&self) -> bool {
        self.capabilities.has_brightness && (self.is_static_only() || self.mode.uses_brightness())
    }

    fn show_led_cards(&self) -> bool {
        self.show_static_color() || self.show_brightness()
    }

    fn timing_visibility(&self) -> (bool, bool) {
        if !self.capabilities.has_light_speed || self.is_static_only() {
            return (false, false);
        }
        (self.mode.uses_fade_delay(), self.mode.uses_rainbow_delay())
    }

    fn effect_hint(&self) -> Option<&'static str> {
        if self.is_static_only() || self.show_static_color() {
            None
        } else if self.mode == LightMode::KeepOff {
            Some(
                "LEDs are turned off in this mode. Choose another lighting effect to adjust colors or brightness.",
            )
        } else if self.show_brightness() {
            Some(
                "Static RGB color pickers are hidden because rainbow colors are generated automatically. Per-LED brightness still applies.",
            )
        } else {
            Some(
                "Static RGB color pickers are hidden because rainbow colors are generated automatically.",
            )
        }
    }

    fn selected_mode_index(&self) -> u32 {
        self.capabilities
            .supported_light_modes
            .iter()
            .position(|&m| m == self.mode)
            .unwrap_or(0) as u32
    }

    fn rebuild_modes_if_needed(&mut self) {
        let modes = self.capabilities.supported_light_modes;
        if self.last_modes_ptr == modes.as_ptr() && self.last_modes_len == modes.len() {
            return;
        }
        let names: Vec<&str> = modes.iter().map(LightMode::name).collect();
        self.modes_list_model = gtk::StringList::new(&names);
        self.last_modes_ptr = modes.as_ptr();
        self.last_modes_len = modes.len();
    }

    fn sync_led_cards(&mut self) {
        let show_static_color = self.show_static_color();
        let show_brightness = self.show_brightness();
        let target_len = self.leds.len().min(self.capabilities.led_count);

        let mut guard = self.led_cards.guard();
        while guard.len() > target_len {
            guard.pop_back();
        }
        for (i, &led) in self.leds.iter().take(target_len).enumerate() {
            let init = LedCardInit {
                led,
                show_static_color,
                show_brightness,
            };
            if let Some(card) = guard.get_mut(i) {
                card.apply_sync(init);
            } else {
                guard.push_back(init);
            }
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for LightingViewState {
    type Init = (
        DeviceCapabilities,
        LightMode,
        Vec<LedConfig>,
        LightSpeed,
        bool,
        bool,
    );
    type Input = LightingViewMsg;
    type Output = LightingViewOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_margin_all: 16,

            adw::StatusPage {
                #[watch]
                set_visible: !model.has_leds(),
                set_vexpand: true,
                set_icon_name: Some("display-brightness-symbolic"),
                set_title: "No RGB LEDs on This Model",
                set_description: Some("This SimPad model does not include configurable RGB lighting hardware."),
            },

            gtk::Box {
                #[watch]
                set_visible: model.has_leds(),
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 16,

                gtk::Label {
                    set_halign: gtk::Align::Start,
                    add_css_class: "title-4",
                    set_text: "Lighting Configuration",
                },

                // Warning displayed whenever Full Speed Mode is enabled in Advanced settings
                gtk::Box {
                    #[watch]
                    set_visible: model.has_leds() && model.capabilities.has_full_speed && model.full_speed_mode,
                    add_css_class: "card",
                    set_orientation: gtk::Orientation::Horizontal,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,
                        set_margin_all: 12,
                        set_hexpand: true,

                        gtk::Image {
                            set_icon_name: Some("dialog-warning-symbolic"),
                            set_valign: gtk::Align::Center,
                            add_css_class: "warning",
                        },

                        gtk::Label {
                            set_hexpand: true,
                            set_halign: gtk::Align::Start,
                            set_wrap: true,
                            set_text: "Full Speed Mode is active — firmware disables all RGB LEDs while Full Speed Mode is enabled.",
                        },

                        gtk::Button {
                            set_label: "Disable Full Speed",
                            set_valign: gtk::Align::Center,
                            connect_clicked => LightingViewMsg::DisableFullSpeedClicked,
                        },
                    },
                },

                // Effect mode selector (hidden on static-LED models like Sayo O2C)
                gtk::Box {
                    #[watch]
                    set_visible: !model.is_static_only(),
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,

                    gtk::Label {
                        set_text: "Lighting Effect:",
                        set_width_chars: 14,
                        set_halign: gtk::Align::Start,
                    },

                    #[name = "mode_dropdown"]
                    gtk::DropDown {
                        set_hexpand: true,
                        set_model: Some(&model.modes_list_model),
                        set_selected: model.selected_mode_index(),
                        #[track(
                            skip_init,
                            mode_dropdown.model().as_ref()
                                != Some(model.modes_list_model.upcast_ref::<gio::ListModel>())
                        )]
                        #[block_signal(mode_handler)]
                        set_model: Some(&model.modes_list_model),
                        #[track(
                            skip_init,
                            !model.is_static_only()
                                && mode_dropdown.selected() != model.selected_mode_index()
                        )]
                        #[block_signal(mode_handler)]
                        set_selected: model.selected_mode_index(),

                        connect_selected_notify[sender] => move |dd| {
                            let sel = dd.selected();
                            if sel != gtk::INVALID_LIST_POSITION {
                                sender.input(LightingViewMsg::SetModeByIndex(sel));
                            }
                        } @mode_handler,
                    },
                },

                gtk::Label {
                    #[watch]
                    set_visible: !model.is_static_only(),
                    set_halign: gtk::Align::Start,
                    set_wrap: true,
                    add_css_class: "dim-label",
                    #[watch]
                    set_text: model.mode.description(),
                },

                gtk::Label {
                    #[watch]
                    set_visible: model.effect_hint().is_some(),
                    set_halign: gtk::Align::Start,
                    set_wrap: true,
                    add_css_class: "dim-label",
                    #[watch]
                    set_text: model.effect_hint().unwrap_or_default(),
                },

                // Dynamic LED 1 & LED 2 Cards managed via FactoryVecDeque
                #[local_ref]
                leds_row -> gtk::Box {
                    #[watch]
                    set_visible: model.show_led_cards(),
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 14,
                    set_homogeneous: true,
                },

                // Animation speed parameters (dynamically shown only when active mode uses them)
                gtk::Frame {
                    set_label: Some("Animation Timing (10 – 65535)"),
                    #[watch]
                    set_visible: {
                        let (fade, rainbow) = model.timing_visibility();
                        fade || rainbow
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 12,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            #[watch]
                            set_visible: model.timing_visibility().0,

                            gtk::Label {
                                set_text: "Fade / Ease Delay:",
                                set_width_chars: 20,
                                set_halign: gtk::Align::Start,
                            },

                            #[name = "fade_spin"]
                            gtk::SpinButton::with_range(10.0, 65535.0, 50.0) {
                                set_hexpand: true,
                                set_value: model.speed.fade_delay as f64,
                                #[track(
                                    skip_init,
                                    (fade_spin.value().round() as u16) != model.speed.fade_delay
                                )]
                                #[block_signal(fade_handler)]
                                set_value: model.speed.fade_delay as f64,

                                connect_value_changed[sender] => move |spin| {
                                    sender.input(LightingViewMsg::SetFadeDelay(spin.value() as u16));
                                } @fade_handler,
                            },
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            #[watch]
                            set_visible: model.timing_visibility().1,

                            gtk::Label {
                                set_text: "Rainbow Cycle Delay:",
                                set_width_chars: 20,
                                set_halign: gtk::Align::Start,
                            },

                            #[name = "rainbow_spin"]
                            gtk::SpinButton::with_range(10.0, 65535.0, 50.0) {
                                set_hexpand: true,
                                set_value: model.speed.rainbow_delay as f64,
                                #[track(
                                    skip_init,
                                    (rainbow_spin.value().round() as u16)
                                        != model.speed.rainbow_delay
                                )]
                                #[block_signal(rainbow_handler)]
                                set_value: model.speed.rainbow_delay as f64,

                                connect_value_changed[sender] => move |spin| {
                                    sender.input(LightingViewMsg::SetRainbowDelay(spin.value() as u16));
                                } @rainbow_handler,
                            },
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 8,

                            gtk::Label {
                                set_text: "Speed Presets:",
                                set_width_chars: 20,
                                set_halign: gtk::Align::Start,
                                add_css_class: "dim-label",
                            },

                            gtk::Button {
                                set_label: "Fast (640)",
                                connect_clicked => LightingViewMsg::ApplySpeedPreset(640),
                            },
                            gtk::Button {
                                set_label: "Default (2560)",
                                connect_clicked => LightingViewMsg::ApplySpeedPreset(2560),
                            },
                            gtk::Button {
                                set_label: "Slow (5120)",
                                connect_clicked => LightingViewMsg::ApplySpeedPreset(5120),
                            },
                        },
                    }
                },

                // Night light switch
                gtk::ListBox {
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    #[watch]
                    set_visible: model.capabilities.has_night_light,

                    adw::ActionRow {
                        set_title: "Night Light",
                        set_subtitle: "Keep RGB lighting enabled when host computer sleeps or powers off USB",

                        add_suffix = &gtk::Switch {
                            set_valign: gtk::Align::Center,
                            set_active: model.night_light,
                            #[watch]
                            #[block_signal(night_handler)]
                            set_active: model.night_light,

                            connect_active_notify[sender] => move |sw| {
                                sender.input(LightingViewMsg::ToggleNightLight(sw.is_active()));
                            } @night_handler,
                        },
                    },
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (capabilities, mode, leds, speed, night_light, full_speed_mode) = init;

        let led_cards =
            FactoryVecDeque::builder()
                .launch_default()
                .forward(sender.input_sender(), |output| match output {
                    LedCardOutput::Changed { index, led } => {
                        LightingViewMsg::LedCardChanged { index, led }
                    }
                });

        let mut model = LightingViewState {
            capabilities,
            mode,
            leds,
            speed,
            night_light,
            full_speed_mode,
            modes_list_model: gtk::StringList::new(&[]),
            last_modes_ptr: std::ptr::null(),
            last_modes_len: 0,
            led_cards,
        };
        model.rebuild_modes_if_needed();
        model.sync_led_cards();

        let leds_row = model.led_cards.widget();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            LightingViewMsg::Sync {
                capabilities,
                mode,
                leds,
                speed,
                night_light,
                full_speed_mode,
            } => {
                self.capabilities = capabilities;
                self.mode = mode;
                self.leds = leds;
                self.speed = speed;
                self.night_light = night_light;
                self.full_speed_mode = full_speed_mode;
                self.rebuild_modes_if_needed();
                self.sync_led_cards();
            }
            LightingViewMsg::SetFullSpeed(val) => {
                self.full_speed_mode = val;
            }
            LightingViewMsg::DisableFullSpeedClicked => {
                self.full_speed_mode = false;
                let _ = sender.output(LightingViewOutput::DisableFullSpeed);
            }
            LightingViewMsg::SetModeByIndex(idx) => {
                if let Some(&mode) = self.capabilities.supported_light_modes.get(idx as usize)
                    && self.mode != mode
                {
                    self.mode = mode;
                    self.sync_led_cards();
                    let _ = sender.output(LightingViewOutput::ModeChanged(mode));
                }
            }
            LightingViewMsg::LedCardChanged { index, led } => {
                if let Some(slot) = self.leds.get_mut(index)
                    && *slot != led
                {
                    *slot = led;
                    let _ = sender.output(LightingViewOutput::LedChanged { index, led });
                }
            }
            LightingViewMsg::SetFadeDelay(val) => {
                if self.speed.fade_delay != val {
                    self.speed.fade_delay = val;
                    let _ = sender.output(LightingViewOutput::SpeedChanged(self.speed));
                }
            }
            LightingViewMsg::SetRainbowDelay(val) => {
                if self.speed.rainbow_delay != val {
                    self.speed.rainbow_delay = val;
                    let _ = sender.output(LightingViewOutput::SpeedChanged(self.speed));
                }
            }
            LightingViewMsg::ApplySpeedPreset(delay) => {
                let (apply_fade, apply_rainbow) = self.timing_visibility();
                let mut new_speed = self.speed;
                if apply_fade {
                    new_speed.fade_delay = delay;
                }
                if apply_rainbow {
                    new_speed.rainbow_delay = delay;
                }
                if self.speed != new_speed {
                    self.speed = new_speed;
                    let _ = sender.output(LightingViewOutput::SpeedChanged(new_speed));
                }
            }
            LightingViewMsg::ToggleNightLight(val) => {
                if self.night_light != val {
                    self.night_light = val;
                    let _ = sender.output(LightingViewOutput::NightLightChanged(val));
                }
            }
        }
    }
}
