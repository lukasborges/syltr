//! Service rail: instances of the same service are grouped under one icon;
//! selection, reordering, add/remove and the instance chooser popover.

use std::rc::Rc;

use adw::prelude::*;
use gettextrs::gettext;
use gtk::{gdk, glib};

use super::dialogs::show_name_instance_dialog;
use super::widgets::{menu_item, service_row};
use super::{Ui, DISABLED_PAGE, EMPTY_PAGE, WELCOME_PAGE};
use crate::config::{self, Service};
use crate::icon::ServiceIcon;
use crate::{catalog, engine};

/// Groups the services by URL, preserving each group's first-occurrence order.
fn group_services(services: &[Service]) -> Vec<Vec<Service>> {
    let mut order: Vec<&str> = Vec::new();
    let mut groups: Vec<Vec<Service>> = Vec::new();
    for svc in services {
        if let Some(pos) = order.iter().position(|u| *u == svc.url) {
            groups[pos].push(svc.clone());
        } else {
            order.push(&svc.url);
            groups.push(vec![svc.clone()]);
        }
    }
    groups
}

fn should_keep_view(service: &Service, current: Option<&str>) -> bool {
    !service.disabled && (service.background || current.is_some_and(|id| id == service.id.as_str()))
}

fn current_group_index(groups: &[Vec<Service>], current: Option<&str>) -> Option<usize> {
    current.and_then(|id| {
        groups
            .iter()
            .position(|group| group.iter().any(|service| service.id == id))
    })
}

impl Ui {
    /// Ensures a service's web view exists and returns a clone. Views are made
    /// on demand for the selected service and eagerly only for services whose
    /// background activity is enabled.
    pub(super) fn ensure_view(&self, svc: &Service, active: bool) -> engine::ServiceView {
        if let Some(view) = self.state.borrow().views.get(&svc.id) {
            let view = view.clone();
            view.set_active(active);
            return view;
        }
        let view = engine::ServiceView::new(
            &svc.id,
            &svc.name,
            &svc.url,
            svc.user_agent.as_deref(),
            &config::session_dir(&svc.id),
            &self.app,
            self.dnd.clone(),
            svc.muted,
            &self.spell.borrow(),
        );
        self.stack.add_named(view.widget(), Some(&svc.id));
        self.state
            .borrow_mut()
            .views
            .insert(svc.id.clone(), view.clone());
        view.set_active(active);
        view
    }

    /// Mirrors the stack selection into the engine. Background-enabled
    /// services stay connected, but enter the lightweight visual state.
    fn sync_view_activity(&self) {
        let state = self.state.borrow();
        let current = state.current.as_deref();
        for (id, view) in &state.views {
            view.set_active(current.is_some_and(|current| current == id));
        }
    }

    /// Removes a web view and its rail callback so WebKit can release all of
    /// the service's page resources.
    fn unload_view(&self, id: &str) -> bool {
        let view = self.state.borrow_mut().views.remove(id);
        if let Some(view) = view {
            view.clear_on_change();
            self.stack.remove(view.widget());
            true
        } else {
            false
        }
    }

    /// Drops views that are disabled or neither selected nor allowed to run in
    /// the background. Returns whether any view was removed.
    fn reconcile_views(&self) -> bool {
        let unload = {
            let st = self.state.borrow();
            let current = st.current.as_deref();
            st.views
                .keys()
                .filter(|id| {
                    st.services
                        .iter()
                        .find(|service| service.id.as_str() == id.as_str())
                        .is_none_or(|service| !should_keep_view(service, current))
                })
                .cloned()
                .collect::<Vec<_>>()
        };

        let mut removed = false;
        for id in unload {
            removed |= self.unload_view(&id);
        }
        removed
    }

    fn schedule_sidebar_refresh(&self) {
        let ui = self.clone();
        glib::idle_add_local_once(move || ui.refresh_sidebar());
    }

    fn groups(&self) -> Vec<Vec<Service>> {
        group_services(&self.state.borrow().services)
    }

    pub(super) fn refresh_sidebar(&self) {
        self.reconcile_views();
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let groups = self.groups();
        let current = self.state.borrow().current.clone();
        for (i, group) in groups.iter().enumerate() {
            let views: Vec<engine::ServiceView> = group
                .iter()
                .filter(|svc| should_keep_view(svc, current.as_deref()))
                .map(|svc| self.ensure_view(svc, current.as_deref() == Some(svc.id.as_str())))
                .collect();
            let rep = &group[0];

            let bundled_icon = catalog::CATALOG
                .iter()
                .find(|entry| entry.url == rep.url)
                .map(|entry| format!("/dev/syltr/Syltr/icons/{}.svg", entry.key));
            let cached_favicon = group
                .iter()
                .find_map(|service| engine::cached_favicon(&config::session_dir(&service.id)));
            let icon =
                ServiceIcon::new(&rep.name, cached_favicon.as_ref(), bundled_icon.as_deref());
            icon.set_stacked(group.len() > 1);
            icon.set_dimmed(group.iter().all(|service| service.disabled));

            // The grouped icon aggregates the unread of every instance and shows
            // the shared favicon; any instance's change re-runs this.
            let update: Rc<dyn Fn()> = {
                let icon = icon.clone();
                let state = Rc::downgrade(&self.state);
                let ids = group
                    .iter()
                    .map(|service| service.id.clone())
                    .collect::<Vec<_>>();
                Rc::new(move || {
                    let Some(state) = state.upgrade() else {
                        return;
                    };
                    let state = state.borrow();
                    let active = ids
                        .iter()
                        .filter_map(|id| state.views.get(id))
                        .collect::<Vec<_>>();
                    let total: u32 = active.iter().map(|view| view.unread()).sum();
                    icon.set_badge(total);
                    if let Some(rep) = active.first() {
                        icon.set_favicon(rep.favicon().as_ref());
                    }
                })
            };
            update();
            for view in &views {
                let update = update.clone();
                view.set_on_change(move || update());
            }

            let row = service_row(rep, icon.widget());
            self.attach_row_controllers(&row, i);
            self.list.append(&row);
        }

        if groups.is_empty() {
            self.stack.set_visible_child_name(EMPTY_PAGE);
            self.title.set_title("Syltr");
            self.state.borrow_mut().current = None;
        } else if let Some(idx) = current_group_index(&groups, current.as_deref()) {
            self.select_index(idx);
        } else {
            self.list.set_selection_mode(gtk::SelectionMode::None);
            self.stack.set_visible_child_name(WELCOME_PAGE);
            self.title.set_title("Syltr");
            self.split.set_show_content(true);
        }
        self.sync_view_activity();
    }

    pub(super) fn select_index(&self, idx: usize) {
        self.list.set_selection_mode(gtk::SelectionMode::Single);
        if let Some(row) = self.list.row_at_index(idx as i32) {
            self.list.select_row(Some(&row));
        }
    }

    /// Activates a rail group from a keyboard shortcut.
    pub(super) fn activate_index(&self, idx: usize) {
        if let Some(row) = self.list.row_at_index(idx as i32) {
            self.on_row_activated(idx, &row);
        }
    }

    /// Attaches drag-to-reorder and the context menu to a rail row (one per
    /// service group).
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
                ui.move_group(src as usize, to);
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

    /// Reorders whole groups: dragging a service icon moves all its instances.
    fn move_group(&self, from: usize, to: usize) {
        if from == to {
            return;
        }
        {
            let mut st = self.state.borrow_mut();
            let mut order: Vec<String> = Vec::new();
            for svc in &st.services {
                if !order.contains(&svc.url) {
                    order.push(svc.url.clone());
                }
            }
            if from >= order.len() || to >= order.len() {
                return;
            }
            let url = order.remove(from);
            order.insert(to, url);
            let mut reordered = Vec::with_capacity(st.services.len());
            for url in &order {
                for svc in &st.services {
                    if &svc.url == url {
                        reordered.push(svc.clone());
                    }
                }
            }
            st.services = reordered;
        }
        self.save();
        self.refresh_sidebar();
    }

    /// Relative step through the rail groups (for next/previous shortcuts).
    pub(super) fn step(&self, delta: i32) {
        let groups = self.groups();
        let len = groups.len() as i32;
        if len == 0 {
            return;
        }
        let current = self.state.borrow().current.clone();
        let cur = current
            .as_deref()
            .and_then(|id| groups.iter().position(|g| g.iter().any(|s| s.id == id)))
            .map(|position| position as i32);
        let next = match cur {
            Some(cur) => (((cur + delta) % len) + len) % len,
            None if delta < 0 => len - 1,
            None => 0,
        };
        self.activate_index(next as usize);
    }

    /// Makes the given group's active instance current — the current one if it
    /// belongs to the group, otherwise the first.
    pub(super) fn show_group_instance_at(&self, group_idx: usize) {
        let groups = self.groups();
        let Some(group) = groups.get(group_idx) else {
            return;
        };
        let current = self.state.borrow().current.clone();
        let active = group
            .iter()
            .find(|s| current.as_deref() == Some(s.id.as_str()))
            .unwrap_or(&group[0]);
        self.show_instance(&active.id);
    }

    fn show_instance(&self, id: &str) {
        let service = {
            let state = self.state.borrow();
            state
                .services
                .iter()
                .find(|service| service.id == id)
                .cloned()
        };
        let Some(service) = service else {
            return;
        };

        let created = !service.disabled && !self.state.borrow().views.contains_key(id);
        if !service.disabled {
            self.ensure_view(&service, true);
        }

        if service.disabled {
            self.stack.set_visible_child_name(DISABLED_PAGE);
        } else {
            self.stack.set_visible_child_name(id);
        }
        self.title.set_title(&service.name);
        self.state.borrow_mut().current = Some(id.to_string());
        self.split.set_show_content(true);
        self.sync_view_activity();

        let unloaded = self.reconcile_views();
        if created || unloaded {
            self.schedule_sidebar_refresh();
        }
    }

    /// A click on a group with several instances opens the instance chooser.
    pub(super) fn on_row_activated(&self, group_idx: usize, row: &gtk::ListBoxRow) {
        self.select_index(group_idx);
        let groups = self.groups();
        let Some(group) = groups.get(group_idx) else {
            return;
        };
        if group.len() > 1 {
            self.show_instance_popover(group, row);
        } else {
            self.show_group_instance_at(group_idx);
        }
    }

    fn show_instance_popover(&self, group: &[Service], row: &gtk::ListBoxRow) {
        let popover = gtk::Popover::new();
        popover.set_parent(&self.window);
        popover.set_has_arrow(false);
        popover.set_position(gtk::PositionType::Right);
        popover.add_css_class("menu");
        if let Some(p) = row.compute_point(&self.window, &gtk::graphene::Point::new(0.0, 0.0)) {
            popover.set_pointing_to(Some(&gdk::Rectangle::new(
                p.x() as i32,
                p.y() as i32,
                row.width(),
                row.height(),
            )));
        }

        let menu = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(220)
            .build();

        let current = self.state.borrow().current.clone();
        for svc in group {
            let unread = self
                .state
                .borrow()
                .views
                .get(&svc.id)
                .map(|v| v.unread())
                .unwrap_or(0);
            let label = if unread > 0 {
                format!("{}  ({unread})", svc.name)
            } else {
                svc.name.clone()
            };
            let item = menu_item(&label);
            if current.as_deref() == Some(svc.id.as_str()) {
                item.add_css_class("instance-current");
            }
            let ui = self.clone();
            let pop = popover.clone();
            let id = svc.id.clone();
            item.connect_clicked(move |_| {
                pop.popdown();
                ui.show_instance(&id);
            });
            menu.append(&item);
        }

        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        let base = catalog::CATALOG
            .iter()
            .find(|e| e.url == group[0].url)
            .map(|e| e.name.to_string())
            .unwrap_or_else(|| group[0].name.clone());
        let url = group[0].url.clone();
        let add = menu_item(&gettext("Add another instance"));
        let ui = self.clone();
        let pop = popover.clone();
        add.connect_clicked(move |_| {
            pop.popdown();
            ui.begin_add(&base, &url);
        });
        menu.append(&add);

        popover.set_child(Some(&menu));
        popover.connect_closed(|p| p.unparent());
        popover.popup();
    }

    /// Entry point from the Add dialog. Adds directly on the first instance of a
    /// URL; when another instance already exists, prompts for a distinct name so
    /// the copies are told apart in the chooser.
    pub(super) fn begin_add(&self, name: &str, url: &str) {
        let norm = config::normalize_url(url);
        let existing = self
            .state
            .borrow()
            .services
            .iter()
            .filter(|s| s.url == norm)
            .count();
        if existing > 0 {
            let suggested = format!("{name} ({})", existing + 1);
            show_name_instance_dialog(self, &norm, &suggested);
        } else {
            self.add_service(name, url);
        }
    }

    pub(super) fn add_service(&self, name: &str, url: &str) {
        let url = config::normalize_url(url);
        let id = {
            let st = self.state.borrow();
            config::make_id(&st.services, name)
        };
        self.state.borrow_mut().services.push(Service {
            id: id.clone(),
            name: name.to_string(),
            background: catalog::background_by_default(&url),
            url,
            muted: false,
            disabled: false,
            user_agent: None,
        });
        self.save();
        self.state.borrow_mut().current = Some(id);
        self.refresh_sidebar();
    }

    /// Applies edits from the "Edit service" dialog. A changed URL rebuilds the
    /// view (so it loads the new home and UA); a changed UA alone just reloads
    /// the live view. Background changes are reconciled during the rail refresh.
    pub(super) fn update_service(
        &self,
        index: usize,
        name: &str,
        url: &str,
        user_agent: Option<String>,
        background: bool,
    ) {
        let new_url = config::normalize_url(url);
        let (id, url_changed, ua_changed) = {
            let mut st = self.state.borrow_mut();
            let Some(svc) = st.services.get_mut(index) else {
                return;
            };
            let url_changed = svc.url != new_url;
            let ua_changed = svc.user_agent != user_agent;
            svc.name = name.to_string();
            svc.url = new_url;
            svc.user_agent = user_agent.clone();
            svc.background = background;
            (svc.id.clone(), url_changed, ua_changed)
        };
        if url_changed {
            self.unload_view(&id);
        } else if ua_changed {
            if let Some(view) = self.state.borrow().views.get(&id) {
                view.set_user_agent(user_agent.as_deref());
            }
        }
        self.save();
        self.refresh_sidebar();
    }

    /// Enables/disables the current service. Disabling frees its web view (no
    /// resources, no notifications); the service stays in the list, dimmed in the
    /// rail, and re-enabling reloads it.
    pub(super) fn set_current_disabled(&self, disabled: bool) {
        let Some(id) = self.state.borrow().current.clone() else {
            return;
        };
        if let Some(svc) = self
            .state
            .borrow_mut()
            .services
            .iter_mut()
            .find(|s| s.id == id)
        {
            svc.disabled = disabled;
        }
        if disabled {
            self.unload_view(&id);
        }
        self.save();
        self.refresh_sidebar();
    }

    pub(super) fn remove_current(&self) {
        let Some(id) = self.state.borrow().current.clone() else {
            return;
        };
        self.state.borrow_mut().services.retain(|s| s.id != id);
        self.unload_view(&id);
        self.state.borrow_mut().current = None;
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

#[cfg(test)]
mod tests;
