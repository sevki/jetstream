use std::{collections::HashMap, sync::Arc};

use jetstream_rpc::context::RemoteAddr;
use quinn::{crypto::rustls::HandshakeData, Incoming};
use tracing::{error, info};

use crate::{quic_handler::QuicHandler, session::peer_from_connection};

#[derive(Clone, Default)]
pub struct Router {
    handlers: HashMap<String, Arc<dyn QuicHandler>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn QuicHandler>) {
        for alpn in handler.alpns() {
            self.handlers.insert(alpn, handler.clone());
        }
    }

    pub fn alpns(&self) -> Vec<Vec<u8>> {
        self.handlers
            .keys()
            .map(|s| s.clone().into_bytes())
            .collect()
    }

    pub async fn handle_incoming(&self, incoming: Incoming) {
        match incoming.await {
            Ok(conn) => {
                let peer = peer_from_connection(&conn);

                let ctx = jetstream_rpc::context::Context::new(
                    Some(RemoteAddr::IpAddr(conn.remote_address().ip())),
                    peer,
                );

                // Get ALPN protocol from handshake data
                let alpn_protocol = if let Some(handshake_data) =
                    conn.handshake_data()
                {
                    if let Ok(data) = handshake_data.downcast::<HandshakeData>()
                    {
                        data.protocol
                            .map(|p| String::from_utf8_lossy(&p).to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

                info!(
                    "new connection established, peer: {}, ALPN: {:?}",
                    ctx, alpn_protocol
                );

                // Dispatch to the appropriate handler based on ALPN
                if let Some(alpn) = alpn_protocol {
                    if let Some(handler) = self.handlers.get(&alpn) {
                        handler.accept(ctx, conn).await;
                    } else {
                        error!("no handler registered for ALPN: {}", alpn);
                    }
                } else {
                    error!("no ALPN protocol negotiated");
                }
            }
            Err(err) => {
                error!("accepting connection failed: {:?}", err);
            }
        }
    }
}
