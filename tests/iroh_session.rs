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
        decode_datagram, Capabilities, Capability, Datagrams, IdentityKind,
        LaneSupport, Session, SessionError,
    },
    Frame,
};
use jetstream_wireformat::WireFormat;
use session_common::{frame, response, Ask, Say, TestProtocol};
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
    assert_eq!(caps.identity, IdentityKind::Key);
    assert!(caps.migration);

    // r[impl jetstream.session.capabilities.degradation]
    // Datagrams are reported from the connection rather than from the
    // row, so this asserts the peers actually negotiated them rather
    // than that iroh supports them in principle.
    assert!(caps.datagrams);
    assert!(
        Datagrams::<TestProtocol>::max_datagram_size(&pair.client).is_some(),
        "the reported capability should match the connection"
    );

    // r[impl jetstream.session.capabilities]
    // `max_datagram_size` reads the *peer's* advertised limit, so it
    // cannot see this endpoint's own configuration. A caller that
    // switched datagrams off locally says so, and both the capability
    // and the size follow — otherwise the size would keep saying yes
    // while every send failed.
    let quiet = pair.client.clone().without_datagrams();
    assert!(!Session::<TestProtocol>::capabilities(&quiet).datagrams);
    assert!(Datagrams::<TestProtocol>::max_datagram_size(&quiet).is_none());
    assert!(matches!(
        Session::<TestProtocol>::capabilities(&quiet)
            .require(Capability::Datagrams),
        Err(SessionError::Unsupported(Capability::Datagrams))
    ));

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
    served_second.send(response(2, "answered")).await.unwrap();
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

// Regression: `IrohSession` held the connection directly, so dropping
// the last handle left the QUIC association up — every lane carries its
// own clone of the connection, and those clones kept it alive. The local
// and single-lane sessions got drop paths a round earlier; this one was
// reported as fixed when it was not.
// r[impl jetstream.session.lifetime]
#[tokio::test(flavor = "multi_thread")]
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

    // The last one is. Note the lane is deliberately still alive: it
    // holds its own clone of the connection, which is what used to keep
    // the association up after its session had gone.
    let Pair { client, .. } = pair;
    drop(client);

    let ended = timeout(Duration::from_secs(10), lane.next())
        .await
        .expect("the lane should have terminated, not hung");
    assert!(
        matches!(ended, Some(Err(_))),
        "a lane whose session was dropped should fail, got {ended:?}"
    );
}

// r[impl jetstream.session.datagrams]
#[tokio::test(flavor = "multi_thread")]
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
        timeout(Duration::from_secs(10), pair.server.recv_datagram_bytes())
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
        timeout(Duration::from_secs(10), pair.client.recv_datagram_bytes())
            .await
            .expect("the response datagram should have arrived")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(got.tag, 7);
    assert_eq!(got.msg.as_str(), "answered");
}

// r[impl jetstream.session.datagrams]
#[tokio::test(flavor = "multi_thread")]
async fn an_oversized_datagram_is_rejected_at_the_sender() {
    let pair = pair().await;

    let limit = Datagrams::<TestProtocol>::max_datagram_size(&pair.client)
        .expect("iroh should report a datagram limit");

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
#[tokio::test(flavor = "multi_thread")]
async fn a_datagram_with_trailing_bytes_is_rejected() {
    let pair = pair().await;

    // A frame, then a byte that is not part of it. Decoding the prefix
    // would succeed and hand back something the peer never sent.
    let mut buf = Vec::new();
    WireFormat::encode(&frame(3, "whole"), &mut buf).unwrap();
    buf.push(0xff);

    pair.client.connection().send_datagram(buf.into()).unwrap();

    let bytes =
        timeout(Duration::from_secs(10), pair.server.recv_datagram_bytes())
            .await
            .expect("the datagram should have arrived")
            .unwrap();
    let got = decode_datagram::<Ask>(bytes);
    assert!(
        got.is_err(),
        "a datagram with trailing bytes is not a complete frame"
    );
}
