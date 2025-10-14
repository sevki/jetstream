#![cfg(feature = "iroh")]
use crate::echo_protocol::{EchoChannel, EchoService};
use criterion::{criterion_group, criterion_main, Criterion};
use jetstream::prelude::*;
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

fn iroh_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_router, mut transport) = rt.block_on(async {
        // Build the server router with the echo service
        let router = jetstream_iroh::server_builder(EchoService {
            inner: EchoServer {},
        })
        .await
        .unwrap();

        let addr = router.endpoint().node_addr();

        // Build client transport and connect
        let transport = jetstream_iroh::client_builder::<EchoChannel>(addr)
            .await
            .unwrap();

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
