//! The same session model over a real QUIC connection.
//!
//! `session_lanes.rs` shows the model with no network under it. This one
//! stands up an mTLS QUIC connection on loopback with the certificates in
//! `certs/` and does the same things over it: reads the session's
//! capabilities, learns who the peer is from the handshake, opens several
//! independent lanes on the one connection, and sends a datagram
//! alongside them.
//!
//! The iroh binding (`jetstream_iroh::IrohSession`) is the same shape;
//! the only differences are in the row it reports — identity by public
//! key rather than by certificate.
//!
//! ```console
//! cargo run --example quic_session --features quic
//! ```

use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};

use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_quic::QuicSession;
use jetstream_rpc::{
    context::Peer,
    session::{decode_datagram, Capabilities, Capability, Datagrams, Session},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::time::{sleep, timeout};

use crate::sleepy_protocol::{SleepyChannel, SleepyService, Tmessage, Tnap};

#[service]
pub trait Sleepy {
    /// Nap for `ms` milliseconds, then say so.
    async fn nap(&self, ctx: Context, ms: u32) -> Result<String>;
}

#[derive(Debug, Clone)]
struct SleepyServer;

impl Sleepy for SleepyServer {
    async fn nap(&self, _ctx: Context, ms: u32) -> Result<String> {
        sleep(Duration::from_millis(ms as u64)).await;
        Ok(format!("slept {ms}ms"))
    }
}

static CA_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca.pem");
static CLIENT_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client.pem");
static CLIENT_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client.key");
static SERVER_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.pem");
static SERVER_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.key");

const ALPN: &[u8] = b"jetstream-session-example";

fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    let data = std::fs::read(Path::new(path)).expect("failed to read cert");
    rustls_pemfile::certs(&mut &*data)
        .filter_map(|cert| cert.ok())
        .collect()
}

fn load_key(path: &str) -> PrivateKeyDer<'static> {
    let data = std::fs::read(Path::new(path)).expect("failed to read key");
    rustls_pemfile::private_key(&mut &*data)
        .expect("failed to parse key")
        .expect("no key in file")
}

/// A server endpoint that requires a client certificate.
fn server_endpoint() -> (quinn::Endpoint, SocketAddr) {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(load_certs(CA_CERT_PEM).pop().unwrap()).unwrap();
    let verifier =
        rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .unwrap();

    let mut tls = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(load_certs(SERVER_CERT_PEM), load_key(SERVER_KEY_PEM))
        .unwrap();
    tls.alpn_protocols = vec![ALPN.to_vec()];

    let config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls).unwrap(),
    ));
    let endpoint =
        quinn::Endpoint::server(config, "127.0.0.1:0".parse().unwrap())
            .unwrap();
    let addr = endpoint.local_addr().unwrap();
    (endpoint, addr)
}

/// A client endpoint that presents the client certificate.
fn client_endpoint() -> quinn::Endpoint {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(load_certs(CA_CERT_PEM).pop().unwrap()).unwrap();
    let mut tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(
            load_certs(CLIENT_CERT_PEM),
            load_key(CLIENT_KEY_PEM),
        )
        .unwrap();
    tls.alpn_protocols = vec![ALPN.to_vec()];

    let mut endpoint =
        quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls).unwrap(),
    )));
    endpoint
}

#[tokio::main]
async fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    let (server_endpoint, addr) = server_endpoint();
    let client_endpoint = client_endpoint();

    let accepting = tokio::spawn(async move {
        let connection = server_endpoint.accept().await.unwrap().await.unwrap();
        // `new_owned` hands the endpoint to the session: dropping the
        // last handle on a quinn `Endpoint` closes every connection
        // opened from it, so the session is the right owner.
        QuicSession::<SleepyChannel>::new_owned(connection, server_endpoint)
    });

    let connection = client_endpoint
        .connect(addr, "localhost")
        .unwrap()
        .await
        .unwrap();
    let client =
        QuicSession::<SleepyChannel>::new_owned(connection, client_endpoint);
    let server = accepting.await.unwrap();

    // QUIC's row: many lanes, datagrams, a certificate for identity, and
    // survival across a change of network path. Datagram support is read
    // from the live connection rather than from the row, so this is what
    // the two peers actually negotiated.
    let caps = Session::<SleepyChannel>::capabilities(&client);
    assert_eq!(caps, Capabilities::quic());
    println!(
        "quic session: lanes={} datagrams={} identity={} migration={}",
        caps.lanes, caps.datagrams, caps.identity, caps.migration
    );

    // A certificate says who answered, not where to find them, so a
    // caller still has to carry the address it dialled.
    assert!(caps.identity.requires_address());
    match Session::<SleepyChannel>::context(&server).peer() {
        Some(Peer::Tls(tls)) => {
            let leaf = tls.leaf().expect("the client presented a chain");
            println!("the server authenticated {}", leaf.fingerprint);
        }
        other => panic!("expected a TLS peer, got {other:?}"),
    }

    // Serve every lane the peer opens, one task each.
    let serving = tokio::spawn({
        let server = server.clone();
        async move {
            while let Ok(lane) =
                Session::<SleepyChannel>::accept_lane(&server).await
            {
                tokio::spawn(async move {
                    let mut service = SleepyService {
                        inner: SleepyServer,
                    };
                    let _ =
                        jetstream_rpc::server::run(&mut service, lane).await;
                });
            }
        }
    });

    // Each lane is its own QUIC bidirectional stream, so the slow call on
    // the first does not hold up the others. Before sessions the iroh and
    // QUIC clients opened one stream at connect and recovered concurrency
    // by tag, which shares one ordered sequence between every call.
    let mut calls = Vec::new();
    for (lane_no, nap_ms) in [200u32, 20, 20].into_iter().enumerate() {
        let lane = Session::<SleepyChannel>::open_lane(&client).await.unwrap();
        let channel = SleepyChannel::new(4, Box::new(lane));
        calls.push(tokio::spawn(async move {
            let started = std::time::Instant::now();
            let answer = channel.nap(Context::default(), nap_ms).await.unwrap();
            (lane_no, answer, started.elapsed())
        }));
    }
    for call in calls {
        let (lane_no, answer, took) = call.await.unwrap();
        println!("lane {lane_no}: {answer} (round trip {took:?})");
    }

    // The datagram channel belongs to the session, not to any lane. It
    // is unordered and unreliable by construction, so it is deliberately
    // not a lane and none of the ordering guarantees apply to it.
    caps.require(Capability::Datagrams).unwrap();
    let limit = Datagrams::<SleepyChannel>::max_datagram_size(&client)
        .expect("the peer advertised datagram support");
    println!("datagrams up to {limit} bytes");

    // Sending names the direction: a session is not fixed to a role, so
    // the end acting as caller sends requests and the end acting as
    // callee sends responses.
    client
        .send_request_datagram(Frame {
            tag: 1,
            msg: Tmessage::Nap(Tnap { ms: 0 }),
        })
        .await
        .unwrap();

    let bytes = timeout(Duration::from_secs(5), server.recv_datagram_bytes())
        .await
        .expect("the datagram should have arrived")
        .unwrap();
    let got: Frame<Tmessage> = decode_datagram(bytes).unwrap();
    println!("datagram arrived out of band: tag {}", got.tag);

    // Closing the session ends every lane on it; work in flight fails
    // rather than hanging.
    Session::<SleepyChannel>::close(&client).await;
    serving.abort();
}
