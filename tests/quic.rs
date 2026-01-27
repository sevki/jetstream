#![cfg(feature = "quic")]
use std::{net::SocketAddr, path::Path, sync::Arc};

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_quic::{Client, QuicTransport, Router, Server};
use jetstream_rpc::Protocol;

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

    let echo_service = echo_protocol::EchoService { inner: EchoImpl {} };

    let mut router = Router::new();
    router.register(Arc::new(echo_service));

    let server =
        Server::new_with_mtls(server_cert, server_key, ca_cert, addr, router);

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

    let alpn = vec![EchoChannel::VERSION.as_bytes().to_vec()];
    let client = Client::new_with_mtls(ca_cert, client_cert, client_key, alpn)?;

    let connection = client.connect(addr, "localhost").await?;

    let (send, recv) = connection.open_bi().await?;
    let transport: QuicTransport<EchoChannel> = (send, recv).into();
    let mut chan = EchoChannel::new(1, Box::new(transport));

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
