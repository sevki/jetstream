use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use askama::Template;
use axum::http::header;
use axum::{routing::get, Router};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as HttpBuilder;
use hyper_util::service::TowerToHyperService;
use jetstream_http::{AltSvcLayer, H3Service, JetStreamContext};
use jetstream_rpc::context::{Peer, RemoteAddr};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::{error, info};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};

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

#[derive(Template)]
#[template(path = "template.html")]
struct JetStreamTemplate<'a> {
    body: &'a str,
    version: &'static str,
    year: u16,
}

impl Default for JetStreamTemplate<'_> {
    fn default() -> Self {
        Self {
            body: env!("CARGO_PKG_DESCRIPTION"),
            version: env!("CARGO_PKG_VERSION"),
            year: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| 1970 + (d.as_secs() / 31_536_000) as u16)
                .unwrap_or(2026),
        }
    }
}

#[derive(Template)]
#[template(path = "context.html")]
struct ContextTemplate<'a> {
    remote_addr: &'a str,
    has_peer_cert: bool,
    common_name: Option<&'a str>,
    fingerprint: &'a str,
    dns_names: Vec<&'a str>,
    ip_addresses: Vec<String>,
    emails: Vec<&'a str>,
    uris: Vec<String>,
    method: &'a str,
    path: &'a str,
    protocol: &'a str,
}

async fn handle_request(
    ctx: JetStreamContext,
    req: axum::extract::Request,
) -> impl axum::response::IntoResponse {
    // Log the peer context
    info!("Request from peer: {}", *ctx);

    // Get remote address
    let remote_addr = match ctx.remote() {
        Some(RemoteAddr::IpAddr(ip)) => ip.to_string(),
        _ => "unknown".to_string(),
    };

    // Determine protocol version
    let protocol = format!("{:?}", req.version());
    // Extract peer cert info
    let (
        has_peer_cert,
        common_name,
        fingerprint,
        dns_names,
        ip_addresses,
        emails,
        uris,
    ) = match ctx.peer() {
        Some(Peer::Tls(tls_peer)) => {
            if let Some(leaf) = tls_peer.leaf() {
                (
                    true,
                    leaf.common_name.clone(),
                    leaf.fingerprint.clone(),
                    leaf.dns_names.clone(),
                    leaf.ip_addresses.iter().map(|ip| ip.to_string()).collect(),
                    leaf.emails.clone(),
                    leaf.uris.iter().map(|u| u.to_string()).collect(),
                )
            } else {
                (false, None, String::new(), vec![], vec![], vec![], vec![])
            }
        }
        _ => (false, None, String::new(), vec![], vec![], vec![], vec![]),
    };

    let method = req.method().as_str();
    let path = req.uri().path();

    let context_html = ContextTemplate {
        remote_addr: &remote_addr,
        has_peer_cert,
        common_name: common_name.as_deref(),
        fingerprint: &fingerprint,
        dns_names: dns_names.iter().map(|s| s.as_str()).collect(),
        ip_addresses,
        emails: emails.iter().map(|s| s.as_str()).collect(),
        uris,
        method,
        path,
        protocol: &protocol,
    }
    .render()
    .unwrap_or_else(|_| "Error rendering context".to_string());

    let template = JetStreamTemplate {
        body: &context_html,
        ..Default::default()
    };
    let body = template.render().unwrap_or_else(|_| "Error".to_string());

    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], body)
}

/// Run HTTP/2 server with TLS
async fn run_http2_server(
    addr: SocketAddr,
    router: Router,
    tls_acceptor: TlsAcceptor,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

/// Run HTTP/3 server with QUIC
async fn run_http3_server(
    addr: SocketAddr,
    router: Router,
    server_cert: CertificateDer<'static>,
    server_key: PrivateKeyDer<'static>,
    ca_cert: Option<CertificateDer<'static>>,
) {
    let h3_service = Arc::new(H3Service::new(router));

    let mut quic_router = jetstream_quic::Router::new();
    quic_router.register(h3_service);

    let server = if let Some(ca) = ca_cert {
        jetstream_quic::Server::new_with_mtls(
            server_cert,
            server_key,
            ca,
            addr,
            quic_router,
        )
    } else {
        jetstream_quic::Server::new_with_addr(
            server_cert,
            server_key,
            addr,
            quic_router,
        )
    };

    server.run().await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create shared Axum router with Alt-Svc layer to advertise HTTP/3
    let router = Router::new()
        .fallback(get(handle_request))
        .layer(AltSvcLayer::new(4433));

    // Setup TLS config for HTTP/2
    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(server_certs.clone(), server_key.clone_key())?;
    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let addr: SocketAddr = "127.0.0.1:4433".parse()?;

    info!("=== JetStream HTTP Server ===");
    info!("Listening on https://{}", addr);
    info!("  - HTTP/2 over TLS (TCP)");
    info!("  - HTTP/3 over QUIC (UDP)");
    if mtls_enabled {
        info!("mTLS enabled - client certificates required");
    } else {
        info!("No client auth (use --mtls to enable)");
    }
    info!("You can now run ./examples/launch_chrome.sh to connect with Chrome");

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
