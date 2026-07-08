//! Translation of GTK input events into CEF event structs.

use cef::{KeyEvent, KeyEventType, MouseButtonType, MouseEvent};
use gtk::gdk;

const EVENTFLAG_SHIFT_DOWN: u32 = 1 << 1;
const EVENTFLAG_CONTROL_DOWN: u32 = 1 << 2;
const EVENTFLAG_ALT_DOWN: u32 = 1 << 3;
const EVENTFLAG_LEFT_MOUSE_BUTTON: u32 = 1 << 4;
const EVENTFLAG_MIDDLE_MOUSE_BUTTON: u32 = 1 << 5;
const EVENTFLAG_RIGHT_MOUSE_BUTTON: u32 = 1 << 6;

pub(super) fn mods(state: gdk::ModifierType) -> u32 {
    let mut m = 0;
    if state.contains(gdk::ModifierType::SHIFT_MASK) {
        m |= EVENTFLAG_SHIFT_DOWN;
    }
    if state.contains(gdk::ModifierType::CONTROL_MASK) {
        m |= EVENTFLAG_CONTROL_DOWN;
    }
    if state.contains(gdk::ModifierType::ALT_MASK) {
        m |= EVENTFLAG_ALT_DOWN;
    }
    m
}

pub(super) fn mouse(x: f64, y: f64, modifiers: u32) -> MouseEvent {
    MouseEvent {
        x: x as i32,
        y: y as i32,
        modifiers,
    }
}

pub(super) fn button_type(button: u32) -> MouseButtonType {
    match button {
        2 => MouseButtonType::MIDDLE,
        3 => MouseButtonType::RIGHT,
        _ => MouseButtonType::LEFT,
    }
}

pub(super) fn button_flag(button: u32) -> u32 {
    match button {
        2 => EVENTFLAG_MIDDLE_MOUSE_BUTTON,
        3 => EVENTFLAG_RIGHT_MOUSE_BUTTON,
        _ => EVENTFLAG_LEFT_MOUSE_BUTTON,
    }
}

pub(super) fn key_event(type_: KeyEventType, modifiers: u32, wkc: i32, ch: Option<char>) -> KeyEvent {
    let character = ch.map(|c| c as u32 as u16).unwrap_or(0);
    KeyEvent {
        size: std::mem::size_of::<KeyEvent>(),
        type_,
        modifiers,
        windows_key_code: wkc,
        native_key_code: 0,
        is_system_key: 0,
        character,
        unmodified_character: character,
        focus_on_editable_field: 0,
    }
}

/// A CHAR event carrying the final character (e.g. 'ã' composed from a dead key
/// plus 'a'). It comes from the IMContext `commit`, so there is no keyval.
pub(super) fn char_event(c: char) -> KeyEvent {
    key_event(KeyEventType::CHAR, 0, c as i32, Some(c))
}

pub(super) fn win_key_code(k: gdk::Key) -> i32 {
    use gdk::Key;
    if k == Key::BackSpace {
        0x08
    } else if k == Key::Tab {
        0x09
    } else if k == Key::Return || k == Key::KP_Enter {
        0x0D
    } else if k == Key::Escape {
        0x1B
    } else if k == Key::space {
        0x20
    } else if k == Key::Left {
        0x25
    } else if k == Key::Up {
        0x26
    } else if k == Key::Right {
        0x27
    } else if k == Key::Down {
        0x28
    } else if k == Key::Delete {
        0x2E
    } else if k == Key::Home {
        0x24
    } else if k == Key::End {
        0x23
    } else if k == Key::Page_Up {
        0x21
    } else if k == Key::Page_Down {
        0x22
    } else if let Some(c) = k.to_unicode() {
        if c.is_ascii_alphanumeric() {
            c.to_ascii_uppercase() as i32
        } else {
            0
        }
    } else {
        0
    }
}
