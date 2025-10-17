#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
use bytes::Bytes;
use h3::server::RequestResolver;
use h3_quinn::quinn::{self};
use quinn::crypto::rustls::{HandshakeData, QuicServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::sync::Arc;
use tracing::{error, info, trace_span};

pub struct Server {
    endpoint: quinn::Endpoint,
}

static H3: &[u8] = b"h3";

impl Server {
    pub fn new_with_addr(
        cert: CertificateDer<'static>,
        key: PrivateKeyDer<'static>,
        addr: &str,
    ) -> Self {
        let mut tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .unwrap();
        tls_config.max_early_data_size = u32::MAX;
        tls_config.alpn_protocols = vec![H3.into()];

        let _server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(tls_config).unwrap(),
        ));
        let endpoint =
            quinn::Endpoint::server(_server_config, addr.parse().unwrap())
                .expect("Failed to create endpoint");

        Self { endpoint }
    }
    pub async fn run(self) {
        // handle incoming connections and requests

        while let Some(new_conn) = self.endpoint.accept().await {
            trace_span!("New connection being attempted");

            tokio::spawn(async move {
                match new_conn.await {
                    Ok(conn) => {
                        // For the rustls TlsSession, the Any type is Vec<rustls::pki_types::CertificateDer>
                        // The dynamic type returned is determined by the configured Session. For the default rustls session, the return value can be downcast to a Vec<rustls::pki_types::CertificateDer>
                        let peer_certs = if let Some(identity) =
                            conn.peer_identity()
                        {
                            match identity.downcast::<Vec<rustls::pki_types::CertificateDer>>() {
                                Ok(certs) => Some(*certs),
                                Err(_) => None,
                            }
                        } else {
                            None
                        };

                        // Get ALPN protocol from handshake data
                        let alpn_protocol = if let Some(handshake_data) =
                            conn.handshake_data()
                        {
                            if let Ok(data) =
                                handshake_data.downcast::<HandshakeData>()
                            {
                                data.protocol.map(|p| {
                                    String::from_utf8_lossy(&p).to_string()
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        info!("new connection established, peer certs: {:?}, ALPN: {:?}",
                              peer_certs.is_some(), alpn_protocol);
                        let mut h3_conn = h3::server::Connection::new(
                            h3_quinn::Connection::new(conn),
                        )
                        .await
                        .unwrap();

                        loop {
                            match h3_conn.accept().await {
                                Ok(Some(resolver)) => {
                                    tokio::spawn(async {
                                        if let Err(e) =
                                            handle_request(resolver).await
                                        {
                                            error!(
                                                "handling request failed: {}",
                                                e
                                            );
                                        }
                                    });
                                }
                                // indicating that the remote sent a go-away frame
                                // all requests have been processed
                                Ok(None) => {
                                    break;
                                }
                                Err(err) => {
                                    error!("error on accept {}", err);
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        error!("accepting connection failed: {:?}", err);
                    }
                }
            });
        }

        // shut down gracefully
        // wait for connections to be closed before exiting
        self.endpoint.wait_idle().await;
    }
}

async fn handle_request<C>(
    resolver: RequestResolver<C, Bytes>,
) -> Result<(), Box<dyn std::error::Error>>
where
    C: h3::quic::Connection<Bytes>,
{
    let (_req, mut stream) = resolver.resolve_request().await?;

    let resp = http::Response::builder().status(200).body(()).unwrap();

    match stream.send_response(resp).await {
        Ok(_) => {
            stream
                .send_data(
                    concat!("jetstream ", env!("CARGO_PKG_VERSION")).into(),
                )
                .await?;
            info!("successfully respond to connection");
        }
        Err(err) => {
            error!("unable to send response to connection peer: {:?}", err);
        }
    }

    Ok(stream.finish().await?)
}
