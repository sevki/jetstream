use std::{
    io::{self, Read, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{SinkExt, StreamExt};
use tokio::time::timeout;

use crate::{
    session::{
        check_datagram_size, Capabilities, Capability, IdentityKind, LaneOrder,
        LaneSupport, LocalSession, Session, SessionError, SingleLaneSession,
    },
    Error, Frame, Framer, Protocol,
};

#[derive(Debug, PartialEq, Eq)]
struct Ping(u32);

impl Framer for Ping {
    fn message_type(&self) -> u8 {
        1
    }

    fn byte_size(&self) -> u32 {
        4
    }

    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0.to_le_bytes())
    }

    fn decode<R: Read>(reader: &mut R, _ty: u8) -> io::Result<Self> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(Ping(u32::from_le_bytes(buf)))
    }
}

#[derive(Debug)]
struct TestProtocol;

impl Protocol for TestProtocol {
    type Error = Error;
    type Request = Ping;
    type Response = Ping;

    const NAME: &'static str = "test";
    const VERSION: &'static str = "dev";
}

fn frame(n: u32) -> Frame<Ping> {
    Frame {
        tag: n as u16,
        msg: Ping(n),
    }
}

// r[impl jetstream.session.local]
#[tokio::test]
async fn in_process_session_reports_its_row() {
    let pair = LocalSession::<TestProtocol>::pair();
    let caps = pair.client.capabilities();
    assert_eq!(caps.lanes, LaneSupport::Many);
    assert!(!caps.datagrams);
    assert_eq!(caps.identity, IdentityKind::None);
    assert!(!caps.migration);
}

// r[impl jetstream.lane.delivery-order]
#[tokio::test]
async fn a_lane_delivers_in_write_order() {
    let pair = LocalSession::<TestProtocol>::pair();
    let mut lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();

    for n in 0..8 {
        lane.send(frame(n)).await.unwrap();
    }

    for n in 0..8 {
        let got = served.next().await.unwrap().unwrap();
        assert_eq!(got.msg, Ping(n));
    }
}

// r[impl jetstream.lane.independence]
#[tokio::test]
async fn a_stalled_lane_does_not_block_another() {
    let pair = LocalSession::<TestProtocol>::pair();
    let mut quiet = pair.client.open_lane().await.unwrap();
    let mut busy = pair.client.open_lane().await.unwrap();
    let mut served_quiet = pair.server.accept_lane().await.unwrap();
    let mut served_busy = pair.server.accept_lane().await.unwrap();

    // Nothing is written to the first lane at all.
    busy.send(frame(7)).await.unwrap();
    let got = served_busy.next().await.unwrap().unwrap();
    assert_eq!(got.msg, Ping(7));

    // ...and the second lane still carries nothing, rather than the
    // first lane's silence having queued behind it.
    let idle = timeout(Duration::from_millis(50), served_quiet.next()).await;
    assert!(idle.is_err(), "quiet lane should still be waiting");

    quiet.send(frame(1)).await.unwrap();
    let got = served_quiet.next().await.unwrap().unwrap();
    assert_eq!(got.msg, Ping(1));
}

// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn closing_a_session_terminates_its_lanes() {
    let pair = LocalSession::<TestProtocol>::pair();
    let mut lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();

    pair.client.close().await;

    // The call in flight fails rather than hanging.
    let err = timeout(Duration::from_millis(200), lane.next())
        .await
        .expect("closed lane must not hang")
        .expect("closed lane reports before ending")
        .expect_err("closed lane reports an error");
    assert_eq!(err.code(), Some("jetstream::session::closed"));

    assert!(lane.send(frame(1)).await.is_err());

    // Opening a new lane on a closed session fails too.
    let opened = pair.client.open_lane().await;
    assert!(matches!(opened, Err(SessionError::Closed)));

    // The peer's end of the lane is unaffected by our close: its own
    // session is still open, and it sees the lane end.
    let _ = timeout(Duration::from_millis(200), served.next()).await;
}

// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn closing_a_lane_does_not_close_its_session() {
    let pair = LocalSession::<TestProtocol>::pair();
    let lane = pair.client.open_lane().await.unwrap();
    let _served = pair.server.accept_lane().await.unwrap();
    drop(lane);

    let second = pair.client.open_lane().await;
    assert!(second.is_ok(), "session outlives one of its lanes");
}

// r[impl jetstream.session.single-lane]
#[tokio::test]
async fn a_single_lane_session_opens_once() {
    let pair = LocalSession::<TestProtocol>::pair();
    let lane = pair.client.open_lane().await.unwrap();
    let session = SingleLaneSession::<TestProtocol, _, _>::client(lane)
        .with_identity(IdentityKind::None);

    assert_eq!(session.capabilities().lanes, LaneSupport::One);
    assert!(session.open_lane().await.is_ok());

    let err = session.open_lane().await.expect_err("one lane only");
    assert!(matches!(err, SessionError::LaneLimitReached));
    assert_eq!(err.code(), "jetstream::session::lane_limit_reached");

    // ...and it is inspectable once it has become a jetstream Error.
    let as_error: Error = err.into();
    assert_eq!(
        as_error.code(),
        Some("jetstream::session::lane_limit_reached")
    );
}

// r[impl jetstream.session.single-lane]
#[tokio::test]
async fn a_client_side_single_lane_session_accepts_nothing() {
    let pair = LocalSession::<TestProtocol>::pair();
    let lane = pair.client.open_lane().await.unwrap();
    let session = SingleLaneSession::<TestProtocol, _, _>::client(lane);

    let err = session.accept_lane().await.expect_err("no accept side");
    assert!(matches!(err, SessionError::AcceptUnsupported));
}

// r[impl jetstream.session.capabilities.degradation]
#[test]
fn asking_for_a_missing_capability_fails_explicitly() {
    let byte_stream = Capabilities::byte_stream();
    let err = byte_stream
        .require(Capability::ManyLanes)
        .expect_err("a byte stream carries one lane");
    assert!(matches!(err, SessionError::Unsupported(_)));

    assert!(Capabilities::iroh().require(Capability::ManyLanes).is_ok());
    assert!(Capabilities::iroh().require(Capability::Datagrams).is_ok());
}

// r[impl jetstream.session.identity.addressing]
#[test]
fn key_identity_needs_no_address() {
    assert!(!IdentityKind::Key.requires_address());
    assert!(IdentityKind::Certificate.requires_address());
    assert!(IdentityKind::None.requires_address());
    assert!(Capabilities::iroh().supports(Capability::KeyAddressing));
    assert!(!Capabilities::quic().supports(Capability::KeyAddressing));
}

// r[impl jetstream.session.datagrams]
#[test]
fn an_oversized_datagram_is_rejected_at_the_send_site() {
    let frame = frame(1);
    assert!(check_datagram_size(&frame, None).is_ok());
    assert!(check_datagram_size(&frame, Some(1200)).is_ok());

    let err = check_datagram_size(&frame, Some(4))
        .expect_err("a frame larger than the path limit");
    assert!(matches!(err, SessionError::DatagramTooLarge { .. }));
}

// r[impl jetstream.session.local.order-handoff]
#[tokio::test(flavor = "multi_thread")]
async fn delivery_keeps_the_order_taken_at_the_call_site() {
    let order = LaneOrder::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    // Tickets are taken here, in this order...
    let tickets: Vec<_> = (0..8).map(|n| (n, order.ticket())).collect();

    // ...and admitted in the opposite one.
    let mut handles = Vec::new();
    for (n, ticket) in tickets.into_iter().rev() {
        let log = log.clone();
        handles.push(tokio::spawn(async move {
            ticket.wait().await;
            log.lock().unwrap().push(n);
            ticket.complete();
        }));
    }

    for handle in handles {
        timeout(Duration::from_secs(5), handle)
            .await
            .expect("ordered delivery must not deadlock")
            .unwrap();
    }

    let delivered = log.lock().unwrap().clone();
    assert_eq!(delivered, (0..8).collect::<Vec<_>>());
}

// r[impl jetstream.session.local.order-handoff]
#[tokio::test]
async fn an_abandoned_frame_passes_its_place_on() {
    let order = LaneOrder::new();
    let first = order.ticket();
    let second = order.ticket();
    let third = order.ticket();

    // The first frame is abandoned before delivery: its place goes to
    // the next frame on the lane rather than being released.
    drop(first);

    timeout(Duration::from_secs(5), second.wait())
        .await
        .expect("the abandoned place must pass on");
    second.complete();

    timeout(Duration::from_secs(5), third.wait())
        .await
        .expect("delivery continues in order");
    third.complete();
}

// r[impl jetstream.session.local.order-handoff]
#[tokio::test]
async fn a_place_abandoned_out_of_turn_is_skipped() {
    let order = LaneOrder::new();
    let first = order.ticket();
    let second = order.ticket();
    let third = order.ticket();

    // The middle frame is abandoned while the first is still in flight.
    drop(second);
    first.wait().await;
    first.complete();

    timeout(Duration::from_secs(5), third.wait())
        .await
        .expect("the skipped place must not stall the lane");
    assert_eq!(order.turn(), third.seq());
    third.complete();
}

// r[impl jetstream.session.local.order-handoff]
#[tokio::test(flavor = "multi_thread")]
async fn an_ordered_sender_writes_in_admission_order() {
    let pair = LocalSession::<TestProtocol>::pair();
    let lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();
    let sender = lane.ordered_sender();

    let admitted: Vec<_> =
        (0..8).map(|n| (n, sender.admit())).collect::<Vec<_>>();

    let mut handles = Vec::new();
    for (n, ticket) in admitted.into_iter().rev() {
        let sender = sender.clone();
        handles.push(tokio::spawn(async move {
            sender.deliver(ticket, frame(n)).await.unwrap();
        }));
    }
    for handle in handles {
        handle.await.unwrap();
    }

    for n in 0..8 {
        let got = served.next().await.unwrap().unwrap();
        assert_eq!(got.msg, Ping(n));
    }
}

// r[impl jetstream.session.symmetric]
#[tokio::test]
async fn either_end_may_open_a_lane() {
    let pair = LocalSession::<TestProtocol>::pair();
    let mut upstream = pair.server.open_lane().await.unwrap();
    let mut served = pair.client.accept_lane().await.unwrap();

    upstream.send(frame(3)).await.unwrap();
    let got = served.next().await.unwrap().unwrap();
    assert_eq!(got.msg, Ping(3));
}
