use criterion::{criterion_group, criterion_main, Criterion};
use jetstream::prelude::*;
use jetstream_macros::service;

#[service]
pub trait Echo {
    async fn square(&self, i: u64) -> Result<String>;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct EchoImpl;

impl Echo for EchoImpl {
    async fn square(&self, i: u64) -> Result<String> {
        Ok((i * i).to_string())
    }
}

// QUIC benchmark
#[cfg(feature = "quic")]
mod quic_bench {
    use super::*;
    use echo_protocol::EchoChannel;
    use jetstream_quic::{
        Client, QuicRouter, QuicRouterHandler, QuicTransport, Server,
    };

    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use std::{
        net::SocketAddr,
        path::Path,
        sync::Arc,
        time::{Duration, Instant},
    };

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

    pub async fn server(
        addr: SocketAddr,
    ) -> std::result::Result<
        (),
        Box<dyn std::error::Error + Send + Sync + 'static>,
    > {
        let server_cert = load_certs(SERVER_CERT_PEM).pop().unwrap();
        let server_key = load_key(SERVER_KEY_PEM);
        let ca_cert = load_certs(CA_CERT_PEM).pop().unwrap();

        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(ca_cert).expect("Failed to add CA cert");
        let client_verifier =
            rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
                .allow_unauthenticated()
                .build()
                .expect("Failed to build client verifier");

        let echo_service = echo_protocol::EchoService {
            inner: super::EchoImpl {},
        };

        let rpc_router = Arc::new(
            jetstream_rpc::Router::new()
                .with_handler(echo_protocol::PROTOCOL_NAME, echo_service),
        );
        let quic_handler = QuicRouterHandler::new(rpc_router);

        let mut router = QuicRouter::new();
        router.register(Arc::new(quic_handler));

        let server = Server::new_with_mtls(
            vec![server_cert],
            server_key,
            client_verifier,
            addr,
            router,
        );

        server.run().await;
        Ok(())
    }

    pub async fn client_square(
        addr: SocketAddr,
        iters: u64,
    ) -> std::result::Result<Duration, Box<dyn std::error::Error + Send + Sync>>
    {
        let ca_cert = load_certs(CA_CERT_PEM).pop().unwrap();
        let client_cert = load_certs(CLIENT_CERT_PEM).pop().unwrap();
        let client_key = load_key(CLIENT_KEY_PEM);

        let alpn = vec![b"jetstream".to_vec()];
        let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let client = Client::new_with_mtls(
            ca_cert,
            client_cert,
            client_key,
            alpn,
            bind_addr,
        )?;

        let connection = client.connect(addr, "localhost").await?;

        let (send, recv) = connection.open_bi().await?;
        let transport: QuicTransport<EchoChannel> = (send, recv).into();
        let mut chan = EchoChannel::new(u16::MAX, Box::new(transport));
        chan.negotiate_version(u32::MAX).await?;

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
    // Install the ring crypto provider for rustls
    #[cfg(feature = "quic")]
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    #[cfg(any(feature = "quic", feature = "iroh"))]
    let mut group = c.benchmark_group("transport_comparison");
    #[cfg(any(feature = "quic", feature = "iroh"))]
    let rt = tokio::runtime::Runtime::new().unwrap();

    #[cfg(feature = "quic")]
    {
        use std::net::SocketAddr;
        let addr: SocketAddr = "127.0.0.1:4436".parse().unwrap();
        rt.block_on(async {
            tokio::spawn(quic_bench::server(addr));
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        });
        group.bench_function("quic", |b| {
            b.to_async(&rt).iter_custom(|iters| async move {
                quic_bench::client_square(addr, iters).await.unwrap()
            })
        });
    }

    #[cfg(feature = "iroh")]
    {
        use echo_protocol::{EchoChannel, EchoService};
        let (_router, chan) = rt.block_on(async {
            let router = jetstream_iroh::server_builder(EchoService {
                inner: EchoImpl {},
            })
            .await
            .unwrap();

            let addr = router.endpoint().addr();
            let transport = jetstream_iroh::client_builder::<EchoChannel>(addr)
                .await
                .unwrap();

            let chan = EchoChannel::new(u16::MAX, Box::new(transport));
            (router, chan)
        });

        group.bench_function("iroh", |b| {
            b.to_async(&rt).iter(|| async {
                chan.square(2).await.unwrap();
            });
        });
    }
    #[cfg(any(feature = "quic", feature = "iroh"))]
    group.finish();
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
