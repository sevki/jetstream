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
    discovery::{
        pkarr::{PkarrPublisher, PkarrResolver},
        IntoDiscovery,
    },
    protocol::Router,
    NodeAddr,
};
use jetstream_rpc::Protocol;
pub use server::IrohServer;

pub extern crate iroh;

fn dt_resolver() -> impl IntoDiscovery {
    PkarrResolver::builder(
        url::Url::parse("https://discovery.devtoo.ls").unwrap(),
    )
    .build()
}

fn dt_publisher_builder() -> impl IntoDiscovery {
    PkarrPublisher::builder(
        url::Url::parse("https://discovery.devtoo.ls").unwrap(),
    )
}

pub fn endpoint_builder<P: Protocol>() -> iroh::endpoint::Builder {
    iroh::Endpoint::builder()
        .alpns(vec![P::VERSION.as_bytes().to_vec()])
        .add_discovery(dt_publisher_builder())
        .add_discovery(dt_resolver())
}

pub async fn client_builder<P: Protocol>(
    addr: NodeAddr,
) -> Result<client::IrohTransport<P>, Box<dyn std::error::Error + 'static>> {
    let endpoint = endpoint_builder::<P>().bind().await.map_err(Box::new)?;
    let conn = endpoint
        .connect(addr, P::VERSION.as_bytes())
        .await
        .map_err(Box::new)?;
    Ok(IrohTransport::from(conn.open_bi().await?))
}

pub async fn server_builder<P: Protocol + Debug + Clone + 'static>(
    inner: P,
) -> Result<Router, Box<dyn std::error::Error + 'static>> {
    let endpoint = endpoint_builder::<P>().bind().await.map_err(Box::new)?;

    let router = Router::builder(endpoint)
        .accept(P::VERSION.as_bytes(), IrohServer::new(inner))
        .spawn();
    Ok(router)
}
