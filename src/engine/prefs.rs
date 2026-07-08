//! Chromium spell-check preferences applied per request context.

use cef::*;

pub(crate) fn apply_spell_prefs(ctx: &RequestContext, langs: &[String]) {
    let enabled = !langs.is_empty();
    if let Some(mut v) = value_create() {
        v.set_bool(enabled as _);
        ctx.set_preference(
            Some(&CefString::from("browser.enable_spellchecking")),
            Some(&mut v),
            None,
        );
    }
    if let (Some(mut list), Some(mut val)) = (list_value_create(), value_create()) {
        list.set_size(langs.len());
        for (i, lang) in langs.iter().enumerate() {
            // Chromium expects a hyphen and region: pt_BR -> pt-BR.
            let code = lang.replace('_', "-");
            list.set_string(i, Some(&CefString::from(code.as_str())));
        }
        val.set_list(Some(&mut list));
        ctx.set_preference(
            Some(&CefString::from("spellcheck.dictionaries")),
            Some(&mut val),
            None,
        );
    }
}
