//! Main window — GNOME layout with AdwNavigationSplitView: a service sidebar on
//! the left, a stack of web views on the right.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use gettextrs::gettext;
use gtk::{gdk, gio, glib};

use crate::config::{self, Service};
use crate::{catalog, engine, spellcheck};

const EMPTY_PAGE: &str = "__empty__";

/// Fixed width of the icon-only side rail.
const RAIL_WIDTH: f64 = 84.0;

/// Mutable state shared across the callbacks.
struct State {
    services: Vec<Service>,
    views: HashMap<String, engine::ServiceView>,
    current: Option<String>,
}

/// Cloneable handle to the window and its main widgets.
#[derive(Clone)]
struct Ui {
    window: adw::ApplicationWindow,
    split: adw::NavigationSplitView,
    list: gtk::ListBox,
    stack: gtk::Stack,
    title: adw::WindowTitle,
    app: adw::Application,
    /// Global "do not disturb" mode (suppresses every notification).
    dnd: Rc<Cell<bool>>,
    /// Active spell-check languages.
    spell: Rc<RefCell<Vec<String>>>,
    /// Media/WebRTC capture enabled (camera, mic, calls).
    media: Rc<Cell<bool>>,
    state: Rc<RefCell<State>>,
}

/// Entry point: builds (or re-presents) the app window.
pub fn build(app: &adw::Application) {
    if let Some(win) = app.active_window() {
        win.present();
        return;
    }

    let settings = config::load_settings();
    let state = Rc::new(RefCell::new(State {
        services: config::load(),
        views: HashMap::new(),
        current: None,
    }));

    let list = build_service_list();
    let sidebar_page = adw::NavigationPage::builder()
        .title(gettext("Services"))
        .child(&scrollable(&list))
        .build();

    let stack = build_content_stack();
    let content_page = adw::NavigationPage::builder()
        .title("Syltr")
        .child(&stack)
        .build();

    let split = adw::NavigationSplitView::builder()
        .sidebar(&sidebar_page)
        .content(&content_page)
        .min_sidebar_width(RAIL_WIDTH)
        .max_sidebar_width(RAIL_WIDTH)
        .build();

    let title = adw::WindowTitle::new("Syltr", "");
    let root_toolbar = adw::ToolbarView::new();
    root_toolbar.add_top_bar(&build_primary_header(&title));
    root_toolbar.set_content(Some(&split));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Syltr")
        .default_width(1100)
        .default_height(760)
        .width_request(360)
        .height_request(360)
        .content(&root_toolbar)
        .build();

    let ui = Ui {
        window: window.clone(),
        split,
        list: list.clone(),
        stack,
        title,
        app: app.clone(),
        dnd: Rc::new(Cell::new(false)),
        spell: Rc::new(RefCell::new(settings.spell_languages.clone())),
        media: Rc::new(Cell::new(settings.media_enabled)),
        state,
    };

    wire_actions(app, &ui);

    let ui_selected = ui.clone();
    list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            ui_selected.show_service_at(row.index() as usize);
        }
    });

    ui.refresh_sidebar();
    engine::start_pump();
    window.present();
}

impl Ui {
    /// Ensures a service's web view exists (loading in the background) and
    /// returns a clone of it. Called upfront for every service so favicons load
    /// and notifications arrive, like in Franz.
    fn ensure_view(&self, svc: &Service) -> engine::ServiceView {
        if let Some(view) = self.state.borrow().views.get(&svc.id) {
            return view.clone();
        }
        let view = engine::ServiceView::new(
            &svc.id,
            &svc.name,
            &svc.url,
            &config::session_dir(&svc.id),
            &self.app,
            self.dnd.clone(),
            svc.muted,
            &self.spell.borrow(),
            self.media.get(),
        );
        self.stack.add_named(view.widget(), Some(&svc.id));
        self.state
            .borrow_mut()
            .views
            .insert(svc.id.clone(), view.clone());
        view
    }

    /// Rebuilds the side rail from the current state.
    fn refresh_sidebar(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let services = self.state.borrow().services.clone();
        for (i, svc) in services.iter().enumerate() {
            let view = self.ensure_view(svc);
            let icon = view.icon();
            // The icon is reused across rows; detach it from its previous parent
            // before reattaching, otherwise the new row stays empty (a bug when
            // reordering).
            if icon.parent().is_some() {
                icon.unparent();
            }
            let row = service_row(svc, icon);
            self.attach_row_controllers(&row, i);
            self.list.append(&row);
        }
        if services.is_empty() {
            self.stack.set_visible_child_name(EMPTY_PAGE);
            self.title.set_title("Syltr");
            self.state.borrow_mut().current = None;
        } else {
            // Keep a valid selection (or select the first).
            let idx = self
                .state
                .borrow()
                .current
                .as_ref()
                .and_then(|cur| services.iter().position(|s| &s.id == cur))
                .unwrap_or(0);
            self.select_index(idx);
        }
    }

    fn select_index(&self, idx: usize) {
        if let Some(row) = self.list.row_at_index(idx as i32) {
            self.list.select_row(Some(&row));
        }
    }

    /// Attaches drag-to-reorder and the context menu to a rail row.
    fn attach_row_controllers(&self, row: &gtk::ListBoxRow, index: usize) {
        // Drag source: carries the source index.
        let drag = gtk::DragSource::new();
        drag.set_actions(gdk::DragAction::MOVE);
        let from = index as i32;
        drag.connect_prepare(move |_, _, _| {
            Some(gdk::ContentProvider::for_value(&from.to_value()))
        });
        row.add_controller(drag);

        // Drop target: moves the source service to this position.
        let drop = gtk::DropTarget::new(glib::Type::I32, gdk::DragAction::MOVE);
        let ui = self.clone();
        let to = index;
        drop.connect_drop(move |_, value, _, _| {
            if let Ok(src) = value.get::<i32>() {
                ui.move_service(src as usize, to);
                return true;
            }
            false
        });
        row.add_controller(drop);

        // Right click: the service context menu.
        let click = gtk::GestureClick::new();
        click.set_button(gdk::BUTTON_SECONDARY);
        let ui = self.clone();
        let row_weak = row.downgrade();
        click.connect_pressed(move |_, _, x, y| {
            if let Some(row) = row_weak.upgrade() {
                ui.show_context_menu(index, &row, x, y);
            }
        });
        row.add_controller(click);
    }

    /// Reorders service `from` to position `to`, persists and rebuilds.
    fn move_service(&self, from: usize, to: usize) {
        if from == to {
            return;
        }
        {
            let mut st = self.state.borrow_mut();
            if from >= st.services.len() || to >= st.services.len() {
                return;
            }
            let svc = st.services.remove(from);
            st.services.insert(to, svc);
        }
        self.save();
        self.refresh_sidebar();
    }

    /// Relative step through the service list (for next/previous shortcuts).
    fn step(&self, delta: i32) {
        let (len, cur) = {
            let st = self.state.borrow();
            let len = st.services.len() as i32;
            let cur = st
                .current
                .as_ref()
                .and_then(|id| st.services.iter().position(|s| &s.id == id))
                .map(|p| p as i32)
                .unwrap_or(0);
            (len, cur)
        };
        if len == 0 {
            return;
        }
        let next = (((cur + delta) % len) + len) % len;
        self.select_index(next as usize);
    }

    /// Shows the context menu for service `index`, anchored to `row`.
    fn show_context_menu(&self, index: usize, row: &gtk::ListBoxRow, x: f64, y: f64) {
        // Select the clicked service (the actions operate on the current one).
        self.select_index(index);

        let muted = self
            .state
            .borrow()
            .services
            .get(index)
            .map(|s| s.muted)
            .unwrap_or(false);

        // A popover with buttons that call the methods directly — we do not rely
        // on GAction resolution, which did not work in a menu-model over CEF.
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

    /// Wires a context-menu button: closes the popover, then runs `action`.
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

    /// Persists the settings (spell-check + media) in one place.
    fn persist_settings(&self) {
        config::save_settings(&config::Settings {
            spell_languages: self.spell.borrow().clone(),
            media_enabled: self.media.get(),
        });
    }

    /// Applies the current spell-check languages to every service and persists.
    fn apply_spell_languages(&self) {
        let langs = self.spell.borrow().clone();
        {
            let st = self.state.borrow();
            for view in st.views.values() {
                view.set_spell_languages(&langs);
            }
        }
        self.persist_settings();
    }

    /// Toggles media/WebRTC capture on every service (and persists).
    fn set_media_enabled(&self, enabled: bool) {
        self.media.set(enabled);
        {
            let st = self.state.borrow();
            for view in st.views.values() {
                view.set_media_enabled(enabled);
            }
        }
        self.persist_settings();
    }

    /// Dialog to choose the spell-check languages.
    fn show_spell_dialog(&self) {
        let dialog = adw::Dialog::builder()
            .title(gettext("Spell-check languages"))
            .content_width(420)
            .build();

        let group = adw::PreferencesGroup::builder()
            .title(gettext("Spell checking"))
            .description(gettext("Dictionaries installed on the system"))
            .build();

        let available = spellcheck::available_dictionaries();
        if available.is_empty() {
            let row = adw::ActionRow::builder()
                .title(gettext("No dictionaries installed"))
                .subtitle(gettext("Install one, e.g.: sudo pacman -S hunspell-en_us"))
                .build();
            group.add(&row);
        } else {
            for lang in &available {
                group.add(&self.spell_language_row(lang));
            }
        }

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(12)
            .margin_bottom(18)
            .margin_start(18)
            .margin_end(18)
            .build();
        content.append(&group);

        dialog.set_child(Some(&dialog_toolbar(&content)));
        dialog.present(Some(&self.window));
    }

    /// A single toggle row for a spell-check language.
    fn spell_language_row(&self, lang: &str) -> adw::SwitchRow {
        let active = self.spell.borrow().iter().any(|l| l == lang);
        let row = adw::SwitchRow::builder().title(lang).active(active).build();
        let ui = self.clone();
        let lang = lang.to_string();
        row.connect_active_notify(move |r| {
            {
                let mut selected = ui.spell.borrow_mut();
                if r.is_active() {
                    if !selected.contains(&lang) {
                        selected.push(lang.clone());
                    }
                } else {
                    selected.retain(|l| *l != lang);
                }
            }
            ui.apply_spell_languages();
        });
        row
    }

    /// Mutes/unmutes the current service (and persists).
    fn set_current_muted(&self, muted: bool) {
        let cur = self.state.borrow().current.clone();
        if let Some(id) = cur {
            let dnd = self.dnd.get();
            {
                let mut st = self.state.borrow_mut();
                if let Some(svc) = st.services.iter_mut().find(|s| s.id == id) {
                    svc.muted = muted;
                }
                if let Some(view) = st.views.get(&id) {
                    view.set_notifications_enabled(!muted && !dnd);
                }
            }
            self.save();
        }
    }

    /// Reapplies the notification state to every service (after DND changes).
    fn apply_all_notifications(&self) {
        let dnd = self.dnd.get();
        let st = self.state.borrow();
        for svc in &st.services {
            if let Some(view) = st.views.get(&svc.id) {
                view.set_notifications_enabled(!svc.muted && !dnd);
            }
        }
    }

    /// Shows the web view for service `idx` (creating it if needed).
    fn show_service_at(&self, idx: usize) {
        let (id, name) = {
            let st = self.state.borrow();
            match st.services.get(idx) {
                Some(s) => (s.id.clone(), s.name.clone()),
                None => return,
            }
        };

        self.stack.set_visible_child_name(&id);
        self.title.set_title(&name);
        self.state.borrow_mut().current = Some(id);
        self.split.set_show_content(true);
    }

    fn add_service(&self, name: &str, url: &str) {
        let id = {
            let st = self.state.borrow();
            config::make_id(&st.services, name)
        };
        self.state.borrow_mut().services.push(Service {
            id: id.clone(),
            name: name.to_string(),
            url: config::normalize_url(url),
            muted: false,
        });
        self.save();
        self.state.borrow_mut().current = Some(id);
        self.refresh_sidebar();
    }

    fn remove_current(&self) {
        let Some(id) = self.state.borrow().current.clone() else {
            return;
        };
        {
            let mut st = self.state.borrow_mut();
            st.services.retain(|s| s.id != id);
            if let Some(view) = st.views.remove(&id) {
                self.stack.remove(view.widget());
            }
            st.current = None;
        }
        self.save();
        self.refresh_sidebar();
    }

    fn current_view(&self) -> Option<engine::ServiceView> {
        let st = self.state.borrow();
        st.current.as_ref().and_then(|id| st.views.get(id).cloned())
    }

    fn save(&self) {
        config::save(&self.state.borrow().services);
    }
}

// ---------------------------------------------------------------------------
// Widget construction
// ---------------------------------------------------------------------------

/// The icon-only side rail (no header of its own).
fn build_service_list() -> gtk::ListBox {
    gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["navigation-sidebar", "rail"])
        .build()
}

/// The content area: a stack of web views, starting on the empty state.
fn build_content_stack() -> gtk::Stack {
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .hexpand(true)
        .build();
    stack.add_named(&empty_state(), Some(EMPTY_PAGE));
    stack
}

/// The single header bar spanning the whole window width.
fn build_primary_header(title: &adw::WindowTitle) -> adw::HeaderBar {
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(gettext("Main menu"))
        .menu_model(&primary_menu())
        .primary(true)
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
    header.pack_start(&reload_button);
    header.pack_start(&home_button);
    header.set_title_widget(Some(title));
    header
}

/// Wraps a widget in a vertically scrolling window (no horizontal scrollbar).
fn scrollable(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(child)
        .vexpand(true)
        .build()
}

/// Wraps dialog content below a plain header bar.
fn dialog_toolbar(content: &impl IsA<gtk::Widget>) -> adw::ToolbarView {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(content));
    toolbar
}

/// A flat, left-aligned button for the popover menus.
fn menu_item(label: &str) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("flat");
    if let Some(lbl) = button.child().and_downcast::<gtk::Label>() {
        lbl.set_xalign(0.0);
    }
    button
}

/// A rail row: just the service icon (favicon/initial), centered, with the name
/// shown as a tooltip on hover.
fn service_row(svc: &Service, icon: &gtk::Widget) -> gtk::ListBoxRow {
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
fn empty_state() -> adw::StatusPage {
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

fn primary_menu() -> gio::Menu {
    let menu = gio::Menu::new();

    let services = gio::Menu::new();
    services.append(Some(&gettext("Add service")), Some("win.add-service"));
    services.append(Some(&gettext("Remove current service")), Some("win.remove-service"));
    menu.append_section(None, &services);

    let preferences = gio::Menu::new();
    preferences.append(Some(&gettext("Do not disturb")), Some("win.toggle-dnd"));
    preferences.append(Some(&gettext("Camera, mic & calls")), Some("win.toggle-media"));
    preferences.append(Some(&gettext("Spell-check languages…")), Some("win.spell-languages"));
    menu.append_section(None, &preferences);

    let about = gio::Menu::new();
    about.append(Some(&gettext("About Syltr")), Some("app.about"));
    about.append(Some(&gettext("Quit")), Some("app.quit"));
    menu.append_section(None, &about);

    menu
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

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

fn wire_actions(app: &adw::Application, ui: &Ui) {
    add_action(ui, "add-service", show_add_dialog);
    add_action(ui, "reload", |ui| {
        if let Some(view) = ui.current_view() {
            view.reload();
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
    add_toggle_action(ui, "toggle-media", ui.media.get(), |ui, on| ui.set_media_enabled(on));

    // win.goto1..9 (Ctrl+1..9)
    for i in 1usize..=9 {
        add_action(ui, &format!("goto{i}"), move |ui| ui.select_index(i - 1));
        app.set_accels_for_action(&format!("win.goto{i}"), &[&format!("<Primary>{i}")]);
    }
    add_action(ui, "next-service", |ui| ui.step(1));
    app.set_accels_for_action("win.next-service", &["<Primary>Page_Down", "<Alt>Down"]);
    add_action(ui, "prev-service", |ui| ui.step(-1));
    app.set_accels_for_action("win.prev-service", &["<Primary>Page_Up", "<Alt>Up"]);

    // app.about
    {
        let win = ui.window.clone();
        let action = gio::SimpleAction::new("about", None);
        action.connect_activate(move |_, _| show_about(&win));
        app.add_action(&action);
    }
    // app.quit
    {
        let win = ui.window.clone();
        let action = gio::SimpleAction::new("quit", None);
        action.connect_activate(move |_, _| win.close());
        app.add_action(&action);
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
    }
    app.set_accels_for_action("win.reload", &["<Primary>r", "F5"]);
    app.set_accels_for_action("win.add-service", &["<Primary>n"]);
}

// ---------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------

/// The "Add service" dialog: the catalog plus a custom URL.
fn show_add_dialog(ui: &Ui) {
    let dialog = adw::Dialog::builder()
        .title(gettext("Add service"))
        .content_width(460)
        .content_height(600)
        .build();

    let (custom, add_button) = custom_group(ui, &dialog);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    content.append(&catalog_group(ui, &dialog));
    content.append(&custom);
    content.append(&add_button);

    dialog.set_child(Some(&dialog_toolbar(&scrollable(&content))));
    dialog.present(Some(&ui.window));
}

/// Preferences group listing the known services from the catalog.
fn catalog_group(ui: &Ui, dialog: &adw::Dialog) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title(gettext("Services")).build();
    for entry in catalog::CATALOG {
        let row = adw::ActionRow::builder()
            .title(entry.name)
            .subtitle(entry.url)
            .activatable(true)
            .build();
        row.add_prefix(&adw::Avatar::new(28, Some(entry.name), true));
        row.add_suffix(&gtk::Image::from_icon_name("list-add-symbolic"));

        let ui = ui.clone();
        let dialog = dialog.clone();
        let name = entry.name.to_string();
        let url = entry.url.to_string();
        row.connect_activated(move |_| {
            ui.add_service(&name, &url);
            dialog.close();
        });
        group.add(&row);
    }
    group
}

/// Preferences group to add any web service by URL, plus its "Add" button. The
/// button is returned separately so it sits below the group in the dialog.
fn custom_group(ui: &Ui, dialog: &adw::Dialog) -> (adw::PreferencesGroup, gtk::Button) {
    let group = adw::PreferencesGroup::builder()
        .title(gettext("Custom"))
        .description(gettext("Add any web service by URL."))
        .build();

    let name_row = adw::EntryRow::builder().title(gettext("Name")).build();
    let url_row = adw::EntryRow::builder().title(gettext("URL (https://…)")).build();
    group.add(&name_row);
    group.add(&url_row);

    let add_button = gtk::Button::builder()
        .label(gettext("Add"))
        .halign(gtk::Align::End)
        .margin_top(12)
        .css_classes(["suggested-action"])
        .build();

    let ui = ui.clone();
    let dialog = dialog.clone();
    add_button.connect_clicked(move |_| {
        let url = url_row.text().to_string();
        if url.trim().is_empty() {
            url_row.add_css_class("error");
            return;
        }
        let mut name = name_row.text().to_string();
        if name.trim().is_empty() {
            name = gettext("Service");
        }
        ui.add_service(&name, &url);
        dialog.close();
    });
    (group, add_button)
}

fn show_about(parent: &impl IsA<gtk::Widget>) {
    let about = adw::AboutDialog::builder()
        .application_name("Syltr")
        .application_icon(crate::APP_ID)
        .developer_name("Lucas Borges")
        .version(env!("CARGO_PKG_VERSION"))
        .comments(gettext("Franz-style messaging aggregator for GNOME."))
        .website("https://github.com/")
        .license_type(gtk::License::Gpl30)
        .build();
    about.present(Some(parent));
}
