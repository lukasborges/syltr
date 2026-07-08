//! Download handler. Without one, CEF cancels every download. In OSR there is no
//! native "Save as" dialog, so we write straight to ~/Downloads (or the XDG
//! equivalent), avoiding overwriting existing files.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use cef::*;
use gtk::glib;
use gtk::prelude::*;

#[derive(Clone)]
struct SyltrDownloadHandler {
    /// Ids already notified — on_download_updated fires repeatedly.
    notified: Rc<RefCell<HashSet<u32>>>,
}

wrap_download_handler! {
    pub(crate) struct DownloadHandlerBuilder {
        handler: SyltrDownloadHandler,
    }

    impl DownloadHandler {
        fn can_download(
            &self,
            _browser: Option<&mut Browser>,
            _url: Option<&CefString>,
            _request_method: Option<&CefString>,
        ) -> ::std::os::raw::c_int {
            1
        }

        fn on_before_download(
            &self,
            _browser: Option<&mut Browser>,
            _download_item: Option<&mut DownloadItem>,
            suggested_name: Option<&CefString>,
            callback: Option<&mut BeforeDownloadCallback>,
        ) -> ::std::os::raw::c_int {
            if let Some(cb) = callback {
                let name = suggested_name
                    .map(|n| n.to_string())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| "download".to_string());
                let path = unique_path(&downloads_dir(), &name);
                let path_str = path.to_string_lossy();
                // show_dialog = 0: write directly, no dialog (unavailable in OSR).
                cb.cont(Some(&CefString::from(path_str.as_ref())), 0);
            }
            1
        }

        fn on_download_updated(
            &self,
            _browser: Option<&mut Browser>,
            download_item: Option<&mut DownloadItem>,
            _callback: Option<&mut DownloadItemCallback>,
        ) {
            let Some(item) = download_item else { return };
            if item.is_complete() != 1 {
                return;
            }
            if !self.handler.notified.borrow_mut().insert(item.id()) {
                return;
            }
            let full = CefString::from(&item.full_path()).to_string();
            let name = Path::new(&full)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&full)
                .to_string();
            eprintln!("[syltr] download complete: {full}");
            if let Some(app) = gtk::gio::Application::default() {
                let notif = gtk::gio::Notification::new("Download complete");
                notif.set_body(Some(&name));
                app.send_notification(Some(&format!("syltr-download-{}", item.id())), &notif);
            }
        }
    }
}

impl DownloadHandlerBuilder {
    pub(crate) fn build() -> DownloadHandler {
        Self::new(SyltrDownloadHandler {
            notified: Rc::new(RefCell::new(HashSet::new())),
        })
    }
}

fn downloads_dir() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Downloads)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("Downloads")))
        .unwrap_or_else(std::env::temp_dir)
}

/// A `dir/name` path that does not collide: appends " (1)", " (2)"… if needed.
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = path.extension().and_then(|s| s.to_str());
    for n in 1.. {
        let filename = match ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dir.join(filename);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}
