use relm4::adw;

/// Apply platform-appropriate theming and desktop color-scheme integration.
///
/// - On Linux / Unix: uses libadwaita's default XDG desktop portal scheme so the
///   application follows GNOME / Hyprland / system appearance natively.
/// - On Windows: detects the Windows light/dark theme preference (`AppsUseLightTheme`)
///   and installs Windows 11 / Fluent-inspired CSS styling.
pub fn apply_platform_theme() {
    #[cfg(not(target_os = "windows"))]
    {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);
    }

    #[cfg(target_os = "windows")]
    {
        apply_windows_theme();
    }
}

#[cfg(target_os = "windows")]
fn apply_windows_theme() {
    let dark = windows_is_dark_mode();
    let scheme = if dark {
        adw::ColorScheme::ForceDark
    } else {
        adw::ColorScheme::ForceLight
    };
    adw::StyleManager::default().set_color_scheme(scheme);

    if let Some(display) = relm4::gtk::gdk::Display::default() {
        let provider = relm4::gtk::CssProvider::new();
        provider.load_from_string(&windows_fluent_css(dark));
        relm4::gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            relm4::gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// Build Fluent / Windows 11 native-like GTK4 + Libadwaita CSS.
pub fn windows_fluent_css(dark: bool) -> String {
    let palette = if dark {
        FluentPalette {
            window_bg: "#202020",
            surface_bg: "#2b2b2b",
            surface_hover: "#323232",
            control_bg: "#343434",
            control_hover: "#3d3d3d",
            border: "#3d3d3d",
            strong_border: "#4d4d4d",
            fg: "#ffffff",
            fg_muted: "#c5c5c5",
            accent: "#60cdff",
            accent_hover: "#72d3ff",
            accent_fg: "#000000",
        }
    } else {
        FluentPalette {
            window_bg: "#f3f3f3",
            surface_bg: "#ffffff",
            surface_hover: "#f9f9f9",
            control_bg: "#fbfbfb",
            control_hover: "#f4f4f4",
            border: "#e5e5e5",
            strong_border: "#d1d1d1",
            fg: "#1b1b1b",
            fg_muted: "#5f5f5f",
            accent: "#0067c0",
            accent_hover: "#1975c5",
            accent_fg: "#ffffff",
        }
    };

    format!(
        r#"
window,
window.background {{
    font-family: "Segoe UI Variable Text", "Segoe UI", sans-serif;
    font-size: 13px;
    background-color: {window_bg};
    color: {fg};
}}

headerbar {{
    background-color: {window_bg};
    border-bottom: 1px solid {border};
    box-shadow: none;
    min-height: 44px;
}}

list.boxed-list,
.card {{
    background-color: {surface_bg};
    border: 1px solid {border};
    border-radius: 8px;
    box-shadow: none;
}}

list.boxed-list row {{
    border-radius: 6px;
}}

button {{
    border-radius: 5px;
    font-weight: 500;
    background-color: {control_bg};
    color: {fg};
    border: 1px solid {border};
    box-shadow: none;
}}

button:hover {{
    background-color: {control_hover};
    border-color: {strong_border};
}}

button.flat,
viewswitcher button {{
    background-color: transparent;
    border-color: transparent;
}}

viewswitcher button:checked,
button:checked {{
    background-color: {surface_hover};
    border-color: {strong_border};
}}

button.suggested-action {{
    background-color: {accent};
    color: {accent_fg};
    border-color: {accent};
}}

button.suggested-action:hover {{
    background-color: {accent_hover};
    border-color: {accent_hover};
}}

entry,
spinbutton {{
    border-radius: 5px;
    background-color: {surface_bg};
    border: 1px solid {strong_border};
    color: {fg};
}}

switch {{
    border-radius: 999px;
}}

switch:checked {{
    background-color: {accent};
}}

.dim-label {{
    color: {fg_muted};
}}
"#,
        window_bg = palette.window_bg,
        surface_bg = palette.surface_bg,
        surface_hover = palette.surface_hover,
        control_bg = palette.control_bg,
        control_hover = palette.control_hover,
        border = palette.border,
        strong_border = palette.strong_border,
        fg = palette.fg,
        fg_muted = palette.fg_muted,
        accent = palette.accent,
        accent_hover = palette.accent_hover,
        accent_fg = palette.accent_fg,
    )
}

struct FluentPalette {
    window_bg: &'static str,
    surface_bg: &'static str,
    surface_hover: &'static str,
    control_bg: &'static str,
    control_hover: &'static str,
    border: &'static str,
    strong_border: &'static str,
    fg: &'static str,
    fg_muted: &'static str,
    accent: &'static str,
    accent_hover: &'static str,
    accent_fg: &'static str,
}

#[cfg(target_os = "windows")]
fn windows_is_dark_mode() -> bool {
    const HKEY_CURRENT_USER: isize = -2147483647; // 0x80000001
    const RRF_RT_REG_DWORD: u32 = 0x0000_0010;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegGetValueW(
            hkey: *mut std::ffi::c_void,
            lp_sub_key: *const u16,
            lp_value: *const u16,
            dw_flags: u32,
            pdw_type: *mut u32,
            pv_data: *mut u8,
            pcb_data: *mut u32,
        ) -> i32;
    }

    fn wide_str(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let subkey = wide_str(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value_name = wide_str("AppsUseLightTheme");
    let mut reg_type: u32 = 0;
    let mut data: u32 = 1; // Default to light theme if key is absent
    let mut size: u32 = std::mem::size_of::<u32>() as u32;

    // SAFETY: Pointers point to valid null-terminated UTF-16 strings and properly
    // sized stack buffers for the duration of this synchronous Win32 registry query.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER as *mut std::ffi::c_void,
            subkey.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            &mut reg_type,
            (&mut data as *mut u32).cast::<u8>(),
            &mut size,
        )
    };

    status == 0 && data == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_fluent_css_light_and_dark() {
        let light = windows_fluent_css(false);
        assert!(light.contains("Segoe UI"));
        assert!(light.contains("#f3f3f3"));
        assert!(light.contains("#0067c0"));

        let dark = windows_fluent_css(true);
        assert!(dark.contains("Segoe UI"));
        assert!(dark.contains("#202020"));
        assert!(dark.contains("#60cdff"));
    }
}
