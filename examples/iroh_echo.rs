use crate::echo_protocol::{EchoChannel, EchoService};
use jetstream::prelude::*;
use jetstream_iroh::{
    iroh::{endpoint::VarInt, protocol::Router, Endpoint, Watcher},
    IrohTransport,
};
use jetstream_macros::service;
use okstd::prelude::*;
use std::fmt::Debug;

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

impl<P: Echo> Debug for EchoService<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EchoService").finish()
    }
}

#[okstd::main]
async fn main() {
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

    // get our own address. At this point we have a running router
    // that's ready to accept connections.
    let addr = router.endpoint().node_addr().initialized().await;

    let client_endpoint =
        Endpoint::builder().discovery_n0().bind().await.unwrap();

    // Open a connection to the accepting node
    let conn = client_endpoint
        .connect(addr, echo_protocol::PROTOCOL_VERSION.as_bytes())
        .await
        .unwrap();

    // Open a bidirectional QUIC stream
    let dup = conn.open_bi().await.unwrap();

    let mut transport: IrohTransport<EchoChannel> = dup.into();

    let mut ec = EchoChannel {
        inner: Box::new(&mut transport),
    };
    for i in 0..10 {
        let b = ec.square(i).await.unwrap();
        println!("square response: {i} * {i} = {b}");
    }
    conn.close(VarInt::from_u32(1), "reason".as_bytes());

    router.shutdown().await.unwrap();
}
