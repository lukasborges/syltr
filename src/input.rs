//! Encaminha input do GTK para o CEF (OSR): mouse, scroll, teclado e foco.
//! O host do browser chega assincronamente, então resolvemos via BrowserSlot.

use std::cell::Cell;
use std::rc::Rc;

use cef::{
    ImplBrowserHost, ImplFrame, KeyEvent, KeyEventType, MouseButtonType, MouseEvent,
};
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::engine::BrowserSlot;

const EVENTFLAG_SHIFT_DOWN: u32 = 1 << 1;
const EVENTFLAG_CONTROL_DOWN: u32 = 1 << 2;
const EVENTFLAG_ALT_DOWN: u32 = 1 << 3;
const EVENTFLAG_LEFT_MOUSE_BUTTON: u32 = 1 << 4;
const EVENTFLAG_MIDDLE_MOUSE_BUTTON: u32 = 1 << 5;
const EVENTFLAG_RIGHT_MOUSE_BUTTON: u32 = 1 << 6;

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

fn button_flag(button: u32) -> u32 {
    match button {
        2 => EVENTFLAG_MIDDLE_MOUSE_BUTTON,
        3 => EVENTFLAG_RIGHT_MOUSE_BUTTON,
        _ => EVENTFLAG_LEFT_MOUSE_BUTTON,
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

// Evento CHAR já com o caractere final (ex.: 'ã' composto de dead key + 'a').
// Vem do `commit` do IMContext, então não temos keyval; usamos o char.
fn char_event(c: char) -> KeyEvent {
    key_event(KeyEventType::CHAR, 0, c as i32, Some(c))
}

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

pub fn attach(area: &gtk::DrawingArea, slot: Rc<BrowserSlot>) {
    area.set_focusable(true);
    area.set_can_focus(true);

    let last = Rc::new(Cell::new((0.0f64, 0.0f64)));
    // Flags dos botões pressionados (para o CEF entender arraste).
    let buttons = Rc::new(Cell::new(0u32));

    // Mouse move
    {
        let motion = gtk::EventControllerMotion::new();
        let slot_m = slot.clone();
        let last_m = last.clone();
        let buttons_m = buttons.clone();
        motion.connect_motion(move |_, x, y| {
            last_m.set((x, y));
            if let Some(host) = slot_m.host() {
                host.send_mouse_move_event(Some(&mouse(x, y, buttons_m.get())), 0);
            }
        });
        let slot_l = slot.clone();
        motion.connect_leave(move |_| {
            if let Some(host) = slot_l.host() {
                host.send_mouse_move_event(Some(&mouse(-1.0, -1.0, 0)), 1);
            }
        });
        area.add_controller(motion);
    }

    // Clique (com arraste)
    {
        let click = gtk::GestureClick::new();
        click.set_button(0);
        let slot_p = slot.clone();
        let area_w = area.downgrade();
        let buttons_p = buttons.clone();
        click.connect_pressed(move |g, n, x, y| {
            if let Some(a) = area_w.upgrade() {
                a.grab_focus();
            }
            let flag = button_flag(g.current_button());
            buttons_p.set(buttons_p.get() | flag);
            if let Some(host) = slot_p.host() {
                host.set_focus(1);
                host.send_mouse_click_event(
                    Some(&mouse(x, y, buttons_p.get())),
                    button_type(g.current_button()),
                    0,
                    n,
                );
            }
        });
        let slot_r = slot.clone();
        let buttons_r = buttons.clone();
        click.connect_released(move |g, n, x, y| {
            if let Some(host) = slot_r.host() {
                host.send_mouse_click_event(
                    Some(&mouse(x, y, buttons_r.get())),
                    button_type(g.current_button()),
                    1,
                    n,
                );
            }
            buttons_r.set(buttons_r.get() & !button_flag(g.current_button()));
        });
        area.add_controller(click);
    }

    // Scroll
    {
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        let slot_s = slot.clone();
        let last_s = last.clone();
        scroll.connect_scroll(move |_, dx, dy| {
            let (x, y) = last_s.get();
            if let Some(host) = slot_s.host() {
                host.send_mouse_wheel_event(
                    Some(&mouse(x, y, 0)),
                    (-dx * 40.0) as i32,
                    (-dy * 40.0) as i32,
                );
            }
            glib::Propagation::Stop
        });
        area.add_controller(scroll);
    }

    // Input method (compõe dead keys: `~`+`a` -> `ã`, `´`+`e` -> `é`, etc.).
    // O texto final chega pelo sinal `commit`; sem isso os acentos se perdem.
    let im = gtk::IMMulticontext::new();
    im.set_client_widget(Some(area));
    // OSR não desenha o preedit sublinhado; deixamos o IM compor sem preview.
    im.set_use_preedit(false);
    {
        let slot_c = slot.clone();
        im.connect_commit(move |_, text| {
            if let Some(host) = slot_c.host() {
                for c in text.chars() {
                    host.send_key_event(Some(&char_event(c)));
                }
            }
        });
    }

    // Teclado
    {
        let key = gtk::EventControllerKey::new();
        let slot_kp = slot.clone();
        let im_kp = im.clone();
        key.connect_key_pressed(move |ctrl, keyval, _code, state| {
            // Atalhos de edição (Ctrl+C/V/X/A/Z) via comandos do frame — mais
            // confiável que depender da tradução de tecla no OSR.
            if state.contains(gdk::ModifierType::CONTROL_MASK)
                && !state.contains(gdk::ModifierType::ALT_MASK)
            {
                if let Some(c) = keyval.to_unicode().map(|c| c.to_ascii_lowercase()) {
                    if matches!(c, 'c' | 'v' | 'x' | 'a' | 'z') {
                        if let Some(frame) = slot_kp.main_frame() {
                            match c {
                                'c' => frame.copy(),
                                'v' => frame.paste(),
                                'x' => frame.cut(),
                                'a' => frame.select_all(),
                                'z' => frame.undo(),
                                _ => {}
                            }
                            return glib::Propagation::Stop;
                        }
                    }
                }
            }

            let m = mods(state);
            let wkc = win_key_code(keyval);
            // RAWKEYDOWN sempre (estado da tecla p/ o renderer e listeners de keydown).
            if let Some(host) = slot_kp.host() {
                host.send_key_event(Some(&key_event(
                    KeyEventType::RAWKEYDOWN,
                    m,
                    wkc,
                    keyval.to_unicode(),
                )));
            }

            // Deixa o IM compor. Se consumir (dead key ou char comum), o CHAR já
            // com acento vem pelo `commit` — não mandamos CHAR aqui.
            if let Some(ev) = ctrl.current_event() {
                if im_kp.filter_keypress(ev) {
                    return glib::Propagation::Stop;
                }
            }

            // IM não tratou (ex.: sem input method ativo): CHAR direto.
            if let Some(host) = slot_kp.host() {
                if let Some(c) = keyval.to_unicode() {
                    if !c.is_control() {
                        host.send_key_event(Some(&key_event(KeyEventType::CHAR, m, wkc, Some(c))));
                    }
                }
            }
            glib::Propagation::Proceed
        });
        let slot_kr = slot.clone();
        key.connect_key_released(move |_, keyval, _code, state| {
            if let Some(host) = slot_kr.host() {
                host.send_key_event(Some(&key_event(
                    KeyEventType::KEYUP,
                    mods(state),
                    win_key_code(keyval),
                    keyval.to_unicode(),
                )));
            }
        });
        area.add_controller(key);
    }

    // Foco
    {
        let focus = gtk::EventControllerFocus::new();
        let slot_fe = slot.clone();
        let im_fe = im.clone();
        focus.connect_enter(move |_| {
            im_fe.focus_in();
            if let Some(host) = slot_fe.host() {
                host.set_focus(1);
            }
        });
        let slot_fl = slot.clone();
        let im_fl = im.clone();
        focus.connect_leave(move |_| {
            // Reseta composição pendente ao perder o foco (dead key solta).
            im_fl.reset();
            im_fl.focus_out();
            if let Some(host) = slot_fl.host() {
                host.set_focus(0);
            }
        });
        area.add_controller(focus);
    }
}
