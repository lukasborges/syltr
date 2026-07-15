//! Dialogs: add service, spell-check languages and about.

use adw::prelude::*;
use gettextrs::gettext;

use super::widgets::{dialog_toolbar, scrollable, service_icon};
use super::Ui;
use crate::{catalog, engine, spellcheck};

impl Ui {
    pub(super) fn show_spell_dialog(&self) {
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

    fn spell_language_row(&self, lang: &str) -> adw::SwitchRow {
        let active = self.spell.borrow().contains(&lang.to_string());
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
}

/// The "Add service" dialog: a searchable, category-grouped catalog with a
/// header button for adding a service by custom URL.
pub(super) fn show_add_dialog(ui: &Ui) {
    let dialog = adw::Dialog::builder()
        .title(gettext("Add service"))
        .content_width(460)
        .content_height(600)
        .build();

    let search = gtk::SearchEntry::builder()
        .placeholder_text(gettext("Search services"))
        .margin_top(12)
        .margin_start(18)
        .margin_end(18)
        .margin_bottom(6)
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(6)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    content.append(&catalog_groups(ui, &dialog, &search));

    // Search stays pinned above the scrolling catalog.
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.append(&search);
    outer.append(&scrollable(&content));

    let custom_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(gettext("Add a custom service by URL"))
        .build();
    let ui_custom = ui.clone();
    let dialog_custom = dialog.clone();
    custom_button.connect_clicked(move |_| show_custom_dialog(&ui_custom, &dialog_custom));

    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.pack_end(&custom_button);
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&outer));

    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&ui.window));
}

/// The "Edit service" dialog: name, URL and an optional custom user-agent.
/// Extensible — further per-service settings can be added as rows here.
pub(super) fn show_edit_dialog(ui: &Ui, index: usize) {
    let (name, url, custom_ua) = {
        let st = ui.state.borrow();
        let Some(svc) = st.services.get(index) else {
            return;
        };
        (svc.name.clone(), svc.url.clone(), svc.user_agent.clone())
    };

    let dialog = adw::Dialog::builder()
        .title(gettext("Edit service"))
        .content_width(520)
        .build();

    let group = adw::PreferencesGroup::builder()
        .title(gettext("Service"))
        .build();
    let name_row = adw::EntryRow::builder().title(gettext("Name")).build();
    name_row.set_text(&name);
    let url_row = adw::EntryRow::builder()
        .title(gettext("URL (https://…)"))
        .build();
    url_row.set_text(&url);
    group.add(&name_row);
    group.add(&url_row);

    let ua_group = adw::PreferencesGroup::builder()
        .title(gettext("User agent"))
        .description(gettext(
            "Browser identification sent to this service. Leave empty to use the default.",
        ))
        .build();
    let ua_row = adw::EntryRow::builder()
        .title(gettext("Custom user agent"))
        .build();
    ua_row.set_text(custom_ua.as_deref().unwrap_or_default());
    let default_row = adw::ActionRow::builder()
        .title(gettext("Default"))
        .subtitle(engine::resolve_user_agent(&url, None))
        .build();
    default_row.set_subtitle_selectable(true);
    ua_group.add(&ua_row);
    ua_group.add(&default_row);

    let save_button = gtk::Button::builder()
        .label(gettext("Save"))
        .halign(gtk::Align::End)
        .margin_top(12)
        .css_classes(["suggested-action"])
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    content.append(&group);
    content.append(&ua_group);
    content.append(&save_button);

    let ui_save = ui.clone();
    let dialog_ref = dialog.clone();
    save_button.connect_clicked(move |_| {
        let url = url_row.text().to_string();
        if url.trim().is_empty() {
            url_row.add_css_class("error");
            return;
        }
        let mut name = name_row.text().to_string();
        if name.trim().is_empty() {
            name = gettext("Service");
        }
        let ua = ua_row.text().to_string();
        let ua = (!ua.trim().is_empty()).then_some(ua);
        ui_save.update_service(index, &name, &url, ua);
        dialog_ref.close();
    });

    dialog.set_child(Some(&dialog_toolbar(&content)));
    dialog.present(Some(&ui.window));
}

/// Builds one preferences group per catalog category and wires `search` to
/// filter the rows (hiding groups that end up empty).
fn catalog_groups(ui: &Ui, dialog: &adw::Dialog, search: &gtk::SearchEntry) -> gtk::Box {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .build();

    // (group, [(row, searchable text)]) captured by the filter closure.
    let mut groups: Vec<(adw::PreferencesGroup, Vec<(adw::ActionRow, String)>)> = Vec::new();

    for category in catalog::categories() {
        let group = adw::PreferencesGroup::builder()
            .title(gettext(category))
            .build();
        let mut rows = Vec::new();
        for entry in catalog::CATALOG.iter().filter(|e| e.category == category) {
            let row = adw::ActionRow::builder()
                .title(entry.name)
                .subtitle(entry.url)
                .activatable(true)
                .build();
            row.add_prefix(&service_icon(entry));
            row.add_suffix(&gtk::Image::from_icon_name("list-add-symbolic"));

            let ui = ui.clone();
            let dialog = dialog.clone();
            let name = entry.name.to_string();
            let url = entry.url.to_string();
            row.connect_activated(move |_| {
                ui.begin_add(&name, &url);
                dialog.close();
            });

            group.add(&row);
            rows.push((row, format!("{} {}", entry.name, entry.url).to_lowercase()));
        }
        container.append(&group);
        groups.push((group, rows));
    }

    search.connect_search_changed(move |entry| {
        let query = entry.text().to_lowercase();
        let query = query.trim();
        for (group, rows) in &groups {
            let mut any_visible = false;
            for (row, haystack) in rows {
                let matches = query.is_empty() || haystack.contains(query);
                row.set_visible(matches);
                any_visible |= matches;
            }
            group.set_visible(any_visible);
        }
    });

    container
}

/// The "Add custom service" dialog: a name and URL, reached from the header
/// button of the catalog dialog.
fn show_custom_dialog(ui: &Ui, parent: &adw::Dialog) {
    let dialog = adw::Dialog::builder()
        .title(gettext("Custom service"))
        .content_width(420)
        .build();

    let group = adw::PreferencesGroup::builder()
        .description(gettext("Add any web service by URL."))
        .build();
    let name_row = adw::EntryRow::builder().title(gettext("Name")).build();
    let url_row = adw::EntryRow::builder()
        .title(gettext("URL (https://…)"))
        .build();
    group.add(&name_row);
    group.add(&url_row);

    let add_button = gtk::Button::builder()
        .label(gettext("Add"))
        .halign(gtk::Align::End)
        .margin_top(12)
        .css_classes(["suggested-action"])
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    content.append(&group);
    content.append(&add_button);

    let ui_add = ui.clone();
    let dialog_ref = dialog.clone();
    let parent = parent.clone();
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
        ui_add.begin_add(&name, &url);
        dialog_ref.close();
        parent.close();
    });

    dialog.set_child(Some(&dialog_toolbar(&content)));
    dialog.present(Some(&ui.window));
}

/// Prompts for a distinct name when adding another instance of a service that
/// already exists, so the copies can be told apart in the rail.
pub(super) fn show_name_instance_dialog(ui: &Ui, url: &str, suggested: &str) {
    let dialog = adw::Dialog::builder()
        .title(gettext("Name this instance"))
        .content_width(420)
        .build();

    let group = adw::PreferencesGroup::builder()
        .description(gettext(
            "You already have this service. Name this copy to tell them apart \
             (e.g. \"Work\", \"Personal\").",
        ))
        .build();
    let name_row = adw::EntryRow::builder().title(gettext("Name")).build();
    name_row.set_text(suggested);
    group.add(&name_row);

    let add_button = gtk::Button::builder()
        .label(gettext("Add"))
        .halign(gtk::Align::End)
        .margin_top(12)
        .css_classes(["suggested-action"])
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    content.append(&group);
    content.append(&add_button);

    let ui_add = ui.clone();
    let dialog_ref = dialog.clone();
    let url = url.to_string();
    add_button.connect_clicked(move |_| {
        let mut name = name_row.text().to_string();
        if name.trim().is_empty() {
            name = gettext("Service");
        }
        ui_add.add_service(&name, &url);
        dialog_ref.close();
    });

    dialog.set_child(Some(&dialog_toolbar(&content)));
    dialog.present(Some(&ui.window));
}

pub(super) fn show_about(parent: &impl IsA<gtk::Widget>) {
    let about = adw::AboutDialog::builder()
        .application_name("Syltr")
        .application_icon(crate::APP_ID)
        .developer_name("Lucas Borges")
        .version(env!("CARGO_PKG_VERSION"))
        .comments(gettext("All-in-one messaging aggregator for GNOME."))
        .website("https://github.com/lukasborges/syltr")
        .issue_url("https://github.com/lukasborges/syltr/issues")
        .license_type(gtk::License::Gpl30)
        .build();
    about.present(Some(parent));
}
