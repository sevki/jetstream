#![cfg(feature = "client")]
use std::time::Instant;

use argh::FromArgs;
use jetstream::{
    cloudflare::JETSTREAM_PROTO_HEADER_KEY,
    prelude::tracing,
    websocket::tokio_tungstenite::{
        connect_async, tungstenite::client::IntoClientRequest,
    },
};
use jetstream_radar::{
    radar_protocol::{self, RadarChannel},
    Radar,
};
use reqwest::header::HeaderValue;
use url::Url;

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
struct Client {
    #[argh(subcommand)]
    nested: Nested,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Nested {
    Ping(Ping),
}

#[derive(FromArgs, PartialEq, Debug)]
/// First subcommand.
#[argh(subcommand, name = "ping")]
struct Ping {
    #[argh(
        option,
        default = "Url::parse(\"wss://radar.jetstream.rs\").unwrap()"
    )]
    /// url to call
    url: Url,
}

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let args: Client = argh::from_env();
    match args.nested {
        Nested::Ping(web_socket) => {
            tracing::info!("Connecting to {}", web_socket.url);
            let mut req = web_socket.url.clone().into_client_request().unwrap();
            // this is a custom header, doesn't have anything to do with websocket handshake
            req.headers_mut().insert(
                JETSTREAM_PROTO_HEADER_KEY,
                HeaderValue::from_static(radar_protocol::PROTOCOL_VERSION),
            );

            tracing::info!("Attempting websocket connection...");
            let (ws_stream, response) = connect_async(req).await.unwrap();
            tracing::info!(
                "Connected! Response status: {:?}",
                response.status()
            );

            let mut ws_transport = jetstream::websocket::WebSocketTransport::<
                RadarChannel,
            >::from(ws_stream);

            let mut client = radar_protocol::RadarChannel {
                inner: Box::new(&mut ws_transport),
            };

            tracing::info!("Sending ping...");
            let now = Instant::now();
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                client.ping(),
            )
            .await
            {
                Ok(Ok(_)) => {
                    println!("pong {}ms", now.elapsed().as_millis());
                }
                Ok(Err(e)) => tracing::error!("Ping failed: {:?}", e),
                Err(_) => tracing::error!("Ping timed out after 5 seconds"),
            }
            tracing::info!("Client completed");
        }
    };
}
