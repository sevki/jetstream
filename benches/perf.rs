use criterion::{criterion_group, criterion_main, Criterion};
use jetstream::prelude::*;
use jetstream_macros::service;

#[service]
pub trait Echo {
    async fn square(&mut self, i: u64) -> Result<String>;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct EchoImpl;

impl Echo for EchoImpl {
    async fn square(&mut self, i: u64) -> Result<String> {
        Ok((i * i).to_string())
    }
}

// QUIC benchmark
#[cfg(feature = "quic")]
mod quic_bench {
    use super::*;
    use echo_protocol::EchoChannel;
    use jetstream_rpc::{client::ClientCodec, server::run, Framed};
    use s2n_quic::{client::Connect, provider::tls, Client, Server};
    use std::{
        net::SocketAddr,
        path::Path,
        time::{Duration, Instant},
    };

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

    pub async fn server() -> std::result::Result<
        (),
        Box<dyn std::error::Error + Send + Sync + 'static>,
    > {
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
            tokio::spawn(async move {
                while let Ok(Some(stream)) =
                    connection.accept_bidirectional_stream().await
                {
                    tokio::spawn(async move {
                        let echo = super::EchoImpl {};
                        let servercodec: jetstream::prelude::server::ServerCodec<
                            echo_protocol::EchoService<super::EchoImpl>,
                        > = Default::default();
                        let framed = Framed::new(stream, servercodec);
                        let mut serv =
                            echo_protocol::EchoService { inner: echo };
                        if let Err(e) = run(&mut serv, framed).await {
                            eprintln!("QUIC server stream error: {:?}", e);
                        }
                    });
                }
            });
        }
        Ok(())
    }

    pub async fn client_square(
        iters: u64,
    ) -> std::result::Result<Duration, Box<dyn std::error::Error>> {
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
        connection.keep_alive(true)?;

        let stream = connection.open_bidirectional_stream().await?;
        let client_codec: ClientCodec<EchoChannel> = Default::default();
        let mut framed = Framed::new(stream, client_codec);
        let mut chan = EchoChannel {
            inner: Box::new(&mut framed),
        };

        let start = Instant::now();
        for i in 0..iters {
            if let Err(e) = chan.square(i).await {
                eprintln!("QUIC square error: {:?}", e);
                break;
            }
        }
        drop(chan);
        Ok(start.elapsed())
    }
}

fn benchmarks(#[allow(unused)] c: &mut Criterion) {
    #[cfg(any(feature = "quic", feature = "iroh"))]
    let mut group = c.benchmark_group("transport_comparison");
    #[cfg(any(feature = "quic", feature = "iroh"))]
    let rt = tokio::runtime::Runtime::new().unwrap();

    #[cfg(feature = "quic")]
    {
        group.bench_function("quic", |b| {
            b.to_async(&rt).iter_custom(|iters| async move {
                tokio::spawn(quic_bench::server());
                tokio::time::sleep(tokio::time::Duration::from_millis(100))
                    .await;
                quic_bench::client_square(iters).await.unwrap()
            })
        });
    }

    #[cfg(feature = "iroh")]
    {
        use echo_protocol::{EchoChannel, EchoService};

        group.bench_function("iroh", |b| {
            b.to_async(&rt).iter(|| async {
                let (_router, mut transport) = {
                    let router = jetstream_iroh::server_builder(EchoService {
                        inner: EchoImpl {},
                    })
                    .await
                    .unwrap();

                    let addr = router.endpoint().node_addr();
                    let transport =
                        jetstream_iroh::client_builder::<EchoChannel>(addr)
                            .await
                            .unwrap();

                    (router, transport)
                };

                let mut chan = EchoChannel {
                    inner: Box::new(&mut transport),
                };

                chan.square(2).await.unwrap();
            });
        });
    }
    #[cfg(any(feature = "quic", feature = "iroh"))]
    group.finish();
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
