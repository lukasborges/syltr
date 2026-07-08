//! Service rail: building the rows, selection, reordering and add/remove.

use adw::prelude::*;
use gtk::{gdk, glib};

use super::widgets::service_row;
use super::{Ui, EMPTY_PAGE};
use crate::config::{self, Service};
use crate::engine;

impl Ui {
    /// Ensures a service's web view exists (loading in the background) and
    /// returns a clone. Called upfront for every service so favicons load and
    /// notifications arrive, like in Franz.
    pub(super) fn ensure_view(&self, svc: &Service) -> engine::ServiceView {
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

    pub(super) fn refresh_sidebar(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let services = self.state.borrow().services.clone();
        for (i, svc) in services.iter().enumerate() {
            let view = self.ensure_view(svc);
            let icon = view.icon();
            // The icon is reused across rows; detach it from its previous parent
            // before reattaching, otherwise the new row stays empty on reorder.
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

    pub(super) fn select_index(&self, idx: usize) {
        if let Some(row) = self.list.row_at_index(idx as i32) {
            self.list.select_row(Some(&row));
        }
    }

    /// Attaches drag-to-reorder and the context menu to a rail row.
    fn attach_row_controllers(&self, row: &gtk::ListBoxRow, index: usize) {
        let drag = gtk::DragSource::new();
        drag.set_actions(gdk::DragAction::MOVE);
        let from = index as i32;
        drag.connect_prepare(move |_, _, _| {
            Some(gdk::ContentProvider::for_value(&from.to_value()))
        });
        row.add_controller(drag);

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
    pub(super) fn step(&self, delta: i32) {
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

    /// Shows the web view for service `idx` (creating it if needed).
    pub(super) fn show_service_at(&self, idx: usize) {
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

    pub(super) fn add_service(&self, name: &str, url: &str) {
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

    pub(super) fn remove_current(&self) {
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

    pub(super) fn current_view(&self) -> Option<engine::ServiceView> {
        let st = self.state.borrow();
        st.current.as_ref().and_then(|id| st.views.get(id).cloned())
    }

    pub(super) fn save(&self) {
        config::save(&self.state.borrow().services);
    }
}
