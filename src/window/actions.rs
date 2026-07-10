//! Window/app actions and keyboard accelerators.

use adw::prelude::*;
use gtk::gio;

use super::dialogs::{show_about, show_add_dialog};
use super::Ui;

pub(super) fn wire_actions(app: &adw::Application, ui: &Ui) {
    add_action(ui, "add-service", show_add_dialog);
    add_action(ui, "reload", |ui| {
        if let Some(view) = ui.current_view() {
            view.reload();
        }
    });
    add_action(ui, "back", |ui| {
        if let Some(view) = ui.current_view() {
            view.go_back();
        }
    });
    add_action(ui, "forward", |ui| {
        if let Some(view) = ui.current_view() {
            view.go_forward();
        }
    });
    add_action(ui, "home", |ui| {
        if let Some(view) = ui.current_view() {
            view.go_home();
        }
    });
    add_action(ui, "remove-service", Ui::remove_current);
    add_action(ui, "spell-languages", Ui::show_spell_dialog);

    add_toggle_action(ui, "mute", false, |ui, muted| ui.set_current_muted(muted));
    add_toggle_action(ui, "toggle-dnd", false, |ui, on| {
        ui.dnd.set(on);
        ui.apply_all_notifications();
    });

    for i in 1usize..=9 {
        add_action(ui, &format!("goto{i}"), move |ui| ui.select_index(i - 1));
        app.set_accels_for_action(&format!("win.goto{i}"), &[&format!("<Primary>{i}")]);
    }
    add_action(ui, "next-service", |ui| ui.step(1));
    app.set_accels_for_action("win.next-service", &["<Primary>Page_Down", "<Alt>Down"]);
    add_action(ui, "prev-service", |ui| ui.step(-1));
    app.set_accels_for_action("win.prev-service", &["<Primary>Page_Up", "<Alt>Up"]);

    {
        let win = ui.window.clone();
        let action = gio::SimpleAction::new("about", None);
        action.connect_activate(move |_, _| show_about(&win));
        app.add_action(&action);
    }
    {
        let win = ui.window.clone();
        let action = gio::SimpleAction::new("quit", None);
        action.connect_activate(move |_, _| win.close());
        app.add_action(&action);
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
    }
    app.set_accels_for_action("win.reload", &["<Primary>r", "F5"]);
    app.set_accels_for_action("win.back", &["<Alt>Left"]);
    app.set_accels_for_action("win.forward", &["<Alt>Right"]);
    app.set_accels_for_action("win.add-service", &["<Primary>n"]);
}

/// Registers a stateless window action that runs `handler` when activated.
fn add_action(ui: &Ui, name: &str, handler: impl Fn(&Ui) + 'static) {
    let window = ui.window.clone();
    let ui = ui.clone();
    let action = gio::SimpleAction::new(name, None);
    action.connect_activate(move |_, _| handler(&ui));
    window.add_action(&action);
}

/// Registers a boolean stateful window action; `handler` receives the new value.
fn add_toggle_action(ui: &Ui, name: &str, initial: bool, handler: impl Fn(&Ui, bool) + 'static) {
    let window = ui.window.clone();
    let ui = ui.clone();
    let action = gio::SimpleAction::new_stateful(name, None, &initial.to_variant());
    action.connect_change_state(move |a, value| {
        if let Some(v) = value {
            handler(&ui, v.get().unwrap_or(false));
            a.set_state(v);
        }
    });
    window.add_action(&action);
}
