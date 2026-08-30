//! The QUIC session binding, over a real handshake.
//!
//! r[impl jetstream.session.conformance]
//! `jetstream_quic` claims a row of the conformance table; these tests
//! hold it to the row rather than to the shape of its types.

#![cfg(feature = "quic")]

mod session_common;

use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};

use futures::{SinkExt, StreamExt};
use jetstream_quic::QuicSession;
use jetstream_rpc::{
    context::{Contextual, Peer, RemoteAddr},
    session::{
        Capabilities, Datagrams, IdentityKind, LaneSupport, Session,
        SessionError,
    },
    Frame,
};
use jetstream_wireformat::WireFormat;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use session_common::{frame, Blob, TestProtocol};
use tokio::time::timeout;

static CA_CERT_PEM: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/certs/ca.pem");
static CLIENT_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client.pem");
static CLIENT_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/client.key");
static SERVER_CERT_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.pem");
static SERVER_KEY_PEM: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/certs/server.key");

const ALPN: &[u8] = b"jetstream-session-test";

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

/// A connected pair of sessions over one mTLS QUIC connection.
///
/// The endpoints are returned alongside so that they outlive the
/// connection: dropping a quinn `Endpoint` tears down everything opened
/// from it.
struct Pair {
    client: QuicSession<TestProtocol>,
    server: QuicSession<TestProtocol>,
    _client_endpoint: quinn::Endpoint,
    _server_endpoint: quinn::Endpoint,
}

async fn pair() -> Pair {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    let ca_cert = load_certs(CA_CERT_PEM).pop().unwrap();

    let mut roots = rustls::RootCertStore::empty();
    roots.add(ca_cert.clone()).unwrap();
    let client_verifier =
        rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .unwrap();

    let mut server_tls = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(load_certs(SERVER_CERT_PEM), load_key(SERVER_KEY_PEM))
        .unwrap();
    server_tls.alpn_protocols = vec![ALPN.to_vec()];

    let server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_tls).unwrap(),
    ));
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server_endpoint = quinn::Endpoint::server(server_config, bind).unwrap();
    let server_addr = server_endpoint.local_addr().unwrap();

    let mut roots = rustls::RootCertStore::empty();
    roots.add(ca_cert).unwrap();
    let mut client_tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(
            load_certs(CLIENT_CERT_PEM),
            load_key(CLIENT_KEY_PEM),
        )
        .unwrap();
    client_tls.alpn_protocols = vec![ALPN.to_vec()];

    let mut client_endpoint = quinn::Endpoint::client(bind).unwrap();
    client_endpoint.set_default_client_config(quinn::ClientConfig::new(
        Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_tls)
                .unwrap(),
        ),
    ));

    let accepting = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.unwrap();
        let connection = incoming.await.unwrap();
        (server_endpoint, connection)
    });

    let client_connection = client_endpoint
        .connect(server_addr, "localhost")
        .unwrap()
        .await
        .unwrap();
    let (server_endpoint, server_connection) = accepting.await.unwrap();

    Pair {
        client: QuicSession::new(client_connection),
        server: QuicSession::new(server_connection),
        _client_endpoint: client_endpoint,
        _server_endpoint: server_endpoint,
    }
}

// r[impl jetstream.session.capabilities]
#[tokio::test]
async fn a_quic_session_reports_its_row() {
    let pair = pair().await;
    let caps = Session::<TestProtocol>::capabilities(&pair.client);

    assert_eq!(caps, Capabilities::quic());
    assert_eq!(caps.lanes, LaneSupport::Many);
    assert!(caps.datagrams);
    assert_eq!(caps.identity, IdentityKind::Certificate);
    assert!(caps.migration);

    // r[impl jetstream.session.identity.addressing]
    // A certificate says who answered, not where to find them.
    assert!(caps.identity.requires_address());
}

// r[impl jetstream.lane.independence]
#[tokio::test]
async fn each_lane_is_its_own_stream() {
    let pair = pair().await;

    let mut first = pair.client.open_lane().await.unwrap();
    let mut second = pair.client.open_lane().await.unwrap();

    // A QUIC stream reaches the peer on its first write, so both lanes
    // are announced before either is accepted.
    first.send(frame(1, "first")).await.unwrap();
    second.send(frame(2, "second")).await.unwrap();

    let mut served_first = pair.server.accept_lane().await.unwrap();
    let mut served_second = pair.server.accept_lane().await.unwrap();

    let got = served_first.next().await.unwrap().unwrap();
    assert_eq!(got.msg.as_str(), "first");
    let got = served_second.next().await.unwrap().unwrap();
    assert_eq!(got.msg.as_str(), "second");

    // Neither lane has anything of the other's on it.
    let quiet = timeout(Duration::from_millis(50), served_first.next()).await;
    assert!(quiet.is_err(), "the first lane should be waiting");

    // r[impl jetstream.session.symmetric]
    // The callee answers on the lane the caller opened.
    served_second.send(frame(2, "answered")).await.unwrap();
    let got = second.next().await.unwrap().unwrap();
    assert_eq!(got.msg.as_str(), "answered");
}

// r[impl jetstream.session.identity]
#[tokio::test]
async fn a_session_reports_the_peer_the_handshake_authenticated() {
    let pair = pair().await;

    let server_view = Session::<TestProtocol>::context(&pair.server);
    match server_view.peer() {
        Some(Peer::Tls(tls)) => {
            let leaf = tls.leaf().expect("client chain has a leaf");
            assert!(
                !leaf.fingerprint.is_empty(),
                "the leaf should have been parsed, got {leaf:?}"
            );
        }
        other => panic!("expected a TLS peer, got {other:?}"),
    }

    // r[impl jetstream.session.identity.addressing]
    assert!(matches!(server_view.remote(), Some(RemoteAddr::IpAddr(_))));

    // A lane carries the session's identity, so a handler reading only
    // the lane sees the same peer.
    let mut lane = pair.client.open_lane().await.unwrap();
    lane.send(frame(1, "hello")).await.unwrap();
    let served = pair.server.accept_lane().await.unwrap();
    assert_eq!(Contextual::context(&served), server_view);
}

// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn closing_a_session_terminates_its_lanes() {
    let pair = pair().await;

    let mut lane = pair.client.open_lane().await.unwrap();
    lane.send(frame(1, "hello")).await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();
    assert_eq!(served.next().await.unwrap().unwrap().msg.as_str(), "hello");

    Session::<TestProtocol>::close(&pair.server).await;

    // The lane fails rather than hanging: a caller parked on a reply
    // gets an error, not silence.
    let ended = timeout(Duration::from_secs(5), lane.next())
        .await
        .expect("the lane should have terminated, not hung");
    assert!(
        matches!(ended, Some(Err(_))),
        "a lane on a closed session should fail, got {ended:?}"
    );

    // ...and so does the next lane open.
    let opened = timeout(Duration::from_secs(5), pair.client.open_lane())
        .await
        .expect("opening on a closed session should not hang");
    assert!(opened.is_err(), "a closed session should not open a lane");
}

// r[impl jetstream.session.datagrams]
#[tokio::test]
async fn a_datagram_round_trips() {
    let pair = pair().await;

    pair.client
        .send_datagram(frame(7, "unordered"))
        .await
        .unwrap();

    let got = timeout(Duration::from_secs(5), pair.server.recv_datagram())
        .await
        .expect("the datagram should have arrived")
        .unwrap();
    assert_eq!(got.tag, 7);
    assert_eq!(got.msg.as_str(), "unordered");
}

// r[impl jetstream.session.datagrams]
#[tokio::test]
async fn an_oversized_datagram_is_rejected_at_the_sender() {
    let pair = pair().await;

    let limit = Datagrams::<TestProtocol>::max_datagram_size(&pair.client)
        .expect("QUIC should report a datagram limit");

    let too_big = Frame {
        tag: 1,
        msg: Blob::of(limit as usize + 1),
    };
    assert!(WireFormat::byte_size(&too_big) > limit);

    match pair.client.send_datagram(too_big).await {
        Err(SessionError::DatagramTooLarge { limit: got, .. }) => {
            assert_eq!(got, limit)
        }
        other => panic!("expected DatagramTooLarge, got {other:?}"),
    }
}

// r[impl jetstream.session.datagrams]
#[tokio::test]
async fn a_datagram_with_trailing_bytes_is_rejected() {
    let pair = pair().await;

    // A frame, then a byte that is not part of it. Decoding the prefix
    // would succeed and hand back something the peer never sent.
    let mut buf = Vec::new();
    WireFormat::encode(&frame(3, "whole"), &mut buf).unwrap();
    buf.push(0xff);

    pair.client.connection().send_datagram(buf.into()).unwrap();

    let got = timeout(Duration::from_secs(5), pair.server.recv_datagram())
        .await
        .expect("the datagram should have arrived");
    assert!(
        got.is_err(),
        "a datagram with trailing bytes is not a complete frame"
    );
}
