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

#[tokio::test(flavor = "multi_thread")]
async fn test_iroh_echo_service() {
    test_iroh_echo_service_inner().await;
}

async fn test_iroh_echo_service_inner() {
    // Build the server router without external relay so the test works in CI
    let router = jetstream_iroh::test_server_builder(EchoService {
        inner: EchoServer {},
    })
    .await
    .unwrap();

    // Build the server's EndpointAddr from its public key and bound loopback
    // sockets — no relay connection or address-discovery required.
    let server_id = router.endpoint().id();
    let server_sockets = router.endpoint().bound_sockets();
    let server_addr = jetstream_iroh::iroh::EndpointAddr::from_parts(
        server_id,
        server_sockets
            .into_iter()
            .map(jetstream_iroh::iroh::TransportAddr::Ip),
    );

    // Build client transport using the relay-free test builder
    let transport = jetstream_iroh::test_client_builder::<EchoChannel>(server_addr)
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
