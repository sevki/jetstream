#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
mod client;
mod server;

use std::fmt::Debug;

pub use client::IrohTransport;
use iroh::{
    address_lookup::{
        AddressLookupBuilder,
        pkarr::{PkarrPublisher, PkarrResolver},
    },
    protocol::Router,
    EndpointAddr, RelayConfig, RelayMap, RelayUrl,
};
use jetstream_rpc::{server::Server, Protocol};
pub use server::{IrohRouter, IrohServer};

pub extern crate iroh;

lazy_static::lazy_static!(
    static ref RELAY_URL: RelayUrl = RelayUrl::from(url::Url::parse("https://relay.jetstream.rs").unwrap());
    static ref DISCOVERY_URL: url::Url = url::Url::parse("https://discovery.jetstream.rs").unwrap();
);

fn jetstream_resolver() -> impl AddressLookupBuilder {
    PkarrResolver::builder(DISCOVERY_URL.clone())
}

fn jetstream_publisher_builder() -> impl AddressLookupBuilder {
    PkarrPublisher::builder(DISCOVERY_URL.clone())
}

pub fn endpoint_builder<P: Protocol>() -> iroh::endpoint::Builder {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Custom(RelayMap::from_iter([
            RelayConfig::new(RELAY_URL.clone(), None),
        ])))
        .alpns(vec![P::NAME.as_bytes().to_vec()])
        .address_lookup(jetstream_publisher_builder())
        .address_lookup(jetstream_resolver())
}

/// Build an iroh endpoint for local/test use with no external relay or discovery services.
///
/// Unlike [`endpoint_builder`], this builder uses [`RelayMode::Disabled`] and binds
/// explicitly to `127.0.0.1` so that the bound socket address is immediately usable
/// as a direct connection target — no relay or address-discovery latency involved.
pub fn test_endpoint_builder<P: Protocol>() -> iroh::endpoint::Builder {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .expect("127.0.0.1:0 is always a valid bind address")
        .alpns(vec![P::NAME.as_bytes().to_vec()])
}

pub async fn client_builder<P: Protocol>(
    addr: EndpointAddr,
) -> Result<client::IrohTransport<P>, Box<dyn std::error::Error + 'static>> {
    let endpoint = endpoint_builder::<P>().bind().await.map_err(Box::new)?;
    let conn = endpoint
        .connect(addr, P::NAME.as_bytes())
        .await
        .map_err(Box::new)?;
    Ok(IrohTransport::from(conn.open_bi().await?))
}

/// Build a client transport for local/test use with no external relay.
///
/// Unlike [`client_builder`], this does not require connectivity to any relay server.
pub async fn test_client_builder<P: Protocol>(
    addr: EndpointAddr,
) -> Result<client::IrohTransport<P>, Box<dyn std::error::Error + 'static>> {
    let endpoint = test_endpoint_builder::<P>().bind().await.map_err(Box::new)?;
    let conn = endpoint
        .connect(addr, P::NAME.as_bytes())
        .await
        .map_err(Box::new)?;
    Ok(IrohTransport::from(conn.open_bi().await?))
}

pub async fn server_builder<P: Server + Protocol + Debug + Clone + 'static>(
    inner: P,
) -> Result<Router, Box<dyn std::error::Error + 'static>> {
    let endpoint = endpoint_builder::<P>().bind().await.map_err(Box::new)?;

    let router = Router::builder(endpoint)
        .accept(P::NAME.as_bytes(), IrohServer::new(inner))
        .spawn();
    Ok(router)
}

/// Build a server router for local/test use with no external relay.
///
/// Unlike [`server_builder`], this does not require connectivity to any relay server.
/// The returned router's endpoint will have direct socket addresses immediately after
/// construction — no need to call `endpoint().online().await`.
pub async fn test_server_builder<P: Server + Protocol + Debug + Clone + 'static>(
    inner: P,
) -> Result<Router, Box<dyn std::error::Error + 'static>> {
    let endpoint = test_endpoint_builder::<P>().bind().await.map_err(Box::new)?;

    let router = Router::builder(endpoint)
        .accept(P::NAME.as_bytes(), IrohServer::new(inner))
        .spawn();
    Ok(router)
}
