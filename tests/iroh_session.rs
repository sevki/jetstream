//! The iroh session binding, over a real handshake.
//!
//! r[impl jetstream.session.conformance.iroh]
//! The tests run against a local relay server, so nothing here reaches
//! the public relay network.

#![cfg(feature = "iroh")]

mod session_common;

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use iroh::{endpoint::presets, tls::CaTlsConfig, Endpoint, RelayMode};
use jetstream_iroh::IrohSession;
use jetstream_rpc::{
    context::{Contextual, Peer},
    session::{
        Capabilities, Datagrams, IdentityKind, LaneSupport, Session,
        SessionError,
    },
    Frame,
};
use jetstream_wireformat::WireFormat;
use session_common::{frame, Blob, TestProtocol};
use tokio::time::timeout;

const ALPN: &[u8] = b"jetstream-session-test";

/// A connected pair of sessions over one iroh connection, plus the
/// relay the two endpoints found each other through.
struct Pair {
    client: IrohSession<TestProtocol>,
    server: IrohSession<TestProtocol>,
    client_id: String,
    server_id: String,
    // The relay stays up for as long as the pair does. Its type is
    // not re-exported from `iroh`, so it is held opaquely.
    _relay: Box<dyn std::any::Any + Send>,
}

async fn pair() -> Pair {
    let (relay_map, _relay_url, relay) =
        iroh::test_utils::run_relay_server().await.unwrap();

    let server_endpoint = Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Custom(relay_map.clone()))
        .ca_tls_config(CaTlsConfig::insecure_skip_verify())
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    server_endpoint.online().await;
    let server_addr = server_endpoint.addr();
    let server_id = server_endpoint.id().to_string();

    let client_endpoint = Endpoint::builder(presets::Minimal)
        .relay_mode(RelayMode::Custom(relay_map))
        .ca_tls_config(CaTlsConfig::insecure_skip_verify())
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    let client_id = client_endpoint.id().to_string();

    let accepting = {
        let server_endpoint = server_endpoint.clone();
        tokio::spawn(async move {
            server_endpoint.accept().await.unwrap().await.unwrap()
        })
    };

    let client_connection =
        client_endpoint.connect(server_addr, ALPN).await.unwrap();
    let server_connection = accepting.await.unwrap();

    Pair {
        client: IrohSession::new_owned(client_connection, client_endpoint),
        server: IrohSession::new_owned(server_connection, server_endpoint),
        client_id,
        server_id,
        _relay: Box::new(relay),
    }
}

// r[impl jetstream.session.capabilities]
#[tokio::test(flavor = "multi_thread")]
async fn an_iroh_session_reports_its_row() {
    let pair = pair().await;
    let caps = Session::<TestProtocol>::capabilities(&pair.client);

    assert_eq!(caps, Capabilities::iroh());
    assert_eq!(caps.lanes, LaneSupport::Many);
    assert!(caps.datagrams);
    assert_eq!(caps.identity, IdentityKind::Key);
    assert!(caps.migration);

    // r[impl jetstream.session.identity.addressing]
    // The key is the address, so a caller never has to carry placement
    // information alongside it.
    assert!(!caps.identity.requires_address());
}

// r[impl jetstream.lane.independence]
#[tokio::test(flavor = "multi_thread")]
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

    let quiet = timeout(Duration::from_millis(50), served_first.next()).await;
    assert!(quiet.is_err(), "the first lane should be waiting");

    // r[impl jetstream.session.symmetric]
    served_second.send(frame(2, "answered")).await.unwrap();
    let got = second.next().await.unwrap().unwrap();
    assert_eq!(got.msg.as_str(), "answered");
}

// r[impl jetstream.session.identity]
#[tokio::test(flavor = "multi_thread")]
async fn both_ends_of_a_session_know_the_other() {
    let pair = pair().await;

    // iroh authenticates both peers during the handshake, so unlike a
    // one-sided TLS connection the caller is named too.
    let server_view = Session::<TestProtocol>::context(&pair.server);
    let client_view = Session::<TestProtocol>::context(&pair.client);

    assert!(matches!(server_view.peer(), Some(Peer::NodeId(_))));
    assert!(matches!(client_view.peer(), Some(Peer::NodeId(_))));
    assert_eq!(server_view.to_string(), pair.client_id);
    assert_eq!(client_view.to_string(), pair.server_id);

    // A lane carries the session's identity, so a handler reading only
    // the lane sees the same peer.
    let mut lane = pair.client.open_lane().await.unwrap();
    lane.send(frame(1, "hello")).await.unwrap();
    let served = pair.server.accept_lane().await.unwrap();
    assert_eq!(Contextual::context(&served), server_view);
}

// r[impl jetstream.session.lifetime]
#[tokio::test(flavor = "multi_thread")]
async fn closing_a_session_terminates_its_lanes() {
    let pair = pair().await;

    let mut lane = pair.client.open_lane().await.unwrap();
    lane.send(frame(1, "hello")).await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();
    assert_eq!(served.next().await.unwrap().unwrap().msg.as_str(), "hello");

    Session::<TestProtocol>::close(&pair.server).await;

    let ended = timeout(Duration::from_secs(10), lane.next())
        .await
        .expect("the lane should have terminated, not hung");
    assert!(
        matches!(ended, Some(Err(_))),
        "a lane on a closed session should fail, got {ended:?}"
    );

    let opened = timeout(Duration::from_secs(10), pair.client.open_lane())
        .await
        .expect("opening on a closed session should not hang");
    assert!(opened.is_err(), "a closed session should not open a lane");
}

// r[impl jetstream.session.datagrams]
#[tokio::test(flavor = "multi_thread")]
async fn a_datagram_round_trips() {
    let pair = pair().await;

    pair.client
        .send_datagram(frame(7, "unordered"))
        .await
        .unwrap();

    let got = timeout(Duration::from_secs(10), pair.server.recv_datagram())
        .await
        .expect("the datagram should have arrived")
        .unwrap();
    assert_eq!(got.tag, 7);
    assert_eq!(got.msg.as_str(), "unordered");
}

// r[impl jetstream.session.datagrams]
#[tokio::test(flavor = "multi_thread")]
async fn an_oversized_datagram_is_rejected_at_the_sender() {
    let pair = pair().await;

    let limit = Datagrams::<TestProtocol>::max_datagram_size(&pair.client)
        .expect("iroh should report a datagram limit");

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
#[tokio::test(flavor = "multi_thread")]
async fn a_datagram_with_trailing_bytes_is_rejected() {
    let pair = pair().await;

    // A frame, then a byte that is not part of it. Decoding the prefix
    // would succeed and hand back something the peer never sent.
    let mut buf = Vec::new();
    WireFormat::encode(&frame(3, "whole"), &mut buf).unwrap();
    buf.push(0xff);

    pair.client.connection().send_datagram(buf.into()).unwrap();

    let got = timeout(Duration::from_secs(10), pair.server.recv_datagram())
        .await
        .expect("the datagram should have arrived");
    assert!(
        got.is_err(),
        "a datagram with trailing bytes is not a complete frame"
    );
}
