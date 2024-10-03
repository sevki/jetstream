use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};

use futures::AsyncReadExt;
use jetstream_rpc::{
    JetStreamProtocol, JetStreamService,
};
use jetstream_wireformat::wire_format_extensions::AsyncWireFormatExt;
use okstd::okasync::{Runtime, Runtimes};
use s2n_quic::client::Connect;
use s2n_quic::provider::tls;

use s2n_quic::stream::SplittableStream;
use s2n_quic::Client;

type Error = anyhow::Error;

type Result<T> = std::result::Result<T, Error>;

pub struct Connection<P: JetStreamProtocol> {
    inner: s2n_quic::Connection,
    rt: Arc<Mutex<Runtimes>>,
    _phantom: std::marker::PhantomData<P>,
}

impl<P: JetStreamProtocol> Connection<P> {
    pub fn new(inner: s2n_quic::Connection, rt: Runtimes) -> Self {
        Connection {
            inner,
            rt: Arc::new(Mutex::new(rt)),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<P: JetStreamProtocol> Connection<P> {
    pub async fn new_handle(&mut self) -> Result<Handle<P>> {
        let stream = self.inner.open_bidirectional_stream().await?;
        Ok(Handle::<P> {
            stream,
            rt: self.rt.clone(),
            _phantom: std::marker::PhantomData,
        })
    }
    pub fn new_handle_sync(&mut self) -> Result<Handle<P>> {
        self.rt
            .clone()
            .lock()
            .unwrap()
            .block_on(async { self.new_handle().await })
    }
}

pub struct Handle<P: JetStreamProtocol> {
    stream: s2n_quic::stream::BidirectionalStream,
    rt: Arc<Mutex<Runtimes>>,
    _phantom: std::marker::PhantomData<P>,
}

impl<P: JetStreamProtocol> JetStreamProtocol for Handle<P> {
    type Request = P::Request;
    type Response = P::Response;
}

impl<P: JetStreamProtocol + Send + Sync> SplittableStream for Handle<P> {
    fn split(
        self,
    ) -> (
        Option<s2n_quic::stream::ReceiveStream>,
        Option<s2n_quic::stream::SendStream>,
    ) {
        let (r, s) = self.stream.split();
        (Some(r), Some(s))
    }
}

impl<P: JetStreamProtocol + Send + Sync> JetStreamService for Handle<P> {
    fn rpc(
        &mut self,
        req: Self::Request,
    ) -> std::prelude::v1::Result<
        Self::Response,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let rt = self.rt.clone();
        let rt = rt.lock().unwrap();
        rt.block_on(async {
            let _ = req.encode_async(&mut self.stream);
            let resp = P::Response::decode_async(&mut self.stream).await;
            resp.map_err(|e| e.into())
        })
    }
}

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
    pub async fn dial(self) -> anyhow::Result<s2n_quic::Connection> {
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
