use std::{net::SocketAddr, path::Path};

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_rpc::{client::ClientCodec, server::run, Framed};
use okstd::prelude::*;
use s2n_quic::{client::Connect, provider::tls, Client, Server};

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
    ) -> Result<String, Error>;

    /// This method uses default auto-instrumentation from #[service(tracing)]
    async fn echo(&mut self, text: String) -> Result<String, Error>;
}

struct EchoImpl {}

impl Echo for EchoImpl {
    async fn ping(
        &mut self,
        ctx: Context,
        message: String,
    ) -> Result<String, Error> {
        tracing::info!("Ping received: {} {:?} ", message, ctx);

        Ok(format!("Pong: {}", message))
    }

    async fn echo(&mut self, text: String) -> Result<String, Error> {
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

#[cfg(not(windows))]
async fn server() -> Result<(), Box<dyn std::error::Error>> {
    let tls = tls::default::Server::builder()
        .with_trusted_certificate(Path::new(CA_CERT_PEM))?
        .with_certificate(
            Path::new(SERVER_CERT_PEM),
            Path::new(SERVER_KEY_PEM),
        )?
        .with_client_authentication()?
        .build()?;

    let mut server = Server::builder()
        .with_tls(tls)?
        .with_io("127.0.0.1:4434")?
        .start()?;

    tracing::info!("Server listening on 127.0.0.1:4434");

    while let Some(mut connection) = server.accept().await {
        tokio::spawn(async move {
            tracing::info!(
                remote_addr = ?connection.remote_addr(),
                "Connection accepted"
            );

            while let Ok(Some(stream)) =
                connection.accept_bidirectional_stream().await
            {
                tokio::spawn(async move {
                    let echo = EchoImpl {};
                    let servercodec: jetstream::prelude::server::ServerCodec<
                        echo_protocol::EchoService<EchoImpl>,
                    > = Default::default();
                    let framed = Framed::new(stream, servercodec);
                    let mut serv = echo_protocol::EchoService { inner: echo };
                    run(&mut serv, framed).await.unwrap();
                });
            }
        });
    }

    Ok(())
}

#[cfg(not(windows))]
async fn client() -> Result<(), Box<dyn std::error::Error>> {
    let tls = tls::default::Client::builder()
        .with_certificate(Path::new(CA_CERT_PEM))?
        .with_client_identity(
            Path::new(CLIENT_CERT_PEM),
            Path::new(CLIENT_KEY_PEM),
        )?
        .build()?;

    let client = Client::builder()
        .with_tls(tls)?
        .with_io("0.0.0.0:0")?
        .start()?;

    let addr: SocketAddr = "127.0.0.1:4434".parse()?;
    let connect = Connect::new(addr).with_server_name("localhost");
    let mut connection = client.connect(connect).await?;

    connection.keep_alive(true)?;

    let stream = connection.open_bidirectional_stream().await?;
    let client_codec: ClientCodec<EchoChannel> = Default::default();
    let mut framed = Framed::new(stream, client_codec);
    let mut chan = EchoChannel {
        inner: Box::new(&mut framed),
    };

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

#[okstd::main]
#[cfg(not(windows))]
async fn main() {
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

    tokio::select! {
      _ = server() => {
          tracing::info!("Server exited");
      },
      _ = client() => {
          tracing::info!("Client exited");
      },
    }
}

#[cfg(windows)]
fn main() {}
