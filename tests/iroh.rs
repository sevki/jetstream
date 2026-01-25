#![cfg(feature = "iroh")]

use futures::{stream::FuturesUnordered, StreamExt};
use jetstream::prelude::*;

use crate::echo_protocol::{EchoChannel, EchoService};

#[service]
pub trait Echo {
    async fn ping(&self) -> Result<String>;
}

#[derive(Debug, Clone)]
struct EchoServer;

impl Echo for EchoServer {
    async fn ping(&self) -> Result<String> {
        Ok("pong".to_string())
    }
}

#[tokio::test]
async fn test_iroh_echo_service() {
    // Build the server router with the echo service
    let router = jetstream_iroh::server_builder(EchoService {
        inner: EchoServer {},
    })
    .await
    .unwrap();

    // get our own address. At this point we have a running router
    // that's ready to accept connections.
    let addr = router.endpoint().node_addr();

    // Build client transport and connect
    let transport = jetstream_iroh::client_builder::<EchoChannel>(addr)
        .await
        .unwrap();

    let ec = EchoChannel::new(10, Box::new(transport));
    let mut futures = FuturesUnordered::new();
    for _ in 0..10 {
        futures.push(ec.ping());
    }

    while let Some(result) = futures.next().await {
        let response = result.unwrap();
        assert_eq!(response, "pong".to_string());
    }

    // sleeep
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Gracefully close the endpoint & protocols.
    // This makes sure that remote nodes are notified about possibly still open connections
    // and any data is written to disk fully (or any other shutdown procedure for protocols).
    router.shutdown().await.unwrap();
}
