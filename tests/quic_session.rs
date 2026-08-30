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
        decode_datagram, Capabilities, Capability, Datagrams, IdentityKind,
        LaneSupport, Session, SessionError,
    },
    Frame,
};
use jetstream_wireformat::WireFormat;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use session_common::{frame, response, Ask, Say, TestProtocol};
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
/// Each session owns its endpoint, since dropping the last handle on a
/// quinn `Endpoint` closes every connection opened from it. That is what
/// `new_owned` is for; nothing outside the session needs to hold them.
struct Pair {
    client: QuicSession<TestProtocol>,
    server: QuicSession<TestProtocol>,
}

/// A bound server endpoint, and the address to reach it on.
fn server_endpoint() -> (quinn::Endpoint, SocketAddr) {
    server_endpoint_with(true)
}

/// As above, but able to refuse datagrams — a peer that does not
/// advertise DATAGRAM support leaves the other end's
/// `max_datagram_size` empty.
fn server_endpoint_with(datagrams: bool) -> (quinn::Endpoint, SocketAddr) {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    let mut roots = rustls::RootCertStore::empty();
    roots.add(load_certs(CA_CERT_PEM).pop().unwrap()).unwrap();
    let client_verifier =
        rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .unwrap();

    let mut tls = rustls::ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(load_certs(SERVER_CERT_PEM), load_key(SERVER_KEY_PEM))
        .unwrap();
    tls.alpn_protocols = vec![ALPN.to_vec()];

    let mut config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls).unwrap(),
    ));
    if !datagrams {
        let mut transport = quinn::TransportConfig::default();
        transport.datagram_receive_buffer_size(None);
        config.transport_config(Arc::new(transport));
    }
    let endpoint =
        quinn::Endpoint::server(config, "127.0.0.1:0".parse().unwrap())
            .unwrap();
    let addr = endpoint.local_addr().unwrap();
    (endpoint, addr)
}

/// A client endpoint that presents the test client certificate.
fn client_endpoint() -> quinn::Endpoint {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

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

async fn pair() -> Pair {
    let (server, server_addr) = server_endpoint();
    let client = client_endpoint();

    let accepting = tokio::spawn(async move {
        let connection = server.accept().await.unwrap().await.unwrap();
        (server, connection)
    });

    let client_connection = client
        .connect(server_addr, "localhost")
        .unwrap()
        .await
        .unwrap();
    let (server, server_connection) = accepting.await.unwrap();

    Pair {
        client: QuicSession::new_owned(client_connection, client),
        server: QuicSession::new_owned(server_connection, server),
    }
}

// r[impl jetstream.session.capabilities]
#[tokio::test]
async fn a_quic_session_reports_its_row() {
    let pair = pair().await;
    let caps = Session::<TestProtocol>::capabilities(&pair.client);

    assert_eq!(caps, Capabilities::quic());
    assert_eq!(caps.lanes, LaneSupport::Many);
    assert_eq!(caps.identity, IdentityKind::Certificate);
    assert!(caps.migration);

    // r[impl jetstream.session.capabilities.degradation]
    // Datagrams are reported from the connection rather than from the
    // row, so this asserts the peers actually negotiated them.
    assert!(caps.datagrams);
    assert!(
        Datagrams::<TestProtocol>::max_datagram_size(&pair.client).is_some(),
        "the reported capability should match the connection"
    );

    // r[impl jetstream.session.identity.addressing]
    // A certificate says who answered, not where to find them.
    assert!(caps.identity.requires_address());
}

// Regression: `capabilities()` returned the conformance row whole, so
// `datagrams` was true even on a connection that cannot carry one, and
// `require(Capability::Datagrams)` succeeded where every send would
// fail. The row is what QUIC can do; a capability is what this session
// has.
// r[impl jetstream.session.capabilities.degradation]
#[tokio::test]
async fn a_session_whose_peer_refuses_datagrams_says_so() {
    let (server, server_addr) = server_endpoint_with(false);
    let client = client_endpoint();

    let accepting = tokio::spawn(async move {
        let connection = server.accept().await.unwrap().await.unwrap();
        QuicSession::<TestProtocol>::new_owned(connection, server)
    });
    let connection = client
        .connect(server_addr, "localhost")
        .unwrap()
        .await
        .unwrap();
    let session = QuicSession::<TestProtocol>::new_owned(connection, client);
    let _served = accepting.await.unwrap();

    let caps = Session::<TestProtocol>::capabilities(&session);
    assert!(
        Datagrams::<TestProtocol>::max_datagram_size(&session).is_none(),
        "the peer should not have advertised datagram support"
    );
    assert!(
        !caps.datagrams,
        "a session that cannot carry a datagram must not claim it can"
    );
    assert!(matches!(
        caps.require(Capability::Datagrams),
        Err(SessionError::Unsupported(Capability::Datagrams))
    ));

    // The rest of the row is unaffected: this is still a QUIC session.
    assert_eq!(caps.lanes, LaneSupport::Many);
    assert_eq!(caps.identity, IdentityKind::Certificate);
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
    served_second.send(response(2, "answered")).await.unwrap();
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

// What `new_owned` is and is not for.
//
// The review finding this answers said a session built inside a helper
// that owned its endpoint would be dead on return, because dropping the
// last `quinn::Endpoint` handle closes every connection made from it.
// That is true of iroh, which is why `IrohTransport` holds an endpoint
// clone — but it is **not** true of quinn. Its endpoint driver finishes
// only when the handle count reaches zero *and* it has no connections
// left (`EndpointDriver::poll` in quinn 0.11), so a live `Connection`
// keeps the driver running on its own.
//
// So this test passes with or without `new_owned` retaining anything —
// checked, by making `new_owned` drop the endpoint on the floor. It is
// here to pin the quinn behaviour the API decision rests on, not as a
// regression test, and it is named for what it actually asserts.
// `new_owned` stays as a convenience that mirrors the iroh binding and
// spares a caller the question.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn a_quic_connection_survives_its_endpoint_handles() {
    let (server, server_addr) = server_endpoint();

    let accepting = tokio::spawn(async move {
        let connection = server.accept().await.unwrap().await.unwrap();
        QuicSession::<TestProtocol>::new_owned(connection, server)
    });

    // A helper that builds its own client and hands back a session. The
    // endpoint goes out of scope on the way out.
    async fn dial(addr: SocketAddr) -> QuicSession<TestProtocol> {
        let endpoint = client_endpoint();
        let connection =
            endpoint.connect(addr, "localhost").unwrap().await.unwrap();
        QuicSession::new_owned(connection, endpoint)
    }

    let session = dial(server_addr).await;
    let served = accepting.await.unwrap();

    let mut lane = session.open_lane().await.unwrap();
    lane.send(frame(1, "still connected")).await.unwrap();

    let mut served_lane = timeout(Duration::from_secs(5), served.accept_lane())
        .await
        .expect("the peer should still be reachable")
        .unwrap();
    assert_eq!(
        served_lane.next().await.unwrap().unwrap().msg.as_str(),
        "still connected"
    );
}

// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn dropping_the_last_session_handle_closes_the_connection() {
    let pair = pair().await;

    let mut lane = pair.client.open_lane().await.unwrap();
    lane.send(frame(1, "hello")).await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();
    assert_eq!(served.next().await.unwrap().unwrap().msg.as_str(), "hello");

    // A clone is not the last handle, so the association survives it.
    drop(pair.client.clone());
    served.send(response(1, "still here")).await.unwrap();
    assert_eq!(
        lane.next().await.unwrap().unwrap().msg.as_str(),
        "still here"
    );

    // The last one is. The lane is deliberately still alive: it holds
    // streams opened from the connection.
    let Pair { client, .. } = pair;
    drop(client);

    let ended = timeout(Duration::from_secs(5), lane.next())
        .await
        .expect("the lane should have terminated, not hung");
    assert!(
        matches!(ended, Some(Err(_))),
        "a lane whose session was dropped should fail, got {ended:?}"
    );
}

// r[impl jetstream.session.datagrams]
#[tokio::test]
async fn a_datagram_round_trips() {
    let pair = pair().await;

    // r[impl jetstream.session.symmetric]
    // The caller sends a request and the callee receives a request. The
    // API named the roles the other way round until this test's protocol
    // stopped sharing one type between them.
    pair.client
        .send_request_datagram(frame(7, "asked"))
        .await
        .unwrap();

    let got: Frame<Ask> = decode_datagram(
        timeout(Duration::from_secs(5), pair.server.recv_datagram_bytes())
            .await
            .expect("the request datagram should have arrived")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(got.tag, 7);
    assert_eq!(got.msg.as_str(), "asked");

    // ...and back the other way, which is the direction a session that
    // decoded by the wrong role would have got right by accident.
    pair.server
        .send_response_datagram(response(7, "answered"))
        .await
        .unwrap();

    let got: Frame<Say> = decode_datagram(
        timeout(Duration::from_secs(5), pair.client.recv_datagram_bytes())
            .await
            .expect("the response datagram should have arrived")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(got.tag, 7);
    assert_eq!(got.msg.as_str(), "answered");
}

// r[impl jetstream.session.datagrams]
#[tokio::test]
async fn an_oversized_datagram_is_rejected_at_the_sender() {
    let pair = pair().await;

    let limit = Datagrams::<TestProtocol>::max_datagram_size(&pair.client)
        .expect("QUIC should report a datagram limit");

    let too_big = Frame {
        tag: 1,
        msg: Ask::of(limit as usize + 1),
    };
    assert!(WireFormat::byte_size(&too_big) > limit);

    match pair.client.send_request_datagram(too_big).await {
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

    let bytes =
        timeout(Duration::from_secs(5), pair.server.recv_datagram_bytes())
            .await
            .expect("the datagram should have arrived")
            .unwrap();
    let got = decode_datagram::<Ask>(bytes);
    assert!(
        got.is_err(),
        "a datagram with trailing bytes is not a complete frame"
    );
}
