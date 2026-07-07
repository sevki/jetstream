#![cfg(feature = "iroh")]

use futures::{stream::FuturesUnordered, StreamExt};
use iroh::{RelayMode, endpoint::presets, tls::CaTlsConfig};
use jetstream::prelude::*;
use jetstream_iroh::{IrohServer, IrohTransport, iroh::protocol::Router};

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
    // Start a local relay server so both endpoints connect without external network
    let (relay_map, _relay_url, _relay_server) =
        iroh::test_utils::run_relay_server().await.unwrap();

    // Build the server endpoint with the local relay (self-signed cert, skip verify)
    let server_endpoint = iroh::Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Custom(relay_map.clone()))
        .ca_tls_config(CaTlsConfig::insecure_skip_verify())
        .alpns(vec![EchoChannel::NAME.as_bytes().to_vec()])
        .bind()
        .await
        .unwrap();

    let router = Router::builder(server_endpoint)
        .accept(
            EchoChannel::NAME.as_bytes(),
            IrohServer::new(EchoService {
                inner: EchoServer {},
            }),
        )
        .spawn();

    // Wait for the server endpoint to be reachable via the local relay
    router.endpoint().online().await;

    // Retrieve the server's address (relay URL + any direct addrs)
    let server_addr = router.endpoint().addr();

    // Build the client endpoint with the same local relay
    let client_endpoint = iroh::Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Custom(relay_map))
        .ca_tls_config(CaTlsConfig::insecure_skip_verify())
        .alpns(vec![EchoChannel::NAME.as_bytes().to_vec()])
        .bind()
        .await
        .unwrap();

    // Connect to the server
    let conn = client_endpoint
        .connect(server_addr, EchoChannel::NAME.as_bytes())
        .await
        .unwrap();

    let transport = IrohTransport::<EchoChannel>::from(conn.open_bi().await.unwrap());

    let ec = EchoChannel::new(10, Box::new(transport));
    let mut futures = FuturesUnordered::new();
    for _ in 0..10 {
        futures.push(ec.ping());
    }

    while let Some(result) = futures.next().await {
        let response = result.unwrap();
        assert_eq!(response, "pong".to_string());
    }

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    router.shutdown().await.unwrap();
}
