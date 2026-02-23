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
        pkarr::{PkarrPublisher, PkarrResolver},
        IntoAddressLookup,
    },
    protocol::Router,
    EndpointAddr, RelayConfig, RelayMap, RelayUrl,
};
use jetstream_rpc::{server::Server, Protocol};
pub use server::{IrohRouter, IrohServer};

pub extern crate iroh;

fn jetstream_resolver() -> impl IntoAddressLookup {
    PkarrResolver::builder(
        url::Url::parse("https://discovery.jetstream.rs").unwrap(),
    )
}

fn jetstream_publisher_builder() -> impl IntoAddressLookup {
    PkarrPublisher::builder(
        url::Url::parse("https://discovery.jetstream.rs").unwrap(),
    )
}

lazy_static::lazy_static!(
    static ref RELAY_URL: RelayUrl = RelayUrl::from(url::Url::parse("https://relay.jetstream.rs").unwrap());
);

pub fn endpoint_builder<P: Protocol>() -> iroh::endpoint::Builder {
    iroh::Endpoint::builder()
        .relay_mode(iroh::RelayMode::Custom(RelayMap::from_iter([
            RelayConfig {
                url: RELAY_URL.clone(),
                quic: None,
            },
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
