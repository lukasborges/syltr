//! Workaround for a CEF 149 bug: following a cross-origin redirect of a
//! `no-cors` subresource (an image) fails with `ERR_INVALID_ARGUMENT`. Google
//! Chat serves attachments and custom emoji exactly this way:
//!
//!   <img src="chat.google.com/api/get_attachment_url?..">  (same-origin)
//!        -> 302 -> lh7-eu.googleusercontent.com/chat_attachment/..  (cross-origin)
//!
//! The final image loads fine when requested directly; only *following the
//! redirect* in the renderer breaks. So we intercept these URLs and fetch the
//! image via `CefURLRequest` (the network service, which follows the 302 without
//! the bug), returning the bytes as a **same-origin 200** response — the
//! renderer never sees the redirect.
//!
//! Threads: the `ResourceHandler` runs on the IO thread, but `urlrequest_create`
//! (with a `RequestContext`, to inherit cookies) can only run on the UI thread.
//! So `open()` posts a task to the UI thread that creates the `URLRequest`; the
//! state is shared via `Arc<Mutex<..>>` (the cef wrappers are `Send`).

use std::sync::{Arc, Mutex};

use cef::rc::Rc as _;
use cef::*;

/// URLs whose `<img>` loading triggers the Chat redirect bug.
fn should_intercept(url: &str) -> bool {
    url.contains("/api/get_attachment_url") || url.contains("/api/get_custom_emoji_image")
}

#[derive(Default)]
struct FetchState {
    data: Vec<u8>,
    read_pos: usize,
    mime: String,
    status: i32,
    open_cb: Option<Callback>,
    request: Option<Urlrequest>,
}

type SharedState = Arc<Mutex<FetchState>>;

// UrlrequestClient — accumulates the bytes and, on completion, releases open().
// (runs on the UI thread, where the URLRequest was created)

wrap_urlrequest_client! {
    struct FetchClientBuilder {
        state: SharedState,
    }

    impl UrlrequestClient {
        fn on_download_data(
            &self,
            _request: Option<&mut Urlrequest>,
            data: *const u8,
            data_length: usize,
        ) {
            if data.is_null() || data_length == 0 {
                return;
            }
            let slice = unsafe { std::slice::from_raw_parts(data, data_length) };
            if let Ok(mut st) = self.state.lock() {
                st.data.extend_from_slice(slice);
            }
        }

        fn on_request_complete(&self, request: Option<&mut Urlrequest>) {
            let (cb, ok) = {
                let mut st = self.state.lock().unwrap();
                if let Some(req) = request {
                    if let Some(resp) = req.response() {
                        st.status = resp.status();
                        st.mime = CefString::from(&resp.mime_type()).to_string();
                    }
                }
                let ok = !st.data.is_empty()
                    && (st.status == 0 || (200..400).contains(&st.status));
                (st.open_cb.take(), ok)
            };
            if let Some(cb) = cb {
                if ok {
                    cb.cont();
                } else {
                    cb.cancel();
                }
            }
        }
    }
}

impl FetchClientBuilder {
    fn build(state: SharedState) -> UrlrequestClient {
        Self::new(state)
    }
}

// Task — creates the URLRequest on the UI thread.

wrap_task! {
    struct FetchTaskBuilder {
        state: SharedState,
        ctx: Option<RequestContext>,
        url: String,
    }

    impl Task {
        fn execute(&self) {
            let Some(mut req) = request_create() else {
                return;
            };
            req.set_url(Some(&CefString::from(self.url.as_str())));
            req.set_method(Some(&CefString::from("GET")));
            // UR_FLAG_ALLOW_STORED_CREDENTIALS (8): without it the CefURLRequest
            // does not send cookies and the endpoint responds with the login page.
            req.set_flags(8);
            // The endpoint does content negotiation: without an image Accept it
            // returns HTML instead of redirecting to the file.
            let hdr = |req: &mut Request, k: &str, v: &str| {
                req.set_header_by_name(
                    Some(&CefString::from(k)),
                    Some(&CefString::from(v)),
                    1,
                );
            };
            hdr(
                &mut req,
                "Accept",
                "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8",
            );
            // Without these the endpoint returns an HTML page instead of the 302->image.
            hdr(&mut req, "Sec-Fetch-Dest", "image");
            hdr(&mut req, "Sec-Fetch-Mode", "no-cors");
            hdr(&mut req, "Sec-Fetch-Site", "same-origin");
            req.set_referrer(
                Some(&CefString::from("https://chat.google.com/")),
                ReferrerPolicy::default(),
            );

            let mut client = FetchClientBuilder::build(self.state.clone());
            let mut ctx = self.ctx.clone();
            let urlreq = urlrequest_create(Some(&mut req), Some(&mut client), ctx.as_mut());
            if let Ok(mut st) = self.state.lock() {
                st.request = urlreq;
            }
        }
    }
}

impl FetchTaskBuilder {
    fn build(state: SharedState, ctx: Option<RequestContext>, url: String) -> Task {
        Self::new(state, ctx, url)
    }
}

// ResourceHandler — fetches the image via URLRequest and serves the bytes.
// (open/read/response_headers run on the IO thread)

wrap_resource_handler! {
    struct FetchHandlerBuilder {
        state: SharedState,
        ctx: Option<RequestContext>,
    }

    impl ResourceHandler {
        fn open(
            &self,
            request: Option<&mut Request>,
            handle_request: Option<&mut ::std::os::raw::c_int>,
            callback: Option<&mut Callback>,
        ) -> ::std::os::raw::c_int {
            let Some(req_in) = request else {
                return 0;
            };
            // Fetch exactly the same URL (the token/parameters matter). There is
            // no loop risk: a browser-process URLRequest does not go through this
            // RequestHandler.
            let fetch_url = CefString::from(&req_in.url()).to_string();

            if let Ok(mut st) = self.state.lock() {
                st.open_cb = callback.map(|c| c.clone());
            }
            // urlrequest_create is only valid on the UI thread; post it there.
            let mut task =
                FetchTaskBuilder::build(self.state.clone(), self.ctx.clone(), fetch_url);
            post_task(ThreadId::UI, Some(&mut task));

            // Asynchronous: handle_request=false, return true; the URLRequest
            // will call callback.cont()/cancel() when it finishes.
            if let Some(hr) = handle_request {
                *hr = 0;
            }
            1
        }

        fn response_headers(
            &self,
            response: Option<&mut Response>,
            response_length: Option<&mut i64>,
            _redirect_url: Option<&mut CefString>,
        ) {
            let st = self.state.lock().unwrap();
            if let Some(resp) = response {
                resp.set_status(if st.status > 0 { st.status } else { 200 });
                let mime = if st.mime.is_empty() {
                    "application/octet-stream"
                } else {
                    st.mime.as_str()
                };
                resp.set_mime_type(Some(&CefString::from(mime)));
            }
            if let Some(rl) = response_length {
                *rl = st.data.len() as i64;
            }
        }

        fn read(
            &self,
            data_out: *mut u8,
            bytes_to_read: ::std::os::raw::c_int,
            bytes_read: Option<&mut ::std::os::raw::c_int>,
            _callback: Option<&mut ResourceReadCallback>,
        ) -> ::std::os::raw::c_int {
            let mut st = self.state.lock().unwrap();
            let remaining = st.data.len().saturating_sub(st.read_pos);
            if remaining == 0 || data_out.is_null() {
                if let Some(br) = bytes_read {
                    *br = 0;
                }
                return 0; // EOF
            }
            let n = remaining.min(bytes_to_read.max(0) as usize);
            let pos = st.read_pos;
            unsafe {
                std::ptr::copy_nonoverlapping(st.data[pos..].as_ptr(), data_out, n);
            }
            st.read_pos += n;
            if let Some(br) = bytes_read {
                *br = n as ::std::os::raw::c_int;
            }
            1
        }

        fn cancel(&self) {
            if let Some(req) = self.state.lock().ok().and_then(|mut st| st.request.take()) {
                req.cancel();
            }
        }
    }
}

impl FetchHandlerBuilder {
    fn build(ctx: Option<RequestContext>) -> ResourceHandler {
        Self::new(Arc::new(Mutex::new(FetchState::default())), ctx)
    }
}

// ResourceRequestHandler

wrap_resource_request_handler! {
    struct ImgResourceRequestHandlerBuilder {
        ctx: Option<RequestContext>,
    }

    impl ResourceRequestHandler {
        // This trait's default is RV_CANCEL, which would abort the request before
        // it reaches resource_handler. We must let it continue.
        fn on_before_resource_load(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            ReturnValue::CONTINUE
        }

        fn resource_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
        ) -> Option<ResourceHandler> {
            let url = request
                .map(|r| CefString::from(&r.url()).to_string())
                .unwrap_or_default();
            if should_intercept(&url) {
                Some(FetchHandlerBuilder::build(self.ctx.clone()))
            } else {
                None
            }
        }
    }
}

impl ImgResourceRequestHandlerBuilder {
    fn build(ctx: Option<RequestContext>) -> ResourceRequestHandler {
        Self::new(ctx)
    }
}

// RequestHandler

wrap_request_handler! {
    pub struct ImgRequestHandlerBuilder {
        ctx: Option<RequestContext>,
    }

    impl RequestHandler {
        #[allow(clippy::too_many_arguments)]
        fn resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _is_navigation: ::std::os::raw::c_int,
            _is_download: ::std::os::raw::c_int,
            _request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut ::std::os::raw::c_int>,
        ) -> Option<ResourceRequestHandler> {
            let url = request
                .map(|r| CefString::from(&r.url()).to_string())
                .unwrap_or_default();
            if should_intercept(&url) {
                Some(ImgResourceRequestHandlerBuilder::build(self.ctx.clone()))
            } else {
                None
            }
        }
    }
}

impl ImgRequestHandlerBuilder {
    pub fn build(ctx: Option<RequestContext>) -> RequestHandler {
        Self::new(ctx)
    }
}
