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

pub async fn client_builder<P: Protocol>(
    addr: EndpointAddr,
) -> Result<client::IrohTransport<P>, Box<dyn std::error::Error + 'static>> {
    let endpoint = endpoint_builder::<P>().bind().await.map_err(Box::new)?;
    let conn = endpoint
        .connect(addr, P::NAME.as_bytes())
        .await
        .map_err(Box::new)?;
    let streams = conn.open_bi().await?;
    Ok(IrohTransport::new_owned(streams, conn, endpoint))
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
