#![cfg(feature = "iroh")]
use std::fmt::Debug;

use jetstream::prelude::*;
use jetstream_iroh::iroh::Watcher;

use crate::echo_protocol::{EchoChannel, EchoService};

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<String, Error>;
}

#[derive(Debug, Clone)]
struct EchoServer;

impl Echo for EchoServer {
    async fn ping(&mut self) -> Result<String, Error> {
        Ok("pong".to_string())
    }
}

impl<P: Echo> Debug for EchoService<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EchoService").finish()
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
    let addr = router.endpoint().node_addr().initialized().await;

    // Build client transport and connect
    let mut transport = jetstream_iroh::client_builder::<EchoChannel>(addr)
        .await
        .unwrap();

    let mut ec = EchoChannel {
        inner: Box::new(&mut transport),
    };
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
