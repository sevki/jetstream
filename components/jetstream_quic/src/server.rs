use std::net::SocketAddr;
use std::sync::Arc;

use h3_quinn::quinn::{self};

use quinn::crypto::rustls::QuicServerConfig;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use rustls::server::danger::ClientCertVerifier;

use tracing::trace_span;

use crate::Router;

pub struct Server {
    pub(crate) endpoint: quinn::Endpoint,
    pub(crate) router: Router,
}

impl Server {
    /// Create a new server without client authentication
    ///
    /// # Arguments
    /// * `cert` - Server certificate
    /// * `key` - Server private key
    /// * `addr` - Socket address to bind to
    /// * `router` - Router for handling connections
    pub fn new_with_addr(
        cert: CertificateDer<'static>,
        key: PrivateKeyDer<'static>,
        addr: SocketAddr,
        router: Router,
    ) -> Self {
        let mut tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .unwrap();
        tls_config.max_early_data_size = u32::MAX;
        tls_config.alpn_protocols = router.alpns();

        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(tls_config).unwrap(),
        ));
        let endpoint = quinn::Endpoint::server(server_config, addr)
            .expect("Failed to create endpoint");

        Self { endpoint, router }
    }

    /// Create a new server with mTLS (client certificate authentication)
    ///
    /// # Arguments
    /// * `cert` - Server certificate
    /// * `key` - Server private key
    /// * `client_verifier` - Client certificate verifier (e.g. `WebPkiClientVerifier` or a custom implementation)
    /// * `addr` - Socket address to bind to
    /// * `router` - Router for handling connections
    pub fn new_with_mtls(
        cert: CertificateDer<'static>,
        key: PrivateKeyDer<'static>,
        client_verifier: Arc<dyn ClientCertVerifier>,
        addr: SocketAddr,
        router: Router,
    ) -> Self {
        let mut tls_config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(vec![cert], key)
            .unwrap();
        tls_config.max_early_data_size = u32::MAX;
        tls_config.alpn_protocols = router.alpns();

        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(tls_config).unwrap(),
        ));
        let endpoint = quinn::Endpoint::server(server_config, addr)
            .expect("Failed to create endpoint");

        Self { endpoint, router }
    }
    pub async fn run(&self) {
        // handle incoming connections and requests

        while let Some(new_conn) = self.endpoint.accept().await {
            trace_span!("New connection being attempted");
            let router = self.router.clone();
            tokio::spawn(async move { router.handle_incoming(new_conn).await });
        }

        // shut down gracefully
        // wait for connections to be closed before exiting
        self.endpoint.wait_idle().await;
    }
}
