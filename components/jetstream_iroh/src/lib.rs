#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
mod client;
mod server;
mod session;

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
use jetstream_rpc::{server::Server, session::Session, Protocol};
pub use server::{IrohRouter, IrohServer};
pub use session::{IrohServiceLane, IrohSession};

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

/// r[impl jetstream.session.conformance.iroh]
/// Dial `addr` and keep the connection as a session, so the caller can
/// open as many lanes as it wants rather than being handed one.
pub async fn session_builder<P: Protocol>(
    addr: EndpointAddr,
) -> Result<IrohSession<P>, Box<dyn std::error::Error + 'static>> {
    let endpoint = endpoint_builder::<P>().bind().await.map_err(Box::new)?;
    let conn = endpoint
        .connect(addr, P::NAME.as_bytes())
        .await
        .map_err(Box::new)?;
    Ok(IrohSession::new_owned(conn, endpoint))
}

/// Dial `addr` and open one lane on it.
///
/// r[impl jetstream.session.compat.existing-clients]
/// This is the pre-session shape: one lane, tag-multiplexed by whatever
/// the caller wraps it in. It stays conforming, and is now a session
/// with one lane opened on it rather than a connection with its extra
/// lanes thrown away — the lane keeps the session alive, so a caller
/// that wants a second lane can ask for a session instead.
pub async fn client_builder<P>(
    addr: EndpointAddr,
) -> Result<client::IrohTransport<P>, Box<dyn std::error::Error + 'static>>
where
    P: Protocol<Error = jetstream_rpc::Error> + 'static,
{
    let session = session_builder::<P>(addr).await?;
    let lane = session.open_lane().await.map_err(Box::new)?;
    Ok(lane)
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
