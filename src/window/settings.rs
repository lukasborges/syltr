//! Notification (mute/DND) and preference (spell-check) toggles.

use super::Ui;
use crate::config;

impl Ui {
    fn persist_settings(&self) {
        config::save_settings(&config::Settings {
            spell_languages: self.spell.borrow().clone(),
        });
    }

    /// Applies the current spell-check languages to every service and persists.
    pub(super) fn apply_spell_languages(&self) {
        let langs = self.spell.borrow().clone();
        {
            let st = self.state.borrow();
            for view in st.views.values() {
                view.set_spell_languages(&langs);
            }
        }
        self.persist_settings();
    }

    /// Mutes/unmutes the current service (and persists).
    pub(super) fn set_current_muted(&self, muted: bool) {
        let Some(id) = self.state.borrow().current.clone() else {
            return;
        };
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

    /// Reapplies the notification state to every service (after DND changes).
    pub(super) fn apply_all_notifications(&self) {
        let dnd = self.dnd.get();
        let st = self.state.borrow();
        for svc in &st.services {
            if let Some(view) = st.views.get(&svc.id) {
                view.set_notifications_enabled(!svc.muted && !dnd);
            }
        }
    }
}
