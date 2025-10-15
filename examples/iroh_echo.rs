#![cfg(feature = "iroh")]
use crate::echo_protocol::{EchoChannel, EchoService};
use jetstream::prelude::*;
use jetstream_macros::service;
use okstd::prelude::*;

#[service]
pub trait Echo {
    async fn square(&mut self, i: u32) -> Result<String, Error>;
}

#[derive(Debug, Clone)]
struct EchoServer;

impl Echo for EchoServer {
    async fn square(&mut self, i: u32) -> Result<String, Error> {
        Ok((i * i).to_string())
    }
}

#[okstd::main]
async fn main() {
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
    let mut transport = jetstream_iroh::client_builder::<EchoChannel>(addr)
        .await
        .unwrap();

    let mut ec = EchoChannel {
        inner: Box::new(&mut transport),
    };
    for i in 0..10 {
        let b = ec.square(i).await.unwrap();
        println!("square response: {i} * {i} = {b}");
    }

    router.shutdown().await.unwrap();
}
