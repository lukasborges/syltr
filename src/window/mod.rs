//! Main window — GNOME layout with AdwNavigationSplitView: a service rail on the
//! left, a stack of web views on the right.
//!
//! `Ui` is a cloneable handle to the window and its widgets; its behavior is
//! split across the submodules by responsibility (sidebar, settings, context
//! menu, dialogs, actions), and the widget builders live in `widgets`.

mod actions;
mod context_menu;
mod dialogs;
mod settings;
mod sidebar;
mod widgets;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use gettextrs::gettext;

use actions::wire_actions;
use widgets::{build_content_stack, build_primary_header, build_service_list, scrollable};

use crate::config::{self, Service};
use crate::engine;

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
