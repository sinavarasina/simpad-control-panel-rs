use relm4::gtk::gdk::{Key, ModifierType};

use crate::config::{KeyCode, ModifierFlags};

/// Returns `true` when `key` itself is a pure modifier key rather than an actionable key.
pub fn is_modifier_key(key: Key) -> bool {
    matches!(
        key,
        Key::Shift_L
            | Key::Shift_R
            | Key::Control_L
            | Key::Control_R
            | Key::Alt_L
            | Key::Alt_R
            | Key::Meta_L
            | Key::Meta_R
            | Key::Super_L
            | Key::Super_R
            | Key::Hyper_L
            | Key::Hyper_R
            | Key::ISO_Level3_Shift
    )
}

/// Extract USB HID modifier flags from a GTK/GDK modifier bitmask.
pub fn modifiers_from_gdk(state: ModifierType) -> ModifierFlags {
    ModifierFlags {
        left_ctrl: state.contains(ModifierType::CONTROL_MASK),
        left_shift: state.contains(ModifierType::SHIFT_MASK),
        left_alt: state.contains(ModifierType::ALT_MASK),
        left_win: state.contains(ModifierType::SUPER_MASK)
            || state.contains(ModifierType::META_MASK),
        right_ctrl: false,
        right_shift: false,
        right_alt: false,
        right_win: false,
    }
}

/// Translate a GDK keyval into a standard USB HID [`KeyCode`].
///
/// Shifted symbol variants (such as `!`, `@`, `_`) are normalized back to their
/// underlying physical USB HID keycode so key capture behaves consistently across OSes.
pub fn keycode_from_gdk(key: Key) -> Option<KeyCode> {
    if let Some(ch) = key.to_unicode() {
        let code = match ch {
            'a'..='z' => Some(KeyCode::from_u8(0x04 + (ch as u8 - b'a'))),
            'A'..='Z' => Some(KeyCode::from_u8(0x04 + (ch as u8 - b'A'))),
            '1' | '!' => Some(KeyCode::KEY_1),
            '2' | '@' => Some(KeyCode::KEY_2),
            '3' | '#' => Some(KeyCode::KEY_3),
            '4' | '$' => Some(KeyCode::KEY_4),
            '5' | '%' => Some(KeyCode::KEY_5),
            '6' | '^' => Some(KeyCode::KEY_6),
            '7' | '&' => Some(KeyCode::KEY_7),
            '8' | '*' => Some(KeyCode::KEY_8),
            '9' | '(' => Some(KeyCode::KEY_9),
            '0' | ')' => Some(KeyCode::KEY_0),
            '`' | '~' => Some(KeyCode::GRAVE),
            '-' | '_' => Some(KeyCode::MINUS),
            '=' | '+' => Some(KeyCode::EQUAL),
            '[' | '{' => Some(KeyCode::LEFT_BRACKET),
            ']' | '}' => Some(KeyCode::RIGHT_BRACKET),
            '\\' | '|' => Some(KeyCode::BACKSLASH),
            ';' | ':' => Some(KeyCode::SEMICOLON),
            '\'' | '"' => Some(KeyCode::QUOTE),
            ',' | '<' => Some(KeyCode::COMMA),
            '.' | '>' => Some(KeyCode::PERIOD),
            '/' | '?' => Some(KeyCode::SLASH),
            ' ' => Some(KeyCode::SPACE),
            _ => None,
        };
        if code.is_some() {
            // Let keypad symbols match explicitly first via the non-unicode keysym branch
            // below when they are keypad-specific (e.g. KP_1 vs '1').
            if !is_keypad_key(key) {
                return code;
            }
        }
    }

    match key {
        Key::Return | Key::ISO_Enter => Some(KeyCode::ENTER),
        Key::Escape => Some(KeyCode::ESCAPE),
        Key::BackSpace => Some(KeyCode::BACKSPACE),
        Key::Tab | Key::ISO_Left_Tab => Some(KeyCode::TAB),
        Key::space => Some(KeyCode::SPACE),
        Key::Caps_Lock => Some(KeyCode::CAPS_LOCK),
        Key::F1 => Some(KeyCode::F1),
        Key::F2 => Some(KeyCode::F2),
        Key::F3 => Some(KeyCode::F3),
        Key::F4 => Some(KeyCode::F4),
        Key::F5 => Some(KeyCode::F5),
        Key::F6 => Some(KeyCode::F6),
        Key::F7 => Some(KeyCode::F7),
        Key::F8 => Some(KeyCode::F8),
        Key::F9 => Some(KeyCode::F9),
        Key::F10 => Some(KeyCode::F10),
        Key::F11 => Some(KeyCode::F11),
        Key::F12 => Some(KeyCode::F12),
        Key::Print | Key::Sys_Req => Some(KeyCode::PRINT_SCREEN),
        Key::Scroll_Lock => Some(KeyCode::SCROLL_LOCK),
        Key::Pause | Key::Break => Some(KeyCode::PAUSE_BREAK),
        Key::Insert => Some(KeyCode::INSERT),
        Key::Home => Some(KeyCode::HOME),
        Key::Page_Up => Some(KeyCode::PAGE_UP),
        Key::Delete => Some(KeyCode::DELETE),
        Key::End => Some(KeyCode::END),
        Key::Page_Down => Some(KeyCode::PAGE_DOWN),
        Key::Right => Some(KeyCode::RIGHT_ARROW),
        Key::Left => Some(KeyCode::LEFT_ARROW),
        Key::Down => Some(KeyCode::DOWN_ARROW),
        Key::Up => Some(KeyCode::UP_ARROW),
        Key::Menu => Some(KeyCode::MENU),
        Key::Num_Lock => Some(KeyCode::NUM_LOCK),
        Key::KP_Divide => Some(KeyCode::KP_SLASH),
        Key::KP_Multiply => Some(KeyCode::KP_ASTERISK),
        Key::KP_Subtract => Some(KeyCode::KP_MINUS),
        Key::KP_Add => Some(KeyCode::KP_PLUS),
        Key::KP_Enter => Some(KeyCode::KP_ENTER),
        Key::KP_1 | Key::KP_End => Some(KeyCode::KP_1),
        Key::KP_2 | Key::KP_Down => Some(KeyCode::KP_2),
        Key::KP_3 | Key::KP_Page_Down => Some(KeyCode::KP_3),
        Key::KP_4 | Key::KP_Left => Some(KeyCode::KP_4),
        Key::KP_5 | Key::KP_Begin => Some(KeyCode::KP_5),
        Key::KP_6 | Key::KP_Right => Some(KeyCode::KP_6),
        Key::KP_7 | Key::KP_Home => Some(KeyCode::KP_7),
        Key::KP_8 | Key::KP_Up => Some(KeyCode::KP_8),
        Key::KP_9 | Key::KP_Page_Up => Some(KeyCode::KP_9),
        Key::KP_0 | Key::KP_Insert => Some(KeyCode::KP_0),
        Key::KP_Decimal | Key::KP_Delete => Some(KeyCode::KP_PERIOD),
        _ => None,
    }
}

fn is_keypad_key(key: Key) -> bool {
    matches!(
        key,
        Key::KP_0
            | Key::KP_1
            | Key::KP_2
            | Key::KP_3
            | Key::KP_4
            | Key::KP_5
            | Key::KP_6
            | Key::KP_7
            | Key::KP_8
            | Key::KP_9
            | Key::KP_Divide
            | Key::KP_Multiply
            | Key::KP_Subtract
            | Key::KP_Add
            | Key::KP_Decimal
            | Key::KP_Enter
    )
}

/// Convert a GTK key press event into USB HID modifiers and keycode.
///
/// Returns `None` for standalone modifier presses or unmapped keysyms.
pub fn translate_key_event(key: Key, state: ModifierType) -> Option<(ModifierFlags, KeyCode)> {
    if is_modifier_key(key) {
        return None;
    }
    let hid_key = keycode_from_gdk(key)?;
    let modifiers = modifiers_from_gdk(state);
    Some((modifiers, hid_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_letters_and_modifiers() {
        let res = translate_key_event(Key::r, ModifierType::CONTROL_MASK);
        let (mods, code) = res.expect("Ctrl+R should map");
        assert!(mods.left_ctrl);
        assert!(!mods.left_shift);
        assert_eq!(code, KeyCode::R);

        let shifted = translate_key_event(Key::Z, ModifierType::SHIFT_MASK).expect("Shift+Z");
        assert!(shifted.0.left_shift);
        assert_eq!(shifted.1, KeyCode::Z);
    }

    #[test]
    fn test_modifier_only_key_ignored() {
        assert_eq!(
            translate_key_event(Key::Control_L, ModifierType::CONTROL_MASK),
            None
        );
        assert_eq!(
            translate_key_event(Key::Shift_R, ModifierType::SHIFT_MASK),
            None
        );
    }

    #[test]
    fn test_navigation_function_and_keypad_keys() {
        assert_eq!(keycode_from_gdk(Key::F12), Some(KeyCode::F12));
        assert_eq!(keycode_from_gdk(Key::Escape), Some(KeyCode::ESCAPE));
        assert_eq!(keycode_from_gdk(Key::Page_Up), Some(KeyCode::PAGE_UP));
        assert_eq!(keycode_from_gdk(Key::KP_7), Some(KeyCode::KP_7));
        assert_eq!(keycode_from_gdk(Key::KP_Enter), Some(KeyCode::KP_ENTER));
        assert_eq!(keycode_from_gdk(Key::exclam), Some(KeyCode::KEY_1));
    }
}
