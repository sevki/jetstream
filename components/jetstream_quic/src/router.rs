use std::{collections::HashMap, sync::Arc};

use jetstream_rpc::context::{Peer, RemoteAddr, TlsPeer};
use quinn::{crypto::rustls::HandshakeData, Incoming};
use tracing::{error, info, warn};

use crate::quic_handler::ProtocolHandler;

#[derive(Clone, Default)]
pub struct Router {
    handlers: HashMap<String, Arc<dyn ProtocolHandler>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register(&mut self, handler: Arc<dyn ProtocolHandler>) {
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
                // For the rustls TlsSession, the Any type is Vec<rustls::pki_types::CertificateDer>
                // The dynamic type returned is determined by the configured Session. For the default rustls session, the return value can be downcast to a Vec<rustls::pki_types::CertificateDer>
                // Extract peer certificates and parse them
                let peer = if let Some(identity) = conn.peer_identity() {
                    match identity
                        .downcast::<Vec<rustls::pki_types::CertificateDer>>()
                    {
                        Ok(certs) => {
                            // Parse the certificate chain
                            match TlsPeer::from_der_chain(
                                &certs
                                    .iter()
                                    .map(|c| c.as_ref())
                                    .collect::<Vec<_>>(),
                            ) {
                                Ok(tls_peer) => Some(Peer::Tls(tls_peer)),
                                Err(e) => {
                                    warn!(
                                        "failed to parse peer certificates: {}",
                                        e
                                    );
                                    None
                                }
                            }
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                };

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
