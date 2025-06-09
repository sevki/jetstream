use std::{net::SocketAddr, sync::Arc};

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_rpc::Framed;
use okstd::prelude::*;
use quinn::{Endpoint, ServerConfig, ClientConfig};
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<(), Error>;
}

struct EchoImpl;

impl Echo for EchoImpl {
    async fn ping(&mut self) -> Result<(), Error> {
        eprintln!("Ping received");
        eprintln!("Pong sent");
        Ok(())
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

async fn server() -> Result<(), Box<dyn std::error::Error>> {
    let cert = CertificateDer::from(std::fs::read(SERVER_CERT_PEM)?);
    let key = PrivateKeyDer::try_from(std::fs::read(SERVER_KEY_PEM)?)?;
    let mut tls = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;
    tls.alpn_protocols = vec![b"hq-29".to_vec()];
    let server_config = ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(tls)?));
    let endpoint = Endpoint::server(server_config, "127.0.0.1:4433".parse()?)?;

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            if let Ok(connection) = connecting.await {
                while let Ok(stream) = connection.accept_bi().await {
                    tokio::spawn(async move {
                        let echo = EchoImpl;
                        let servercodec: jetstream::prelude::server::service::ServerCodec<
                            echo_protocol::EchoService<EchoImpl>,
                        > = Default::default();
                        let framed = Framed::new(stream, servercodec);
                        let mut serv = echo_protocol::EchoService { inner: echo };
                        server::service::run(&mut serv, framed).await.unwrap();
                    });
                }
            }
        });
    }
    Ok(())
}

async fn client() -> Result<(), Box<dyn std::error::Error>> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(CertificateDer::from(std::fs::read(CA_CERT_PEM)?))?;

    let cert = CertificateDer::from(std::fs::read(CLIENT_CERT_PEM)?);
    let key = PrivateKeyDer::try_from(std::fs::read(CLIENT_KEY_PEM)?)?;
    let mut tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_single_cert(vec![cert], key)?;
    tls.alpn_protocols = vec![b"hq-29".to_vec()];
    let client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls)?));

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);
    let addr: SocketAddr = "127.0.0.1:4433".parse()?;
    let connection = endpoint.connect(addr, "localhost")?.await?;
    connection.keep_alive(true)?;

    let stream = connection.open_bi().await?;
    let client_codec: jetstream_client::ClientCodec<EchoChannel> = Default::default();
    let mut framed = Framed::new(stream, client_codec);
    let mut chan = EchoChannel {
        inner: Box::new(&mut framed),
    };
    chan.ping().await?;
    Ok(())
}

#[okstd::main]
async fn main() {
    tokio::select! {
      _ = server() => {},
      _ = client() => {},
    }
}
