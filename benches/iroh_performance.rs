use crate::echo_protocol::{EchoChannel, EchoService};
use criterion::{criterion_group, criterion_main, Criterion};
use jetstream::prelude::*;
use jetstream_iroh::{
    iroh::{protocol::Router, Endpoint, Watcher},
    IrohTransport,
};
use jetstream_macros::service;
use std::{fmt::Debug, sync::Arc};
use tokio::sync::Mutex;

#[service]
pub trait Echo {
    async fn square(&mut self, i: u64) -> Result<String, Error>;
}

#[derive(Debug, Clone)]
struct EchoServer;

impl Echo for EchoServer {
    async fn square(&mut self, i: u64) -> Result<String, Error> {
        Ok((i * i).to_string())
    }
}

impl<P: Echo> Debug for EchoService<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EchoService").finish()
    }
}

fn iroh_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_router, mut transport) = rt.block_on(async {
        // Build an endpoint, defaulting to the public n0 relay network
        let endpoint = Endpoint::builder().discovery_n0().bind().await.unwrap();

        // configure the blobs protocol to run in-memory
        let srv = jetstream_iroh::IrohServer::new(EchoService {
            inner: EchoServer {},
        });

        // Build our router and add the blobs protocol,
        // identified by its ALPN. Spawn the router to start listening.
        let router = Router::builder(endpoint)
            .accept(echo_protocol::PROTOCOL_VERSION, srv)
            .spawn();

        let client_endpoint =
            Endpoint::builder().discovery_n0().bind().await.unwrap();

        let addr = router.endpoint().node_addr().initialized().await;

        // Open a connection to the accepting node
        let conn = client_endpoint
            .connect(addr, echo_protocol::PROTOCOL_VERSION.as_bytes())
            .await
            .unwrap();

        // Open a bidirectional QUIC stream
        let dup = conn.open_bi().await.unwrap();

        let transport: IrohTransport<EchoChannel> = dup.into();

        (router, transport)
    });
    let ec = Arc::new(Mutex::new(EchoChannel {
        inner: Box::new(&mut transport),
    }));
    c.bench_function("iroh", |b| {
        b.to_async(&rt).iter(|| async {
            let binding = ec.clone();

            let mut a = binding.lock().await;
            a.square(2).await.unwrap();
        });
    });
}

criterion_group!(benches, iroh_benchmark);
criterion_main!(benches);
