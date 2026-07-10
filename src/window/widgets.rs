//! Pure widget builders shared across the window: the rail, headers, menus and
//! small reusable pieces.

use adw::prelude::*;
use gettextrs::gettext;
use gtk::gio;

use super::EMPTY_PAGE;
use crate::catalog;
use crate::config::Service;

/// Icon size (px) used for the service logo in the Add dialog.
const SERVICE_ICON_SIZE: i32 = 28;

/// The icon-only side rail (no header of its own).
pub(super) fn build_service_list() -> gtk::ListBox {
    gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["navigation-sidebar", "rail"])
        .build()
}

/// A stack of web views, starting on the empty state.
pub(super) fn build_content_stack() -> gtk::Stack {
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .hexpand(true)
        .build();
    stack.add_named(&empty_state(), Some(EMPTY_PAGE));
    stack
}

/// The single header bar spanning the whole window width.
pub(super) fn build_primary_header(title: &adw::WindowTitle) -> adw::HeaderBar {
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(gettext("Main menu"))
        .menu_model(&primary_menu())
        .primary(true)
        .build();
    let back_button = gtk::Button::builder()
        .icon_name("go-previous-symbolic")
        .tooltip_text(gettext("Previous page"))
        .action_name("win.back")
        .build();
    let forward_button = gtk::Button::builder()
        .icon_name("go-next-symbolic")
        .tooltip_text(gettext("Next page"))
        .action_name("win.forward")
        .build();
    let reload_button = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text(gettext("Reload"))
        .action_name("win.reload")
        .build();
    let home_button = gtk::Button::builder()
        .icon_name("go-home-symbolic")
        .tooltip_text(gettext("Service home"))
        .action_name("win.home")
        .build();

    let header = adw::HeaderBar::new();
    header.pack_start(&menu_button);
    header.pack_start(&back_button);
    header.pack_start(&forward_button);
    header.pack_start(&reload_button);
    header.pack_start(&home_button);
    header.set_title_widget(Some(title));
    header
}

/// Wraps a widget in a vertically scrolling window (no horizontal scrollbar).
pub(super) fn scrollable(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(child)
        .vexpand(true)
        .build()
}

/// Wraps dialog content below a plain header bar.
pub(super) fn dialog_toolbar(content: &impl IsA<gtk::Widget>) -> adw::ToolbarView {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(content));
    toolbar
}

/// The catalog entry's bundled logo, or an initial-based Avatar as a fallback
/// for services without a bundled icon.
pub(super) fn service_icon(entry: &catalog::Entry) -> gtk::Widget {
    let path = format!("/dev/syltr/Syltr/icons/{}.svg", entry.key);
    if gio::resources_lookup_data(&path, gio::ResourceLookupFlags::NONE).is_ok() {
        let image = gtk::Image::from_resource(&path);
        image.set_pixel_size(SERVICE_ICON_SIZE);
        image.upcast()
    } else {
        adw::Avatar::new(SERVICE_ICON_SIZE, Some(entry.name), true).upcast()
    }
}

/// A flat, left-aligned button for the popover menus.
pub(super) fn menu_item(label: &str) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("flat");
    if let Some(lbl) = button.child().and_downcast::<gtk::Label>() {
        lbl.set_xalign(0.0);
    }
    button
}

/// A rail row: the service icon, centered, with the name as a hover tooltip.
pub(super) fn service_row(svc: &Service, icon: &gtk::Widget) -> gtk::ListBoxRow {
    let bx = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    bx.append(icon);

    gtk::ListBoxRow::builder()
        .child(&bx)
        .tooltip_text(&svc.name)
        .build()
}

/// The page shown when there are no services.
pub(super) fn empty_state() -> adw::StatusPage {
    let button = gtk::Button::builder()
        .label(gettext("Add service"))
        .halign(gtk::Align::Center)
        .action_name("win.add-service")
        .css_classes(["suggested-action", "pill"])
        .build();

    adw::StatusPage::builder()
        .icon_name("chat-symbolic")
        .title(gettext("No services"))
        .description(gettext("Add a messaging service to get started."))
        .child(&button)
        .build()
}

pub(super) fn primary_menu() -> gio::Menu {
    let menu = gio::Menu::new();

    let services = gio::Menu::new();
    services.append(Some(&gettext("Add service")), Some("win.add-service"));
    services.append(Some(&gettext("Remove current service")), Some("win.remove-service"));
    menu.append_section(None, &services);

    let preferences = gio::Menu::new();
    preferences.append(Some(&gettext("Do not disturb")), Some("win.toggle-dnd"));
    preferences.append(Some(&gettext("Spell-check languages…")), Some("win.spell-languages"));
    menu.append_section(None, &preferences);

    let about = gio::Menu::new();
    about.append(Some(&gettext("About Syltr")), Some("app.about"));
    about.append(Some(&gettext("Quit")), Some("app.quit"));
    menu.append_section(None, &about);

    menu
}
