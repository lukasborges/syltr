//! Right-click context menu for a service in the rail.

use adw::prelude::*;
use gettextrs::gettext;
use gtk::gdk;

use super::dialogs::show_edit_dialog;
use super::widgets::menu_item;
use super::Ui;

impl Ui {
    /// Shows the context menu for the group at `index`, anchored to `row`. It
    /// acts on that group's active instance (selecting the group makes it
    /// current first).
    pub(super) fn show_context_menu(&self, index: usize, row: &gtk::ListBoxRow, x: f64, y: f64) {
        // Selecting the group makes its active instance current; the actions
        // then operate on that instance.
        self.select_index(index);
        self.show_group_instance_at(index);

        let current = self.state.borrow().current.clone();
        let (svc_index, muted, disabled) = {
            let st = self.state.borrow();
            let idx = current
                .as_deref()
                .and_then(|id| st.services.iter().position(|s| s.id == id));
            let muted = idx.map(|i| st.services[i].muted).unwrap_or(false);
            let disabled = idx.map(|i| st.services[i].disabled).unwrap_or(false);
            (idx, muted, disabled)
        };

        // Buttons call the methods directly — GAction resolution did not work in
        // a menu-model over CEF.
        let popover = gtk::Popover::new();
        popover.set_parent(&self.window);
        popover.set_has_arrow(false);
        popover.add_css_class("menu");
        let (wx, wy) = row
            .compute_point(&self.window, &gtk::graphene::Point::new(x as f32, y as f32))
            .map(|p| (p.x() as f64, p.y() as f64))
            .unwrap_or((x, y));
        popover.set_pointing_to(Some(&gdk::Rectangle::new(wx as i32, wy as i32, 1, 1)));

        let menu = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(200)
            .build();

        let reload = menu_item(&gettext("Reload"));
        self.connect_menu_item(&reload, &popover, |ui| {
            if let Some(v) = ui.current_view() {
                v.reload();
            }
        });
        let home = menu_item(&gettext("Go to home"));
        self.connect_menu_item(&home, &popover, |ui| {
            if let Some(v) = ui.current_view() {
                v.go_home();
            }
        });
        let inspector = menu_item(&gettext("Open Web Inspector"));
        self.connect_menu_item(&inspector, &popover, |ui| {
            if let Some(v) = ui.current_view() {
                v.show_inspector();
            }
        });
        let edit = menu_item(&gettext("Edit service…"));
        self.connect_menu_item(&edit, &popover, move |ui| {
            if let Some(i) = svc_index {
                show_edit_dialog(ui, i);
            }
        });
        let mute_label = if muted {
            gettext("Unmute notifications")
        } else {
            gettext("Mute notifications")
        };
        let mute = menu_item(&mute_label);
        self.connect_menu_item(&mute, &popover, move |ui| ui.set_current_muted(!muted));
        let disable_label = if disabled {
            gettext("Enable service")
        } else {
            gettext("Disable service")
        };
        let disable = menu_item(&disable_label);
        self.connect_menu_item(&disable, &popover, move |ui| {
            ui.set_current_disabled(!disabled)
        });
        let remove = menu_item(&gettext("Remove service"));
        self.connect_menu_item(&remove, &popover, |ui| ui.remove_current());

        menu.append(&reload);
        menu.append(&home);
        menu.append(&inspector);
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        menu.append(&edit);
        menu.append(&mute);
        menu.append(&disable);
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        menu.append(&remove);

        popover.set_child(Some(&menu));
        popover.connect_closed(|p| p.unparent());
        popover.popup();
    }

    /// Wires a menu button: closes the popover, then runs `action`.
    fn connect_menu_item(
        &self,
        button: &gtk::Button,
        popover: &gtk::Popover,
        action: impl Fn(&Ui) + 'static,
    ) {
        let ui = self.clone();
        let popover = popover.clone();
        button.connect_clicked(move |_| {
            popover.popdown();
            action(&ui);
        });
    }
}
