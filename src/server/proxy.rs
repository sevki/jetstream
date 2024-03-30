use std::{net::SocketAddr, path::Path, fmt::Debug};

use anyhow::Ok;
use crate::coding::{Rframe, Tframe};
use s2n_quic::{
    client::{Client, Connect},
    provider::tls,
};

use slog_scope::{debug, error};

use crate::wire_format_extensions::AsyncWireFormatExt;

#[derive(Debug, Clone)]
/// Represents a DialQuic struct.
pub struct DialQuic {
    host: String,
    port: u16,
    client_cert: Box<Path>,
    key: Box<Path>,
    ca_cert: Box<Path>,
    hostname: String,
}

impl DialQuic {
    /// Creates a new instance of `DialQuic`.
    ///
    /// # Arguments
    ///
    /// * `host` - The host to connect to.
    /// * `port` - The port to connect to.
    /// * `cert` - The path to the client certificate file.
    /// * `key` - The path to the client private key file.
    /// * `ca_cert` - The path to the CA certificate file.
    /// * `hostname` - The hostname for the TLS handshake.
    ///
    /// # Returns
    ///
    /// A new instance of `DialQuic`.
    pub fn new(
        host: String,
        port: u16,
        cert: Box<Path>,
        key: Box<Path>,
        ca_cert: Box<Path>,
        hostname: String,
    ) -> Self {
        Self {
            host,
            port,
            client_cert: cert,
            key,
            ca_cert,
            hostname,
        }
    }
}

/// Establishes a QUIC connection with the specified server.
///
/// This function dials a QUIC connection using the provided certificates and keys.
/// It creates a TLS client with the given CA certificate, client certificate, and client key.
/// The connection is established with the specified server address and hostname.
/// The connection is configured to keep alive and not time out due to inactivity.
///
/// # Arguments
///
/// - `self`: The `DialQuic` instance.
///
/// # Returns
///
/// Returns a `Result` containing the established `s2n_quic::Connection` if successful,
/// or an `anyhow::Error` if an error occurs during the connection establishment.
impl DialQuic {
    async fn dial(self) -> anyhow::Result<s2n_quic::Connection> {
        let ca_cert = self.ca_cert.to_str().unwrap();
        let client_cert = self.client_cert.to_str().unwrap();
        let client_key = self.key.to_str().unwrap();
        let tls = tls::default::Client::builder()
            .with_certificate(Path::new(ca_cert))?
            .with_client_identity(
                Path::new(client_cert),
                Path::new(client_key),
            )?
            .build()?;

        let client = Client::builder()
            .with_tls(tls)?
            .with_io("0.0.0.0:0")?
            .start()?;

        let host_port = format!("{}:{}", self.host, self.port);

        let addr: SocketAddr = host_port.parse()?;
        let connect = Connect::new(addr).with_server_name(&*self.hostname);
        let mut connection = client.connect(connect).await?;

        // ensure the connection doesn't time out with inactivity
        connection.keep_alive(true)?;
        Ok(connection)
    }
}

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_vsock::{VsockListener, VsockAddr};

#[async_trait::async_trait]
pub trait ListenerStream: Send + Sync + Debug+ 'static{
    type Stream: AsyncRead + AsyncWrite + Unpin + Send + Sync;
    type Addr: std::fmt::Debug;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)>;
}

#[async_trait::async_trait]
impl ListenerStream for tokio::net::UnixListener {
    type Stream = tokio::net::UnixStream;
    type Addr = tokio::net::unix::SocketAddr;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)> {
        tokio::net::UnixListener::accept(self).await
    }
}

#[async_trait::async_trait]
impl ListenerStream for VsockListener {
    type Stream = tokio_vsock::VsockStream;
    type Addr = VsockAddr;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)> {
        VsockListener::accept(self).await
    }
}

pub struct Proxy<L>
where
    L: ListenerStream,
{
    dial: DialQuic,
    listener: L,
}

impl<L> Proxy<L>
where
    L: ListenerStream,
{
    pub fn new(dial: DialQuic, listener: L) -> Self {
        Self { dial, listener }
    }

    pub async fn run(&mut self) {
        debug!("Listening on {:?}", self.listener);
        while let std::result::Result::Ok((down_stream, _)) = self.listener.accept().await {
            debug!("Accepted connection from");
            let down_stream = down_stream;
            let dial = self.dial.clone();
            tokio::spawn(async move {
                debug!("Dialing {:?}", dial);
                let mut dial = dial.clone().dial().await.unwrap();
                debug!("Connected to {:?}", dial.remote_addr());
                let up_stream = dial.open_bidirectional_stream().await.unwrap();
                let (rx, mut tx) = up_stream.split();
                let (read, mut write) = tokio::io::split(down_stream);
                let mut upstream_reader = tokio::io::BufReader::new(rx);
                let mut downstream_reader = tokio::io::BufReader::new(read);
                loop {
                    // read and send to up_stream
                    {
                        debug!("Reading from down_stream");
                        let tframe = Tframe::decode_async(&mut downstream_reader).await;
                        if let Err(e) = tframe {
                            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                break;
                            } else {
                                error!("Error decoding from down_stream: {:?}", e);
                                break;
                            }
                        } else if let std::io::Result::Ok(tframe) = tframe {
                            debug!("Sending to up_stream {:?}", tframe);
                            tframe.encode_async(&mut tx).await.unwrap();
                        }
                    }
                    // write and send to down_stream
                    {
                        debug!("Reading from up_stream");
                        let rframe = Rframe::decode_async(&mut upstream_reader).await.unwrap();
                        debug!("Sending to down_stream");
                        rframe.encode_async(&mut write).await.unwrap();
                    }
                }
            });
        }
    }
}