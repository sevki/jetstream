#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod client;
mod jetstream_over_quic;
mod quic_handler;
mod router;
mod server;

pub use client::{Client, QuicTransport};
pub use jetstream_over_quic::QuicRouterHandler;
pub use quic_handler::QuicHandler;
pub use router::Router as QuicRouter;
pub use server::Server;
