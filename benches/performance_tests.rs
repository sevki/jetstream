use {
    criterion::{criterion_group, criterion_main, Criterion},
    echo_protocol::EchoChannel,
    jetstream::prelude::*,
    jetstream_rpc::Framed,
    s2n_quic::{client::Connect, provider::tls, Client, Server},
    std::{
        net::SocketAddr,
        path::Path,
        time::{Duration, Instant},
    },
};

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<String, Error>;
}

struct EchoImpl {}

impl Echo for EchoImpl {
    async fn ping(&mut self) -> Result<String, Error> {
        Ok("pong".to_string())
    }
}

pub static CA_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca-cert.pem");
pub static CLIENT_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client-cert.pem");
pub static CLIENT_KEY_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client-key.pem");
pub static SERVER_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server-cert.pem");
pub static SERVER_KEY_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server-key.pem");

async fn server() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let tls = tls::default::Server::builder()
        .with_trusted_certificate(Path::new(CA_CERT_PEM))?
        .with_certificate(Path::new(SERVER_CERT_PEM), Path::new(SERVER_KEY_PEM))?
        .with_client_authentication()?
        .build()?;

    let mut server = Server::builder()
        .with_tls(tls)?
        .with_io("127.0.0.1:4433")?
        .start()?;

    while let Some(mut connection) = server.accept().await {
        // spawn a new task for the connection
        tokio::spawn(async move {
            while let Ok(Some(stream)) = connection.accept_bidirectional_stream().await {
                // spawn a new task for the stream
                tokio::spawn(async move {
                    let echo = EchoImpl {};
                    let servercodec: jetstream::prelude::server::service::ServerCodec<
                        echo_protocol::EchoService<EchoImpl>,
                    > = Default::default();
                    let framed = Framed::new(stream, servercodec);
                    let mut serv = echo_protocol::EchoService { inner: echo };
                    if let Err(e) = server::service::run(&mut serv, framed).await {
                        eprintln!("Server stream error: {:?}", e);
                    }
                });
            }
        });
    }

    Ok(())
}

async fn client(iters: u64) -> Result<Duration, Box<dyn std::error::Error>> {
    let tls = tls::default::Client::builder()
        .with_certificate(Path::new(CA_CERT_PEM))?
        .with_client_identity(Path::new(CLIENT_CERT_PEM), Path::new(CLIENT_KEY_PEM))?
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
    let client_codec: jetstream_client::ClientCodec<EchoChannel> = Default::default();
    let mut framed = Framed::new(stream, client_codec);
    let mut chan = EchoChannel {
        inner: Box::new(&mut framed),
    };
    let start = Instant::now();
    for _ in 0..iters {
        if let Err(e) = chan.ping().await {
            eprintln!("Ping error: {:?}", e);
            break;
        }
    }
    drop(chan);
    Ok(start.elapsed())
}

fn jetstream_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("ping-pong", |b| {
        b.to_async(&rt).iter_custom(|iters| {
            async move {
                tokio::spawn(server());
                client(iters).await.unwrap()
            }
        })
    });
}

criterion_group!(benches, jetstream_benchmark);
criterion_main!(benches);
