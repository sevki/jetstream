#![cfg(feature = "quic")]
use std::{net::SocketAddr, path::Path, sync::Arc};

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_quic::{
    Client, QuicRouter, QuicRouterHandler, QuicTransport, Server,
};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<String>;
}

#[derive(Clone)]
struct EchoImpl {}

impl Echo for EchoImpl {
    async fn ping(&mut self) -> Result<String> {
        eprintln!("Ping received");
        eprintln!("Pong sent");
        Ok("pong".to_string())
    }
}

pub static CA_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca.pem");
pub static CLIENT_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client.pem");
pub static CLIENT_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client.key");
pub static SERVER_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.pem");
pub static SERVER_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.key");

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

    let mut root_store = rustls::RootCertStore::empty();
    root_store.add(ca_cert).expect("Failed to add CA cert");
    let client_verifier =
        rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
            .allow_unauthenticated()
            .build()
            .expect("Failed to build client verifier");

    let echo_service = echo_protocol::EchoService { inner: EchoImpl {} };

    let rpc_router = Arc::new(
        jetstream_rpc::Router::new()
            .with_handler(echo_protocol::PROTOCOL_NAME, echo_service),
    );
    let quic_handler = QuicRouterHandler::new(rpc_router);

    let mut quic_router = QuicRouter::new();
    quic_router.register(Arc::new(quic_handler));

    let server = Server::new_with_mtls(
        vec![server_cert],
        server_key,
        client_verifier,
        addr,
        quic_router,
    );

    eprintln!("Server listening on {}", addr);
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

    let alpn = vec![b"jetstream".to_vec()];
    let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let client = Client::new_with_mtls(
        ca_cert,
        client_cert,
        client_key,
        alpn,
        bind_addr,
    )?;

    let connection = client.connect(addr, "localhost").await?;

    let (send, recv) = connection.open_bi().await?;
    let transport: QuicTransport<EchoChannel> = (send, recv).into();
    let mut chan = EchoChannel::new(1, Box::new(transport));
    chan.negotiate_version(u32::MAX).await?;

    for _ in 0..100 {
        if let Err(e) = chan.ping().await {
            eprintln!("Ping error: {:?}", e);
            break;
        }
    }
    // Properly close the stream
    drop(chan);
    Ok(())
}

#[tokio::test]
async fn echo() {
    // Install the ring crypto provider for rustls
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    let addr: SocketAddr = "127.0.0.1:4435".parse().unwrap();
    tokio::select! {
      _ = server(addr) => {},
      _ = client(addr) => {},
    }
}
