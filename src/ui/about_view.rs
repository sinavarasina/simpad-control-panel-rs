use relm4::adw::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct AboutViewState;

#[relm4::component(pub)]
impl SimpleComponent for AboutViewState {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_margin_all: 24,

            gtk::Image {
                set_icon_name: Some("input-keyboard-symbolic"),
                set_pixel_size: 64,
                set_halign: gtk::Align::Center,
            },

            gtk::Label {
                set_halign: gtk::Align::Center,
                add_css_class: "title-2",
                set_text: &format!("SimPad Control Panel v{}", env!("CARGO_PKG_VERSION")),
            },

            gtk::Label {
                set_halign: gtk::Align::Center,
                add_css_class: "dim-label",
                set_text: "USB HID keypad configurator built with Rust, GTK4, and Libadwaita",
            },

            gtk::ListBox {
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,

                adw::ActionRow {
                    set_title: "License",
                    set_subtitle: "Apache License 2.0",
                },

                adw::ActionRow {
                    set_title: "Supported Models",
                    set_subtitle: "SimPad Nano / Year Edition (3 keys), v2 / Anniversary / Lite (5 keys), v2 EX (6 keys), SP-E / SP-E2 (8 keys), Sayo O2C (5 keys)",
                },

                adw::ActionRow {
                    set_title: if cfg!(target_os = "windows") {
                        "USB HID Transport (Windows)"
                    } else {
                        "USB HID Transport (Linux)"
                    },
                    set_subtitle: if cfg!(target_os = "windows") {
                        "Uses native Windows HID interface (&mi_01) with Fluent system appearance"
                    } else {
                        "Uses hidraw Interface 1; install udev/99-simpad.rules into /etc/udev/rules.d/ if permission is denied"
                    },
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AboutViewState;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
