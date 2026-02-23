#![cfg(feature = "iroh")]
use crate::square_protocol::{SquareChannel, SquareService};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use jetstream::prelude::*;
use jetstream_macros::service;

#[service(tracing)]
pub trait Square {
    async fn square(&self, ctx: Context, i: u32) -> Result<String>;
}

#[derive(Debug, Clone)]
struct SquareServer;

impl Square for SquareServer {
    async fn square(&self, _ctx: Context, i: u32) -> Result<String> {
        Ok((i * i).to_string())
    }
}

#[tokio::main]
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
    // Build the server router with the echo service
    let router = jetstream_iroh::server_builder(SquareService {
        inner: SquareServer {},
    })
    .await
    .unwrap();

    // get our own address. At this point we have a running router
    // that's ready to accept connections.
    let addr = router.endpoint().addr();

    // Build client transport and connect
    let transport = jetstream_iroh::client_builder::<SquareChannel>(addr)
        .await
        .unwrap();

    let ec = SquareChannel::new(10, Box::new(transport));
    let mut futures = FuturesUnordered::new();
    for i in 0..1000 {
        futures.push(ec.square(Context::default(), i));
    }

    while let Some(result) = futures.next().await {
        let response = result.unwrap();
        println!("Response: {}", response);
    }
    router.shutdown().await.unwrap();
}
