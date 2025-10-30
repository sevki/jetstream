mod error;
mod websocket_transport;

pub use error::JetstreamProtocolError;

use askama::Template;
use async_trait::async_trait;
use futures::lock::Mutex;
use jetstream_rpc::{context, Protocol};
use jetstream_wireformat::wire_format_extensions::{
    bytes::Bytes, ConvertWireFormat,
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use worker::{wasm_bindgen_futures, Env, WebSocketPair};

use crate::websocket_transport::WebSocketTransport;

pub extern crate async_trait;

pub const JETSTREAM_PROTO_HEADER_KEY: &str = "X-JetStream-Proto";

#[async_trait]
pub trait DynamicProtocol: Send + Sync {
    fn protocol_version(&self) -> &'static str;
    async fn rpc(
        &mut self,
        context: context::Context,
        data: &[u8],
    ) -> Result<Vec<u8>, jetstream_rpc::Error>;
}

#[async_trait]
impl<P: Protocol> DynamicProtocol for P {
    fn protocol_version(&self) -> &'static str {
        P::VERSION
    }

    async fn rpc(
        &mut self,
        context: context::Context,
        data: &[u8],
    ) -> Result<Vec<u8>, jetstream_rpc::Error> {
        let frame = jetstream_rpc::Frame::<P::Request>::from_bytes(
            &Bytes::copy_from_slice(data),
        )?;
        Ok(self
            .rpc(context, frame)
            .await
            .map_err(|e| jetstream_rpc::Error::Generic(e.into()))?
            .as_bytes())
    }
}

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

/// A protocol aware router
pub struct Router<H: HtmlAssets> {
    html_fallbacks: PhantomData<H>,
    handlers: Arc<Mutex<HashMap<String, Box<dyn DynamicProtocol>>>>,
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
        P: DynamicProtocol + 'static,
    {
        let mut map = HashMap::new();
        for handler in handlers.into_iter() {
            let version = handler.protocol_version().to_string();
            map.insert(version, Box::new(handler) as Box<dyn DynamicProtocol>);
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
        let proto = req.headers().get(JETSTREAM_PROTO_HEADER_KEY)?;
        // Check for WebSocket upgrade request
        let upgrade_header = req.headers().get("Upgrade")?;

        worker::console_log!(
            "Request received - Proto: {:?}, Upgrade: {:?}",
            proto,
            upgrade_header
        );

        match (proto, upgrade_header) {
            (Some(proto), Some(upgrade_header))
                if upgrade_header == "websocket" =>
            {
                worker::console_log!(
                    "WebSocket upgrade request for protocol: {}",
                    proto
                );
                let ip = req.headers().get("CF-Connecting-IP")?;
                let Some(ip) = ip else {
                    worker::console_log!("Missing IP address");
                    return error::JetstreamProtocolError::MissingIPAddress
                        .into();
                };
                worker::console_log!("Client IP: {}", ip);
                let ws = WebSocketPair::new()?;
                let (client, server) = (ws.client, ws.server);
                worker::console_log!("WebSocket pair created");
                let mut server_transport =
                    WebSocketTransport::new(server, ip).unwrap();
                let handlers = self.handlers.clone();

                worker::console_log!("Starting background handler");
                wasm_bindgen_futures::spawn_local(async move {
                    worker::console_log!("Background handler started");
                    let mut guard = handlers.lock().await;
                    let handler = guard.get_mut(&proto).unwrap();
                    server_transport.handle(handler).await
                });

                worker::console_log!("Returning WebSocket response");
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
