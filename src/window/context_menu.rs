//! Right-click context menu for a service in the rail.

use adw::prelude::*;
use gettextrs::gettext;
use gtk::gdk;

use super::widgets::menu_item;
use super::Ui;

impl Ui {
    /// Shows the context menu for service `index`, anchored to `row`.
    pub(super) fn show_context_menu(&self, index: usize, row: &gtk::ListBoxRow, x: f64, y: f64) {
        // The actions operate on the current service, so select the clicked one.
        self.select_index(index);

        let muted = self
            .state
            .borrow()
            .services
            .get(index)
            .map(|s| s.muted)
            .unwrap_or(false);

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
        let home = menu_item(&gettext("Service home"));
        self.connect_menu_item(&home, &popover, |ui| {
            if let Some(v) = ui.current_view() {
                v.go_home();
            }
        });
        let mute_label = if muted {
            gettext("Unmute notifications")
        } else {
            gettext("Mute notifications")
        };
        let mute = menu_item(&mute_label);
        self.connect_menu_item(&mute, &popover, move |ui| ui.set_current_muted(!muted));
        let remove = menu_item(&gettext("Remove service"));
        self.connect_menu_item(&remove, &popover, |ui| ui.remove_current());

        menu.append(&reload);
        menu.append(&home);
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        menu.append(&mute);
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
