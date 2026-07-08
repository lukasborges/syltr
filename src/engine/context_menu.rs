//! In OSR the native context menu never shows, so we draw a GTK popover from the
//! model CEF provides (spelling suggestions, copy/paste, spell-check languages…).

use std::cell::Cell;
use std::rc::Rc;

use cef::*;
use gtk::prelude::*;
use gtk::gdk;

use super::render::RenderState;

wrap_context_menu_handler! {
    pub(crate) struct ContextMenuHandlerBuilder {
        state: Rc<RenderState>,
    }

    impl ContextMenuHandler {
        fn run_context_menu(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            params: Option<&mut ContextMenuParams>,
            model: Option<&mut MenuModel>,
            callback: Option<&mut RunContextMenuCallback>,
        ) -> ::std::os::raw::c_int {
            let (Some(params), Some(model), Some(callback)) = (params, model, callback) else {
                return 0;
            };
            if model.count() == 0 {
                return 0;
            }
            let (x, y) = (params.xcoord(), params.ycoord());

            let popover = gtk::Popover::new();
            popover.set_parent(self.state.area());
            popover.set_has_arrow(false);
            popover.add_css_class("menu");
            popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 1, 1)));

            let done = Rc::new(Cell::new(false));
            let menu = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .width_request(220)
                .build();
            let scroll = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .max_content_height(500)
                .propagate_natural_height(true)
                .child(&menu)
                .build();
            fill_menu(&menu, model, callback, &done, &popover);
            popover.set_child(Some(&scroll));

            {
                let cb = callback.clone();
                let done = done.clone();
                popover.connect_closed(move |p| {
                    if !done.get() {
                        cb.cancel();
                    }
                    p.unparent();
                });
            }
            popover.popup();
            1
        }
    }
}

impl ContextMenuHandlerBuilder {
    pub(crate) fn build(state: Rc<RenderState>) -> ContextMenuHandler {
        Self::new(state)
    }
}

/// Fills the popover with buttons from the CefMenuModel (recursing into submenus,
/// flattened under a heading). Each button calls callback.cont(id).
fn fill_menu(
    menu: &gtk::Box,
    model: &MenuModel,
    cb: &RunContextMenuCallback,
    done: &Rc<Cell<bool>>,
    pop: &gtk::Popover,
) {
    for i in 0..model.count() {
        let t = model.type_at(i);
        if t == MenuItemType::SEPARATOR {
            menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        } else if t == MenuItemType::SUBMENU {
            if let Some(sub) = model.sub_menu_at(i) {
                let header = gtk::Label::builder()
                    .label(menu_label(model, i))
                    .xalign(0.0)
                    .margin_start(8)
                    .margin_top(4)
                    .css_classes(["dim-label", "caption-heading"])
                    .build();
                menu.append(&header);
                fill_menu(menu, &sub, cb, done, pop);
            }
        } else {
            let mut label = menu_label(model, i);
            if model.is_checked_at(i) != 0 {
                label = format!("\u{2713} {label}");
            }
            let button = gtk::Button::with_label(&label);
            button.add_css_class("flat");
            button.set_sensitive(model.is_enabled_at(i) != 0);
            if let Some(lbl) = button.child().and_downcast::<gtk::Label>() {
                lbl.set_xalign(0.0);
            }
            let id = model.command_id_at(i);
            let cb = cb.clone();
            let done = done.clone();
            let pop = pop.clone();
            button.connect_clicked(move |_| {
                done.set(true);
                cb.cont(id, EventFlags::default());
                pop.popdown();
            });
            menu.append(&button);
        }
    }
}

fn menu_label(model: &MenuModel, i: usize) -> String {
    let raw = CefString::from(&model.label_at(i)).to_string();
    // Strip the Windows-style mnemonic marker ('&'); '&&' becomes '&'.
    raw.replace("&&", "\u{1}")
        .replace('&', "")
        .replace('\u{1}', "&")
}
