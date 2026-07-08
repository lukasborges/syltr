//! Fase B: encaminha input do GTK para o CEF (OSR).
//! Mouse (move/click), scroll, teclado e foco → `host.send_*_event`.

use std::cell::Cell;
use std::rc::Rc;

use cef::{BrowserHost, ImplBrowserHost, KeyEvent, KeyEventType, MouseButtonType, MouseEvent};
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

// Flags de modificador do CEF (cef_event_flags_t).
const EVENTFLAG_SHIFT_DOWN: u32 = 1 << 1;
const EVENTFLAG_CONTROL_DOWN: u32 = 1 << 2;
const EVENTFLAG_ALT_DOWN: u32 = 1 << 3;

fn mods(state: gdk::ModifierType) -> u32 {
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

fn mouse(x: f64, y: f64, modifiers: u32) -> MouseEvent {
    MouseEvent {
        x: x as i32,
        y: y as i32,
        modifiers,
    }
}

fn button_type(button: u32) -> MouseButtonType {
    match button {
        2 => MouseButtonType::MIDDLE,
        3 => MouseButtonType::RIGHT,
        _ => MouseButtonType::LEFT,
    }
}

fn key_event(type_: KeyEventType, modifiers: u32, wkc: i32, ch: Option<char>) -> KeyEvent {
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

/// Mapeia tecla GDK para "Windows virtual key code" (o que o CEF espera).
fn win_key_code(k: gdk::Key) -> i32 {
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

pub fn attach(area: &gtk::DrawingArea, host: BrowserHost) {
    area.set_focusable(true);
    area.set_can_focus(true);

    let last = Rc::new(Cell::new((0.0f64, 0.0f64)));

    // Mouse move
    {
        let motion = gtk::EventControllerMotion::new();
        let host_m = host.clone();
        let last_m = last.clone();
        motion.connect_motion(move |_, x, y| {
            last_m.set((x, y));
            host_m.send_mouse_move_event(Some(&mouse(x, y, 0)), 0);
        });
        let host_l = host.clone();
        motion.connect_leave(move |_| {
            host_l.send_mouse_move_event(Some(&mouse(-1.0, -1.0, 0)), 1);
        });
        area.add_controller(motion);
    }

    // Clique
    {
        let click = gtk::GestureClick::new();
        click.set_button(0); // todos os botões
        let host_p = host.clone();
        let area_w = area.downgrade();
        click.connect_pressed(move |g, n, x, y| {
            if let Some(a) = area_w.upgrade() {
                a.grab_focus();
            }
            host_p.set_focus(1);
            host_p.send_mouse_click_event(Some(&mouse(x, y, 0)), button_type(g.current_button()), 0, n);
        });
        let host_r = host.clone();
        click.connect_released(move |g, n, x, y| {
            host_r.send_mouse_click_event(Some(&mouse(x, y, 0)), button_type(g.current_button()), 1, n);
        });
        area.add_controller(click);
    }

    // Scroll
    {
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        let host_s = host.clone();
        let last_s = last.clone();
        scroll.connect_scroll(move |_, dx, dy| {
            let (x, y) = last_s.get();
            host_s.send_mouse_wheel_event(
                Some(&mouse(x, y, 0)),
                (-dx * 40.0) as i32,
                (-dy * 40.0) as i32,
            );
            glib::Propagation::Stop
        });
        area.add_controller(scroll);
    }

    // Teclado
    {
        let key = gtk::EventControllerKey::new();
        let host_kp = host.clone();
        key.connect_key_pressed(move |_, keyval, _code, state| {
            let m = mods(state);
            let wkc = win_key_code(keyval);
            let ch = keyval.to_unicode();
            host_kp.send_key_event(Some(&key_event(KeyEventType::RAWKEYDOWN, m, wkc, ch)));
            if let Some(c) = ch {
                if !c.is_control() {
                    host_kp.send_key_event(Some(&key_event(KeyEventType::CHAR, m, wkc, Some(c))));
                }
            }
            glib::Propagation::Proceed
        });
        let host_kr = host.clone();
        key.connect_key_released(move |_, keyval, _code, state| {
            host_kr.send_key_event(Some(&key_event(
                KeyEventType::KEYUP,
                mods(state),
                win_key_code(keyval),
                keyval.to_unicode(),
            )));
        });
        area.add_controller(key);
    }

    // Foco
    {
        let focus = gtk::EventControllerFocus::new();
        let host_fe = host.clone();
        focus.connect_enter(move |_| host_fe.set_focus(1));
        let host_fl = host.clone();
        focus.connect_leave(move |_| host_fl.set_focus(0));
        area.add_controller(focus);
    }
}
