#![cfg(feature = "iroh")]

use jetstream::prelude::*;

use crate::echo_protocol::{EchoChannel, EchoService};

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<String>;
}

#[derive(Debug, Clone)]
struct EchoServer;

impl Echo for EchoServer {
    async fn ping(&mut self) -> Result<String> {
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

    let mut ec = EchoChannel::new(10, Box::new(transport));
    for _ in 0..10 {
        let b = ec.ping().await.unwrap();
        println!("Ping response: {:?}", b);
    }

    // sleeep
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Gracefully close the endpoint & protocols.
    // This makes sure that remote nodes are notified about possibly still open connections
    // and any data is written to disk fully (or any other shutdown procedure for protocols).
    router.shutdown().await.unwrap();
}
