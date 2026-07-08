//! Contorno de um bug do CEF 149: seguir um redirect cross-origin de um
//! subrecurso `no-cors` (imagem) falha com `ERR_INVALID_ARGUMENT`. O Google
//! Chat serve anexos e emojis customizados assim:
//!
//!   <img src="chat.google.com/api/get_attachment_url?..">  (same-origin)
//!        -> 302 -> lh7-eu.googleusercontent.com/chat_attachment/..  (cross-origin)
//!
//! A imagem final carrega bem quando pedida direto; só o *seguir-redirect* no
//! renderer quebra. Então interceptamos essas URLs e buscamos a imagem via
//! `CefURLRequest` (network service, que segue o 302 sem o bug), devolvendo os
//! bytes como uma resposta **same-origin 200** — o renderer nunca vê o redirect.
//!
//! Threads: o `ResourceHandler` roda na IO thread, mas `urlrequest_create` (com
//! um `RequestContext`, para herdar cookies) só pode rodar na UI thread. Então
//! `open()` posta uma task pra UI thread que cria o `URLRequest`; o estado é
//! compartilhado via `Arc<Mutex<..>>` (os wrappers do cef são `Send`).

use std::sync::{Arc, Mutex};

use cef::rc::Rc as _;
use cef::*;

/// URLs cujo carregamento por `<img>` dispara o bug do redirect no Chat.
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

// ---------------------------------------------------------------------------
// UrlrequestClient — acumula os bytes e, ao completar, libera o open().
// (roda na UI thread, onde o URLRequest foi criado)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Task — cria o URLRequest na UI thread.
// ---------------------------------------------------------------------------

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
            // UR_FLAG_ALLOW_STORED_CREDENTIALS (8): sem isto o CefURLRequest não
            // envia cookies e o endpoint responde com a página de login.
            req.set_flags(8);
            // O endpoint faz content-negotiation: sem Accept de imagem ele
            // devolve HTML em vez de redirecionar pro arquivo.
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
            // Sem estes o endpoint devolve uma página HTML em vez do 302->imagem.
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

// ---------------------------------------------------------------------------
// ResourceHandler — busca a imagem via URLRequest e serve os bytes.
// (open/read/response_headers rodam na IO thread)
// ---------------------------------------------------------------------------

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
            // Busca exatamente a mesma URL (o token/parâmetros importam). Não há
            // risco de loop: URLRequest do processo do browser não passa por este
            // RequestHandler.
            let fetch_url = CefString::from(&req_in.url()).to_string();

            if let Ok(mut st) = self.state.lock() {
                st.open_cb = callback.map(|c| c.clone());
            }
            // urlrequest_create só é válido na UI thread; posta pra lá.
            let mut task =
                FetchTaskBuilder::build(self.state.clone(), self.ctx.clone(), fetch_url);
            post_task(ThreadId::UI, Some(&mut task));

            // Assíncrono: handle_request=false, retorna true; o URLRequest
            // chamará callback.cont()/cancel() quando terminar.
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

// ---------------------------------------------------------------------------
// ResourceRequestHandler
// ---------------------------------------------------------------------------

wrap_resource_request_handler! {
    struct ImgResourceRequestHandlerBuilder {
        ctx: Option<RequestContext>,
    }

    impl ResourceRequestHandler {
        // O default desta trait é RV_CANCEL, que abortaria a requisição antes
        // de chegar no resource_handler. Precisamos deixar continuar.
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

// ---------------------------------------------------------------------------
// RequestHandler
// ---------------------------------------------------------------------------

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
