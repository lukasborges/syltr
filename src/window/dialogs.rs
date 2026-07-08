//! Dialogs: add service, spell-check languages and about.

use adw::prelude::*;
use gettextrs::gettext;

use super::widgets::{dialog_toolbar, scrollable};
use super::Ui;
use crate::{catalog, spellcheck};

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

/// The "Add service" dialog: the catalog plus a custom URL.
pub(super) fn show_add_dialog(ui: &Ui) {
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

/// The custom-URL group plus its "Add" button, returned separately so the button
/// sits below the group in the dialog.
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

pub(super) fn show_about(parent: &impl IsA<gtk::Widget>) {
    let about = adw::AboutDialog::builder()
        .application_name("Syltr")
        .application_icon(crate::APP_ID)
        .developer_name("Lucas Borges")
        .version(env!("CARGO_PKG_VERSION"))
        .comments(gettext("All-in-one messaging aggregator for GNOME."))
        .website("https://github.com/")
        .license_type(gtk::License::Gpl30)
        .build();
    about.present(Some(parent));
}
