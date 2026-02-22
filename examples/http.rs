use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as HttpBuilder;
use hyper_util::service::TowerToHyperService;
use jetstream::prelude::*;
use jetstream_http::{AltSvcLayer, H3Service};
use jetstream_macros::service;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tower_http::services::ServeDir;
use tracing::{error, info};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};

// r[impl jetstream.webtransport.http-example]
#[service]
pub trait EchoHttp {
    async fn ping(&mut self, message: String) -> Result<String>;
    async fn add(&mut self, a: i32, b: i32) -> Result<i32>;
}

#[derive(Clone)]
struct EchoHttpImpl;

impl EchoHttp for EchoHttpImpl {
    async fn ping(&mut self, message: String) -> Result<String> {
        Ok(message)
    }
    async fn add(&mut self, a: i32, b: i32) -> Result<i32> {
        Ok(a + b)
    }
}

pub static CA_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca.pem");
pub static SERVER_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.pem");
pub static SERVER_KEY: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.key");

fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    let data = fs::read(Path::new(path)).expect("Failed to read cert");
    rustls_pemfile::certs(&mut &*data)
        .filter_map(|r| r.ok())
        .collect()
}

fn load_key(path: &str) -> PrivateKeyDer<'static> {
    let data = fs::read(Path::new(path)).expect("Failed to read key");
    rustls_pemfile::private_key(&mut &*data)
        .expect("Failed to parse key")
        .expect("No key found")
}

pub static APP_DIST: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/app/dist");

/// Run HTTP/2 server with TLS
async fn run_http2_server(
    addr: SocketAddr,
    router: Router,
    tls_acceptor: TlsAcceptor,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _remote_addr) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        let router = router.clone();

        tokio::spawn(async move {
            let tls_stream = match tls_acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    error!("TLS accept error: {}", e);
                    return;
                }
            };

            let io = TokioIo::new(tls_stream);
            let svc = TowerToHyperService::new(router);

            if let Err(e) = HttpBuilder::new(TokioExecutor::new())
                .serve_connection(io, svc)
                .await
            {
                error!("HTTP/2 connection error: {}", e);
            }
        });
    }
}

/// Run HTTP/3 server with QUIC + WebTransport
async fn run_http3_server(
    addr: SocketAddr,
    router: Router,
    server_cert: CertificateDer<'static>,
    server_key: PrivateKeyDer<'static>,
    ca_cert: Option<CertificateDer<'static>>,
) {
    // Register the EchoHttp service as a WebTransport handler
    let echo = echohttp_protocol::EchoHttpService {
        inner: EchoHttpImpl,
    };
    let rpc_router = Arc::new(
        jetstream_rpc::Router::new()
            .with_handler(echohttp_protocol::PROTOCOL_NAME, echo),
    );

    let mut quic_router = jetstream_quic::QuicRouter::new();

    let server = if let Some(ca) = ca_cert {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(ca).expect("Failed to add CA cert");
        let client_verifier =
            rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
                .allow_unauthenticated()
                .build()
                .expect("Failed to build client verifier");

        let h3_service = Arc::new(H3Service::new_with_cert_verifier(
            router,
            rpc_router,
            client_verifier.clone(),
        ));
        quic_router.register(h3_service);

        jetstream_quic::Server::new_with_mtls(
            vec![server_cert],
            server_key,
            client_verifier,
            addr,
            quic_router,
        )
    } else {
        let h3_service = Arc::new(H3Service::new(router, rpc_router));
        quic_router.register(h3_service);

        jetstream_quic::Server::new_with_addr(
            vec![server_cert],
            server_key,
            addr,
            quic_router,
        )
    };

    server.run().await;
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Install the ring crypto provider for rustls
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    // Check for --mtls flag
    let mtls_enabled = std::env::args().any(|arg| arg == "--mtls");

    let server_certs = load_certs(SERVER_PEM);
    let server_key = load_key(SERVER_KEY);
    let ca_cert = if mtls_enabled {
        Some(load_certs(CA_PEM).pop().unwrap())
    } else {
        None
    };

    // Create shared Axum router serving React app build + Alt-Svc header
    let router = Router::new()
        .fallback_service(ServeDir::new(APP_DIST))
        .layer(AltSvcLayer::new(4433));

    // Setup TLS config for HTTP/2
    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(server_certs.clone(), server_key.clone_key())?;
    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let addr: SocketAddr = "127.0.0.1:4433".parse()?;

    info!("=== JetStream HTTP + WebTransport Server ===");
    info!("Listening on https://{}", addr);
    info!("  - HTTP/2 over TLS (TCP)");
    info!("  - HTTP/3 + WebTransport over QUIC (UDP)");
    info!(
        "  - WebTransport protocol: {}",
        echohttp_protocol::PROTOCOL_VERSION
    );
    if mtls_enabled {
        info!("mTLS enabled - client certificates required");
    } else {
        info!("No client auth (use --mtls to enable)");
    }

    // Run both servers concurrently on the same port (TCP for HTTP/2, UDP for HTTP/3)
    tokio::select! {
        result = run_http2_server(addr, router.clone(), tls_acceptor) => {
            if let Err(e) = result {
                error!("HTTP/2 server error: {}", e);
            }
        }
        _ = run_http3_server(
            addr,
            router,
            server_certs.into_iter().next().unwrap(),
            server_key,
            ca_cert,
        ) => {}
    }

    Ok(())
}
