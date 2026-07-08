//! Forwards GTK input to CEF (OSR): mouse, scroll, keyboard and focus.
//! The browser host arrives asynchronously, so we resolve it through BrowserSlot.

mod translate;

use std::cell::Cell;
use std::rc::Rc;

use cef::{ImplBrowserHost, ImplFrame, KeyEventType};
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use translate::{button_flag, button_type, char_event, key_event, mods, mouse, win_key_code};

use crate::engine::BrowserSlot;

/// One "line" of scroll, in CEF wheel-delta units.
const WHEEL_STEP: f64 = 40.0;

/// Last known pointer position; shared so wheel events (which carry no
/// coordinates) can be sent at the current cursor location.
type PointerPos = Rc<Cell<(f64, f64)>>;

/// Bitmask of the currently pressed mouse buttons, so CEF understands drags.
type PressedButtons = Rc<Cell<u32>>;

pub fn attach(area: &gtk::DrawingArea, slot: Rc<BrowserSlot>) {
    area.set_focusable(true);
    area.set_can_focus(true);

    let pointer: PointerPos = Rc::new(Cell::new((0.0, 0.0)));
    let buttons: PressedButtons = Rc::new(Cell::new(0));
    let im = build_input_method(area, &slot);

    attach_motion(area, &slot, &pointer, &buttons);
    attach_click(area, &slot, &buttons);
    attach_scroll(area, &slot, &pointer);
    attach_keyboard(area, &slot, &im);
    attach_focus(area, &slot, &im);
}

fn attach_motion(
    area: &gtk::DrawingArea,
    slot: &Rc<BrowserSlot>,
    pointer: &PointerPos,
    buttons: &PressedButtons,
) {
    let motion = gtk::EventControllerMotion::new();

    let slot_move = slot.clone();
    let pointer_move = pointer.clone();
    let buttons_move = buttons.clone();
    motion.connect_motion(move |_, x, y| {
        pointer_move.set((x, y));
        if let Some(host) = slot_move.host() {
            host.send_mouse_move_event(Some(&mouse(x, y, buttons_move.get())), 0);
        }
    });

    let slot_leave = slot.clone();
    motion.connect_leave(move |_| {
        if let Some(host) = slot_leave.host() {
            host.send_mouse_move_event(Some(&mouse(-1.0, -1.0, 0)), 1);
        }
    });

    area.add_controller(motion);
}

fn attach_click(area: &gtk::DrawingArea, slot: &Rc<BrowserSlot>, buttons: &PressedButtons) {
    let click = gtk::GestureClick::new();
    click.set_button(0); // any button

    let slot_press = slot.clone();
    let area_press = area.downgrade();
    let buttons_press = buttons.clone();
    click.connect_pressed(move |g, n, x, y| {
        if let Some(area) = area_press.upgrade() {
            area.grab_focus();
        }
        buttons_press.set(buttons_press.get() | button_flag(g.current_button()));
        if let Some(host) = slot_press.host() {
            host.set_focus(1);
            host.send_mouse_click_event(
                Some(&mouse(x, y, buttons_press.get())),
                button_type(g.current_button()),
                0,
                n,
            );
        }
    });

    let slot_release = slot.clone();
    let buttons_release = buttons.clone();
    click.connect_released(move |g, n, x, y| {
        if let Some(host) = slot_release.host() {
            host.send_mouse_click_event(
                Some(&mouse(x, y, buttons_release.get())),
                button_type(g.current_button()),
                1,
                n,
            );
        }
        buttons_release.set(buttons_release.get() & !button_flag(g.current_button()));
    });

    area.add_controller(click);
}

fn attach_scroll(area: &gtk::DrawingArea, slot: &Rc<BrowserSlot>, pointer: &PointerPos) {
    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    let slot_scroll = slot.clone();
    let pointer_scroll = pointer.clone();
    scroll.connect_scroll(move |_, dx, dy| {
        let (x, y) = pointer_scroll.get();
        if let Some(host) = slot_scroll.host() {
            host.send_mouse_wheel_event(
                Some(&mouse(x, y, 0)),
                (-dx * WHEEL_STEP) as i32,
                (-dy * WHEEL_STEP) as i32,
            );
        }
        glib::Propagation::Stop
    });
    area.add_controller(scroll);
}

/// Builds the input method that composes dead keys (`~`+`a` -> `ã`, `´`+`e` ->
/// `é`, etc.). The final text arrives via the `commit` signal; without it the
/// accents are lost.
fn build_input_method(area: &gtk::DrawingArea, slot: &Rc<BrowserSlot>) -> gtk::IMMulticontext {
    let im = gtk::IMMulticontext::new();
    im.set_client_widget(Some(area));
    // OSR does not draw the underlined preedit, so we let the IM compose without a preview.
    im.set_use_preedit(false);

    let slot_commit = slot.clone();
    im.connect_commit(move |_, text| {
        if let Some(host) = slot_commit.host() {
            for c in text.chars() {
                host.send_key_event(Some(&char_event(c)));
            }
        }
    });
    im
}

fn attach_keyboard(area: &gtk::DrawingArea, slot: &Rc<BrowserSlot>, im: &gtk::IMMulticontext) {
    let key = gtk::EventControllerKey::new();

    let slot_press = slot.clone();
    let im_press = im.clone();
    key.connect_key_pressed(move |ctrl, keyval, _code, state| {
        if try_editing_shortcut(&slot_press, keyval, state) {
            return glib::Propagation::Stop;
        }

        let modifiers = mods(state);
        let wkc = win_key_code(keyval);
        // RAWKEYDOWN always (the key state for the renderer and keydown listeners).
        if let Some(host) = slot_press.host() {
            host.send_key_event(Some(&key_event(
                KeyEventType::RAWKEYDOWN,
                modifiers,
                wkc,
                keyval.to_unicode(),
            )));
        }

        // Let the IM compose. If it consumes the key (dead key or plain char),
        // the accented CHAR arrives via `commit` — we do not send CHAR here.
        if let Some(ev) = ctrl.current_event() {
            if im_press.filter_keypress(ev) {
                return glib::Propagation::Stop;
            }
        }

        // The IM did not handle it (e.g. no active input method): send CHAR directly.
        if let Some(host) = slot_press.host() {
            if let Some(c) = keyval.to_unicode() {
                if !c.is_control() {
                    host.send_key_event(Some(&key_event(
                        KeyEventType::CHAR,
                        modifiers,
                        wkc,
                        Some(c),
                    )));
                }
            }
        }
        glib::Propagation::Proceed
    });

    let slot_release = slot.clone();
    key.connect_key_released(move |_, keyval, _code, state| {
        if let Some(host) = slot_release.host() {
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

/// Handles the edit shortcuts (Ctrl+C/V/X/A/Z) through frame commands — more
/// reliable than relying on key translation in the OSR. Returns `true` when the
/// key press was consumed.
fn try_editing_shortcut(
    slot: &Rc<BrowserSlot>,
    keyval: gdk::Key,
    state: gdk::ModifierType,
) -> bool {
    if !state.contains(gdk::ModifierType::CONTROL_MASK)
        || state.contains(gdk::ModifierType::ALT_MASK)
    {
        return false;
    }
    let Some(c) = keyval.to_unicode().map(|c| c.to_ascii_lowercase()) else {
        return false;
    };
    let Some(frame) = slot.main_frame() else {
        return false;
    };
    match c {
        'c' => frame.copy(),
        'v' => frame.paste(),
        'x' => frame.cut(),
        'a' => frame.select_all(),
        'z' => frame.undo(),
        _ => return false,
    }
    true
}

fn attach_focus(area: &gtk::DrawingArea, slot: &Rc<BrowserSlot>, im: &gtk::IMMulticontext) {
    let focus = gtk::EventControllerFocus::new();

    let slot_enter = slot.clone();
    let im_enter = im.clone();
    focus.connect_enter(move |_| {
        im_enter.focus_in();
        if let Some(host) = slot_enter.host() {
            host.set_focus(1);
        }
    });

    let slot_leave = slot.clone();
    let im_leave = im.clone();
    focus.connect_leave(move |_| {
        // Reset any pending composition when focus is lost (dangling dead key).
        im_leave.reset();
        im_leave.focus_out();
        if let Some(host) = slot_leave.host() {
            host.set_focus(0);
        }
    });

    area.add_controller(focus);
}
