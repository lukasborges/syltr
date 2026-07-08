//! Handle to a service's CEF browser and host, filled asynchronously once the
//! browser has been created (see the life-span handler).

use std::cell::RefCell;
use std::rc::Rc;

use cef::{Browser, BrowserHost, Frame, ImplBrowser};

pub struct BrowserSlot {
    browser: RefCell<Option<Browser>>,
    host: RefCell<Option<BrowserHost>>,
}

impl BrowserSlot {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            browser: RefCell::new(None),
            host: RefCell::new(None),
        })
    }

    /// Records the browser and host once CEF has created them.
    pub(crate) fn fill(&self, browser: Browser, host: Option<BrowserHost>) {
        *self.browser.borrow_mut() = Some(browser);
        *self.host.borrow_mut() = host;
    }

    pub fn host(&self) -> Option<BrowserHost> {
        self.host.borrow().clone()
    }

    pub(crate) fn browser(&self) -> Option<Browser> {
        self.browser.borrow().clone()
    }

    pub fn main_frame(&self) -> Option<Frame> {
        self.browser().and_then(|b| b.main_frame())
    }
}
