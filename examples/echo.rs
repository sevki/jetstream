use std::{net::SocketAddr, path::Path};

use echo_protocol::EchoChannel;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_rpc::Framed;
use okstd::prelude::*;
use s2n_quic::{client::Connect, provider::tls, Client, Server};

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<(), Error>;
}

struct EchoImpl {}

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
        .with_io("127.0.0.1:4433")?
        .start()?;

    while let Some(mut connection) = server.accept().await {
        // spawn a new task for the connection
        tokio::spawn(async move {
            eprintln!(
                "Connection accepted from {:?}",
                connection.remote_addr()
            );

            while let Ok(Some(stream)) =
                connection.accept_bidirectional_stream().await
            {
                // spawn a new task for the stream
                tokio::spawn(async move {
                    eprintln!(
                        "Stream opened from {:?}",
                        &stream.connection().remote_addr()
                    );
                    let echo = EchoImpl {};
                    let servercodec: jetstream::prelude::server::service::ServerCodec<
                        echo_protocol::EchoService<EchoImpl>,
                    > = Default::default();
                    let framed = Framed::new(stream, servercodec);
                    let mut serv = echo_protocol::EchoService { inner: echo };
                    server::service::run(&mut serv, framed).await.unwrap();
                });
            }
        });
    }

    Ok(())
}

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

    let addr: SocketAddr = "127.0.0.1:4433".parse()?;
    let connect = Connect::new(addr).with_server_name("localhost");
    let mut connection = client.connect(connect).await?;

    // ensure the connection doesn't time out with inactivity
    connection.keep_alive(true)?;

    // open a new stream and split the receiving and sending sides
    let stream = connection.open_bidirectional_stream().await?;
    let client_codec: jetstream_client::ClientCodec<EchoChannel> =
        Default::default();
    let mut framed = Framed::new(stream, client_codec);
    let mut chan = EchoChannel {
        inner: Box::new(&mut framed),
    };
    eprintln!("Ping sent");
    chan.ping().await?;
    eprintln!("Pong received");
    Ok(())
}

#[okstd::main]
async fn main() {
    tokio::select! {
      _ = server() => {},
      _ = client() => {},
    }
}
