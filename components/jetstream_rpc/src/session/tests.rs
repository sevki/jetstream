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

// Regression: a single-lane session used to become a no-op on close
// once its lane had been handed out, leaving the lane running and an
// in-flight call waiting on a session that had closed.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn closing_a_single_lane_session_terminates_the_handed_out_lane() {
    let pair = LocalSession::<TestProtocol>::pair();
    let inner = pair.client.open_lane().await.unwrap();
    let session = SingleLaneSession::<TestProtocol, _, _>::client(inner);

    let mut lane = session.open_lane().await.unwrap();
    session.close().await;

    let err = timeout(Duration::from_millis(200), lane.next())
        .await
        .expect("a closed session must not leave the lane hanging")
        .expect("the lane reports the closure")
        .expect_err("the lane reports it as an error");
    assert_eq!(err.code(), Some("jetstream::session::closed"));

    assert!(lane.send(frame(1)).await.is_err());
    assert!(lane.next().await.is_none(), "and then the lane ends");
}

// Regression: the second of two concurrent accepts used to build its
// wakeup after `close` had already woken the waiters, and wait forever.
// r[impl jetstream.session.lifetime]
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_accepts_all_terminate_when_the_session_closes() {
    let pair = LocalSession::<TestProtocol>::pair();

    let mut waiting = Vec::new();
    for _ in 0..4 {
        let server = pair.server.clone();
        waiting.push(tokio::spawn(async move { server.accept_lane().await }));
    }

    // Let them reach the accept before closing: one holds the offers
    // lock, the rest are queued behind it.
    tokio::time::sleep(Duration::from_millis(50)).await;
    pair.server.close().await;

    for handle in waiting {
        let outcome = timeout(Duration::from_secs(5), handle)
            .await
            .expect("every parked accept must wake on close")
            .unwrap();
        assert!(matches!(outcome, Err(SessionError::Closed)));
    }
}

// Regression: every lane ever opened used to leave a token behind, so a
// session that opened many short-lived lanes grew without bound.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn dropped_lanes_do_not_accumulate_tokens() {
    let pair = LocalSession::<TestProtocol>::pair();

    for _ in 0..64 {
        let lane = pair.client.open_lane().await.unwrap();
        let served = pair.server.accept_lane().await.unwrap();
        drop(lane);
        drop(served);
    }

    assert!(
        pair.client.token_slots() <= 2,
        "token slots should track live lanes, not historical ones: {}",
        pair.client.token_slots()
    );
    assert_eq!(pair.client.live_lanes(), 0);

    // Live lanes are still counted.
    let _held = pair.client.open_lane().await.unwrap();
    assert_eq!(pair.client.live_lanes(), 1);
}

// r[impl jetstream.lane.backpressure]
#[tokio::test]
async fn a_lane_reports_pending_rather_than_buffering_without_bound() {
    use futures::FutureExt;

    let pair = LocalSession::<TestProtocol>::pair_with_capacity(1);
    let mut lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();

    // Write until the sink stops accepting immediately. A lane that
    // buffered without bound would take all of these.
    let mut written = 0u32;
    for n in 0..64 {
        match lane.send(frame(n)).now_or_never() {
            Some(result) => {
                result.unwrap();
                written += 1;
            }
            None => break,
        }
    }
    assert!(written > 0, "the lane accepted nothing at all");
    assert!(
        written < 64,
        "a bounded lane must eventually report pending, wrote {written}"
    );

    // Draining the reader lets the writer proceed again.
    for n in 0..written {
        let got = timeout(Duration::from_secs(5), served.next())
            .await
            .expect("what the lane accepted must be readable")
            .unwrap()
            .unwrap();
        assert_eq!(got.msg, Ping(n));
    }
    timeout(Duration::from_secs(5), lane.send(frame(written)))
        .await
        .expect("a drained lane accepts writes again")
        .unwrap();
}

// r[impl jetstream.session.trait]
#[tokio::test(flavor = "multi_thread")]
async fn a_session_opens_lanes_from_several_tasks_at_once() {
    let pair = LocalSession::<TestProtocol>::pair();

    let mut opening = Vec::new();
    for _ in 0..16 {
        let client = pair.client.clone();
        opening.push(tokio::spawn(async move { client.open_lane().await }));
    }
    for handle in opening {
        handle
            .await
            .unwrap()
            .expect("opening needs no exclusive access");
    }

    for _ in 0..16 {
        timeout(Duration::from_secs(5), pair.server.accept_lane())
            .await
            .expect("every opened lane is offered")
            .expect("and accepted");
    }
}

// r[impl jetstream.lane.delivery-order]
// r[impl jetstream.lane.no-cross-lane-order]
#[tokio::test(flavor = "multi_thread")]
async fn each_lane_keeps_its_own_order_while_lanes_interleave() {
    let pair = LocalSession::<TestProtocol>::pair();

    let mut lanes = Vec::new();
    let mut served = Vec::new();
    for _ in 0..4 {
        lanes.push(pair.client.open_lane().await.unwrap());
        served.push(pair.server.accept_lane().await.unwrap());
    }

    // Write to the lanes round-robin, so the lanes interleave on the
    // wire while each one's own frames stay in order.
    for n in 0..8u32 {
        for (lane_index, lane) in lanes.iter_mut().enumerate() {
            lane.send(frame(n * 10 + lane_index as u32)).await.unwrap();
        }
    }

    for (lane_index, incoming) in served.iter_mut().enumerate() {
        for n in 0..8u32 {
            let got = incoming.next().await.unwrap().unwrap();
            assert_eq!(
                got.msg,
                Ping(n * 10 + lane_index as u32),
                "lane {lane_index} delivered out of order"
            );
        }
    }
}

// r[impl jetstream.session.local.order-handoff]
#[tokio::test(flavor = "multi_thread")]
async fn order_survives_abandonment_scattered_through_the_queue() {
    let order = LaneOrder::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    let tickets: Vec<_> = (0..32u32).map(|n| (n, order.ticket())).collect();

    // Abandon every third place; the rest must still arrive in order.
    let mut expected = Vec::new();
    let mut handles = Vec::new();
    for (n, ticket) in tickets.into_iter().rev() {
        if n % 3 == 0 {
            drop(ticket);
            continue;
        }
        expected.push(n);
        let log = log.clone();
        handles.push(tokio::spawn(async move {
            ticket.wait().await;
            log.lock().unwrap().push(n);
            ticket.complete();
        }));
    }
    expected.sort_unstable();

    for handle in handles {
        timeout(Duration::from_secs(5), handle)
            .await
            .expect("an abandoned place must not stall the lane")
            .unwrap();
    }

    assert_eq!(*log.lock().unwrap(), expected);
}

// r[impl jetstream.session.local.order-handoff]
#[tokio::test]
async fn every_place_is_eventually_reached() {
    let order = LaneOrder::new();
    let tickets: Vec<_> = (0..16).map(|_| order.ticket()).collect();

    assert_eq!(order.turn(), 0);
    for (n, ticket) in tickets.into_iter().enumerate() {
        assert_eq!(ticket.seq(), n as u64);
        ticket.wait().await;
        assert_eq!(order.turn(), n as u64);
        ticket.complete();
    }
    assert_eq!(order.turn(), 16);
}

// r[impl jetstream.lane.backpressure]
// A write that is cancelled — a `select!` arm that loses, a write under
// a timeout — must not wedge the lane. `futures::channel::mpsc` refuses
// every later write on a sender whose send future was dropped while
// pending, which is why the lane does not use one.
#[tokio::test]
async fn a_cancelled_write_does_not_wedge_the_lane() {
    let pair = LocalSession::<TestProtocol>::pair_with_capacity(1);
    let mut lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();

    lane.send(frame(0)).await.unwrap();

    // This one cannot complete: the lane holds one frame and nothing is
    // reading. Cancel it.
    let cancelled =
        timeout(Duration::from_millis(50), lane.send(frame(1))).await;
    assert!(cancelled.is_err(), "the lane should be full here");

    // Draining lets writes proceed again.
    let got = served.next().await.unwrap().unwrap();
    assert_eq!(got.msg, Ping(0));

    timeout(Duration::from_secs(5), lane.send(frame(2)))
        .await
        .expect("a cancelled write must not wedge the lane")
        .unwrap();

    let got = timeout(Duration::from_secs(5), served.next())
        .await
        .expect("and the frame arrives")
        .unwrap()
        .unwrap();
    assert_eq!(got.msg, Ping(2));
}
