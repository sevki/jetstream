mod error;
mod websocket_transport;

pub use error::JetstreamProtocolError;

use askama::Template;
use futures::lock::Mutex;
use jetstream_rpc::{Protocol, HEADER_KEY_JETSTREAM_PROTO};
use jetstream_rpc::{AnyServer, IntoError};
use jetstream_wireformat::wire_format_extensions::ConvertWireFormat;
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use worker::{wasm_bindgen_futures, Env, WebSocket, WebSocketPair};

use crate::websocket_transport::WebSocketTransport;

pub extern crate async_trait;

pub trait HtmlAssets {
    fn fallback_html() -> worker::Result<worker::Response>;
}

pub struct DefaultHtmlFallback;

impl HtmlAssets for DefaultHtmlFallback {
    fn fallback_html() -> worker::Result<worker::Response> {
        JetStreamTemplate {
            body: include_str!("doc.html"),
            ..Default::default()
        }
        .try_into()
    }
}

#[repr(u16)]
enum Shutdown {
    MissingProtocol = 1,
    RpcFailure = 2,
}

impl Shutdown {
    fn shutdown(self, ws: &mut WebSocket) -> worker::Result<()> {
        match self {
            Shutdown::MissingProtocol => {
                ws.close(Some(self as u16), Some("Missing Protocol"))
            }
            Shutdown::RpcFailure => {
                ws.close(Some(self as u16), Some("RPC Failure"))
            }
        }
    }
}

/// A protocol aware router
pub struct Router<H: HtmlAssets> {
    html_fallbacks: PhantomData<H>,
    // Cloudflare Workers run on a single OS thread, but several request futures
    // can still be in-flight on the executor. Because we await on handler RPC
    // calls, we need an async-aware lock to queue those borrows instead of
    // panicking on a re-entrant `RefCell` borrow.
    handlers: Arc<Mutex<HashMap<String, Box<dyn AnyServer>>>>,
}

#[derive(Template)]
#[template(path = "template.html")]
struct JetStreamTemplate<'a> {
    body: &'a str,
    version: &'static str,
}

impl Default for JetStreamTemplate<'_> {
    fn default() -> Self {
        Self {
            body: env!("CARGO_PKG_DESCRIPTION"),
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

impl<H: HtmlAssets> Router<H> {
    pub fn new<I, P>(handlers: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AnyServer + 'static,
    {
        let mut map = HashMap::new();
        for handler in handlers.into_iter() {
            let version = handler.protocol_version().to_string();
            map.insert(version, Box::new(handler) as Box<dyn AnyServer>);
        }
        Self {
            html_fallbacks: PhantomData,
            handlers: Arc::new(Mutex::new(map)),
        }
    }

    /// Fetch is a one-shot RPC call to a protocol handler.
    pub async fn fetch(
        &mut self,
        req: worker::Request,
        _env: Env,
        _ctx: worker::Context,
    ) -> worker::Result<worker::Response> {
        let proto = req.headers().get(HEADER_KEY_JETSTREAM_PROTO)?;
        // Check for WebSocket upgrade request
        let upgrade_header = req.headers().get("Upgrade")?;

        match (proto, upgrade_header) {
            (Some(proto), Some(upgrade_header))
                if upgrade_header == "websocket" =>
            {
                let ip = req.headers().get("CF-Connecting-IP")?;
                let Some(ip) = ip else {
                    return error::JetstreamProtocolError::MissingIPAddress
                        .into();
                };
                let ws = WebSocketPair::new()?;
                let (client, server) = (ws.client, ws.server);
                let mut server_transport =
                    WebSocketTransport::new(server, ip).unwrap();
                let handlers = self.handlers.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let mut guard = handlers.lock().await;
                    let Some(handler) = guard.get_mut(&proto) else {
                        let _ = server_transport
                            .shutdown(Shutdown::MissingProtocol);
                        return;
                    };
                    server_transport.handle(handler).await
                });

                return worker::Response::from_websocket(client);
            }
            _ => {
                worker::console_log!("Not a WebSocket request, returning HTML");
            }
        }

        H::fallback_html()
    }
}

impl TryInto<worker::Response> for JetStreamTemplate<'_> {
    type Error = worker::Error;

    fn try_into(self) -> Result<worker::Response, Self::Error> {
        worker::Response::from_html(
            self.render()
                .map_err(|e| worker::Error::RustError(e.to_string()))?,
        )
    }
}
