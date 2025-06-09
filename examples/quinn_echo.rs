use std::{net::SocketAddr, sync::Arc};

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_rpc::Framed;
use okstd::prelude::*;
use quinn::{Endpoint, ServerConfig, ClientConfig, SendStream, RecvStream};
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use rustls::pki_types::{self, CertificateDer, PrivateKeyDer};
use rustls::pki_types::pem::PemObject;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

struct QuinnStream {
    send: SendStream,
    recv: RecvStream,
}

impl AsyncRead for QuinnStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.recv).poll_read(cx, buf)
    }
}

impl AsyncWrite for QuinnStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.send).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => Poll::Ready(Ok(n)),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match Pin::new(&mut self.get_mut().send).poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match Pin::new(&mut self.get_mut().send).poll_shutdown(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}

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
    let cert = CertificateDer::from_pem_file(SERVER_CERT_PEM)?;
    let key = PrivateKeyDer::from_pem_file(SERVER_KEY_PEM)?;
    let mut tls = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;
    tls.alpn_protocols = vec![b"hq-29".to_vec()];
    let server_config = ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(tls)?));
    let endpoint = Endpoint::server(server_config, "127.0.0.1:4433".parse()?)?;

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            if let Ok(connection) = connecting.await {
                while let Ok((send, recv)) = connection.accept_bi().await {
                    tokio::spawn(async move {
                        let stream = QuinnStream { send, recv };
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
    roots.add(CertificateDer::from_pem_file(CA_CERT_PEM)?)?;

    let cert = CertificateDer::from_pem_file(CLIENT_CERT_PEM)?;
    let key = PrivateKeyDer::from_pem_file(CLIENT_KEY_PEM)?;
    let mut tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(vec![cert], key)?;
    tls.alpn_protocols = vec![b"hq-29".to_vec()];
    let client_config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls)?));

    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config);
    let addr: SocketAddr = "127.0.0.1:4433".parse()?;
    let connection = endpoint.connect(addr, "localhost")?.await?;

    let (send, recv) = connection.open_bi().await?;
    let stream = QuinnStream { send, recv };
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
    rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();
    tokio::select! {
      _ = server() => {},
      _ = client() => {},
    }
}
