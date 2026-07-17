//! Pure widget builders shared across the window: the rail, headers, menus and
//! small reusable pieces.

use adw::prelude::*;
use gettextrs::gettext;
use gtk::gio;

use super::{DISABLED_PAGE, EMPTY_PAGE, WELCOME_PAGE};
use crate::catalog;
use crate::config::Service;

/// Icon size (px) used for the service logo in the Add dialog.
const SERVICE_ICON_SIZE: i32 = 28;

/// The icon-only side rail (no header of its own).
pub(super) fn build_service_list() -> gtk::ListBox {
    gtk::ListBox::builder()
        // Startup deliberately has no selected row. The first explicit
        // activation switches this to Single and selects the chosen service.
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["navigation-sidebar", "rail"])
        .build()
}

/// A stack of web views, starting on the welcome state.
pub(super) fn build_content_stack() -> gtk::Stack {
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .hexpand(true)
        .build();
    stack.add_named(&welcome_state(), Some(WELCOME_PAGE));
    stack.add_named(&empty_state(), Some(EMPTY_PAGE));
    stack.add_named(&disabled_state(), Some(DISABLED_PAGE));
    stack
}

/// The quiet landing page shown at startup until the user chooses a service.
pub(super) fn welcome_state() -> gtk::CenterBox {
    let illustration = welcome_illustration();

    let title = gtk::Label::builder()
        .label(gettext("Welcome to Syltr"))
        .justify(gtk::Justification::Center)
        .wrap(true)
        .css_classes(["welcome-title"])
        .build();
    let description = gtk::Label::builder()
        .label(gettext("Choose a service from the sidebar to get started."))
        .justify(gtk::Justification::Center)
        .wrap(true)
        .max_width_chars(44)
        .css_classes(["welcome-description"])
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(36)
        .margin_bottom(36)
        .margin_start(36)
        .margin_end(36)
        .build();
    content.append(&illustration);
    content.append(&title);
    content.append(&description);

    gtk::CenterBox::builder()
        .orientation(gtk::Orientation::Vertical)
        .center_widget(&content)
        .vexpand(true)
        .hexpand(true)
        .css_classes(["welcome-page"])
        .build()
}

/// Theme-colored conversation bubbles drawn locally, avoiding icon-theme
/// differences that can turn a missing symbolic icon into an error glyph.
fn welcome_illustration() -> gtk::DrawingArea {
    let drawing = gtk::DrawingArea::builder()
        .content_width(144)
        .content_height(144)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes(["welcome-illustration"])
        .build();

    drawing.set_draw_func(|area, cr, width, height| {
        let scale_x = width as f64 / 144.0;
        let scale_y = height as f64 / 144.0;
        cr.scale(scale_x, scale_y);

        let foreground = area.color();

        // Rear bubble: deliberately translucent to suggest several services.
        rounded_path(cr, 55.0, 67.0, 62.0, 43.0, 12.0);
        cr.set_source_rgba(
            foreground.red() as f64,
            foreground.green() as f64,
            foreground.blue() as f64,
            0.46,
        );
        let _ = cr.fill();
        cr.move_to(93.0, 107.0);
        cr.line_to(105.0, 119.0);
        cr.line_to(105.0, 105.0);
        cr.close_path();
        let _ = cr.fill();

        // Main bubble and its tail.
        rounded_path(cr, 27.0, 34.0, 82.0, 55.0, 15.0);
        cr.set_source_rgba(
            foreground.red() as f64,
            foreground.green() as f64,
            foreground.blue() as f64,
            1.0,
        );
        let _ = cr.fill();
        cr.move_to(43.0, 84.0);
        cr.line_to(39.0, 101.0);
        cr.line_to(58.0, 87.0);
        cr.close_path();
        let _ = cr.fill();

        // Three accent-colored conversation dots.
        cr.set_source_rgba(0.16, 0.18, 0.22, 0.72);
        for x in [48.0, 68.0, 88.0] {
            cr.arc(x, 61.0, 4.5, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
        }
    });

    drawing
}

fn rounded_path(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, radius: f64) {
    let right = x + w;
    let bottom = y + h;
    cr.new_sub_path();
    cr.arc(
        right - radius,
        y + radius,
        radius,
        -std::f64::consts::FRAC_PI_2,
        0.0,
    );
    cr.arc(
        right - radius,
        bottom - radius,
        radius,
        0.0,
        std::f64::consts::FRAC_PI_2,
    );
    cr.arc(
        x + radius,
        bottom - radius,
        radius,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        x + radius,
        y + radius,
        radius,
        std::f64::consts::PI,
        std::f64::consts::PI * 1.5,
    );
    cr.close_path();
}

/// The single header bar spanning the whole window width.
pub(super) fn build_primary_header(title: &adw::WindowTitle) -> adw::HeaderBar {
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(gettext("Main menu"))
        .menu_model(&primary_menu())
        .primary(true)
        .build();
    let add_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(gettext("Add service"))
        .action_name("win.add-service")
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
        .tooltip_text(gettext("Go to home"))
        .action_name("win.home")
        .build();
    let dnd_button = gtk::ToggleButton::builder()
        .icon_name("preferences-system-notifications-symbolic")
        .tooltip_text(gettext("Do not disturb"))
        .action_name("win.toggle-dnd")
        .css_classes(["dnd-toggle"])
        .build();
    dnd_button.connect_toggled(|button| {
        button.set_icon_name(if button.is_active() {
            "notifications-disabled-symbolic"
        } else {
            "preferences-system-notifications-symbolic"
        });
    });

    let header = adw::HeaderBar::new();
    header.pack_start(&menu_button);
    header.pack_start(&add_button);
    header.pack_start(&back_button);
    header.pack_start(&forward_button);
    header.pack_start(&reload_button);
    header.pack_start(&home_button);
    header.pack_end(&dnd_button);
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

/// A rail row: the service (group) icon, centered, with the name as a hover
/// tooltip. A service with several instances shows one icon with a stacked-card
/// hint; clicking it opens the instance chooser.
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

/// The page shown when the selected service is disabled.
pub(super) fn disabled_state() -> adw::StatusPage {
    adw::StatusPage::builder()
        .icon_name("action-unavailable-symbolic")
        .title(gettext("Service disabled"))
        .description(gettext(
            "Enable this service from its right-click menu in the rail.",
        ))
        .build()
}

pub(super) fn primary_menu() -> gio::Menu {
    let menu = gio::Menu::new();

    let preferences = gio::Menu::new();
    preferences.append(
        Some(&gettext("Spell-check languages…")),
        Some("win.spell-languages"),
    );
    menu.append_section(None, &preferences);

    let about = gio::Menu::new();
    about.append(Some(&gettext("About Syltr")), Some("app.about"));
    about.append(Some(&gettext("Quit")), Some("app.quit"));
    menu.append_section(None, &about);

    menu
}
