use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{Sink, SinkExt, Stream, StreamExt};
use jetstream_rpc::{client::ClientCodec, Error, Protocol};
use quinn::{ClientConfig, Endpoint, RecvStream, SendStream};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    RootCertStore,
};
use tokio_util::codec::{FramedRead, FramedWrite};

/// QUIC Client for establishing connections to jetstream servers
pub struct Client {
    endpoint: Endpoint,
}

impl Client {
    /// Create a client without client authentication (server cert verification only)
    ///
    /// # Arguments
    /// * `ca_cert` - CA certificate to verify the server
    /// * `alpn_protocols` - ALPN protocols to negotiate (e.g., protocol version strings)
    pub fn new(
        ca_cert: CertificateDer<'static>,
        alpn_protocols: Vec<Vec<u8>>,
        socket_address: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut root_store = RootCertStore::empty();
        root_store.add(ca_cert)?;

        let mut tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        tls_config.alpn_protocols = alpn_protocols;

        let client_config = ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
        ));

        let mut endpoint = Endpoint::client(socket_address)?;
        endpoint.set_default_client_config(client_config);

        Ok(Self { endpoint })
    }

    /// Create a client with mTLS (client certificate authentication)
    ///
    /// # Arguments
    /// * `ca_cert` - CA certificate to verify the server
    /// * `client_cert` - Client certificate for authentication
    /// * `client_key` - Client private key
    /// * `alpn_protocols` - ALPN protocols to negotiate (e.g., protocol version strings)
    pub fn new_with_mtls(
        ca_cert: CertificateDer<'static>,
        client_cert: CertificateDer<'static>,
        client_key: PrivateKeyDer<'static>,
        alpn_protocols: Vec<Vec<u8>>,
        socket_address: SocketAddr,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut root_store = RootCertStore::empty();
        root_store.add(ca_cert)?;

        let mut tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(vec![client_cert], client_key)?;
        tls_config.alpn_protocols = alpn_protocols;

        let client_config = ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
        ));

        let mut endpoint = Endpoint::client(socket_address)?;
        endpoint.set_default_client_config(client_config);

        Ok(Self { endpoint })
    }

    /// Connect to a server
    ///
    /// # Arguments
    /// * `addr` - Server address to connect to
    /// * `server_name` - Server name for TLS verification (SNI)
    pub async fn connect(
        &self,
        addr: SocketAddr,
        server_name: &str,
    ) -> Result<quinn::Connection, Box<dyn std::error::Error + Send + Sync>>
    {
        Ok(self.endpoint.connect(addr, server_name)?.await?)
    }

    /// Get the underlying endpoint for advanced use cases
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
}

/// Client
pub struct QuicTransport<P: Protocol> {
    send_stream: FramedWrite<SendStream, ClientCodec<P>>,
    recv_stream: FramedRead<RecvStream, ClientCodec<P>>,
}

impl<P: Protocol> From<(SendStream, RecvStream)> for QuicTransport<P> {
    fn from((send, recv): (SendStream, RecvStream)) -> Self {
        Self {
            send_stream: FramedWrite::new(send, ClientCodec::<P>::default()),
            recv_stream: FramedRead::new(recv, ClientCodec::<P>::default()),
        }
    }
}

impl<P: Protocol> Sink<jetstream_rpc::Frame<P::Request>> for QuicTransport<P>
where
    Self: Unpin,
{
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_ready_unpin(cx)
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: jetstream_rpc::Frame<P::Request>,
    ) -> Result<(), Self::Error> {
        self.get_mut().send_stream.start_send_unpin(item)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_flush_unpin(cx)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.get_mut().send_stream.poll_close_unpin(cx)
    }
}

impl<P: Protocol> Stream for QuicTransport<P>
where
    Self: Unpin,
{
    type Item = Result<jetstream_rpc::Frame<P::Response>, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.get_mut().recv_stream.poll_next_unpin(cx)
    }
}
