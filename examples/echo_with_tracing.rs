use std::{net::SocketAddr, path::Path, sync::Arc};

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_quic::{Client, QuicTransport, Router, Server};
use jetstream_rpc::Protocol;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};

/// Example service demonstrating tracing support.
///
/// This example shows three ways to use tracing:
/// 1. Method-level #[instrument] attributes with custom configuration
/// 2. Auto-instrumentation via #[service(tracing)]
/// 3. No tracing for specific methods
#[service(tracing)] // Enable auto-instrumentation for all methods
pub trait Echo {
    /// This method has custom tracing configuration
    #[instrument(
        name = "echo_ping",
        skip(self),
        fields(
            message_len = message.len(),
        ),
        level = "debug"
    )]
    async fn ping(
        &mut self,
        ctx: Context,
        message: String,
    ) -> jetstream_error::Result<String>;

    /// This method uses default auto-instrumentation from #[service(tracing)]
    async fn echo(&mut self, text: String) -> jetstream_error::Result<String>;
}

#[derive(Clone)]
struct EchoImpl {}

impl Echo for EchoImpl {
    async fn ping(
        &mut self,
        ctx: Context,
        message: String,
    ) -> jetstream_error::Result<String> {
        tracing::info!("Ping received: {} {:?} ", message, ctx);

        Ok(format!("Pong: {}", message))
    }

    async fn echo(&mut self, text: String) -> jetstream_error::Result<String> {
        tracing::info!("Echo received: {}", text);
        Ok(text)
    }
}

pub static CA_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca-cert.pem");
pub static CLIENT_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client-cert.pem");
pub static CLIENT_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client-key.pem");
pub static SERVER_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server-cert.pem");
pub static SERVER_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server-key.pem");

fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    let data = std::fs::read(Path::new(path)).expect("Failed to read cert");
    rustls_pemfile::certs(&mut &*data)
        .filter_map(|r| r.ok())
        .collect()
}

fn load_key(path: &str) -> PrivateKeyDer<'static> {
    let data = std::fs::read(Path::new(path)).expect("Failed to read key");
    rustls_pemfile::private_key(&mut &*data)
        .expect("Failed to parse key")
        .expect("No key found")
}

async fn server(
    addr: SocketAddr,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server_cert = load_certs(SERVER_CERT_PEM).pop().unwrap();
    let server_key = load_key(SERVER_KEY_PEM);
    let ca_cert = load_certs(CA_CERT_PEM).pop().unwrap();

    // Register the EchoService as a protocol handler
    let echo_service = echo_protocol::EchoService { inner: EchoImpl {} };

    let mut router = Router::new();
    router.register(Arc::new(echo_service));

    let server =
        Server::new_with_mtls(server_cert, server_key, ca_cert, addr, router);

    tracing::info!("Server listening on {}", addr);
    server.run().await;

    Ok(())
}

async fn client(
    addr: SocketAddr,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let ca_cert = load_certs(CA_CERT_PEM).pop().unwrap();
    let client_cert = load_certs(CLIENT_CERT_PEM).pop().unwrap();
    let client_key = load_key(CLIENT_KEY_PEM);

    // Use the protocol version as ALPN
    let alpn = vec![EchoChannel::VERSION.as_bytes().to_vec()];
    let client = Client::new_with_mtls(ca_cert, client_cert, client_key, alpn)?;

    let connection = client.connect(addr, "localhost").await?;

    // Open a bidirectional stream and wrap it in QuicTransport
    let (send, recv) = connection.open_bi().await?;
    let transport: QuicTransport<EchoChannel> = (send, recv).into();
    let mut chan = EchoChannel::new(10, Box::new(transport));

    tracing::info!("Sending ping...");
    let response = chan
        .ping(Context::default(), "Hello, World!".to_string())
        .await?;
    tracing::info!("Received: {}", response);

    tracing::info!("Sending echo...");
    let response = chan.echo("Echo test".to_string()).await?;
    tracing::info!("Received: {}", response);

    Ok(())
}

#[tokio::main]
async fn main() {
    // Install the ring crypto provider for rustls
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_thread_ids(true)
        .with_span_events(
            tracing_subscriber::fmt::format::FmtSpan::ENTER
                | tracing_subscriber::fmt::format::FmtSpan::EXIT,
        )
        .init();

    tracing::info!("Starting echo service with tracing example");

    let addr: SocketAddr = "127.0.0.1:4434".parse().unwrap();
    tokio::select! {
      _ = server(addr) => {
          tracing::info!("Server exited");
      },
      _ = client(addr) => {
          tracing::info!("Client exited");
      },
    }
}
