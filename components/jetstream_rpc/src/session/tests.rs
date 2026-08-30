use std::{
    io::{self, Read, Write},
    pin::Pin,
    sync::{atomic::AtomicUsize, Arc, Mutex},
    time::Duration,
};

use futures::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::{
    context::Contextual,
    session::{
        check_datagram_size, local::LocalSessionPair, Capabilities, Capability,
        IdentityKind, LaneOrder, LaneSupport, LocalSession, Session,
        SessionError, SingleLaneSession,
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

    // r[impl jetstream.session.lifetime]
    // The peer's end terminates too. A lane has two ends, and leaving
    // the far one waiting on a lane whose opener has gone is the hang
    // this requirement exists to prevent.
    let peer = timeout(Duration::from_millis(500), served.next())
        .await
        .expect("the peer's end must not hang on a closed session");
    match peer {
        Some(Err(err)) => {
            assert_eq!(err.code(), Some("jetstream::session::closed"))
        }
        None => {}
        Some(Ok(frame)) => panic!("unexpected frame {:?}", frame.msg),
    }
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

// r[impl jetstream.session.conformance.single-stream]
#[tokio::test]
async fn a_byte_stream_is_a_session_with_one_lane() {
    use tokio::net::{TcpListener, TcpStream};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (connected, accepted) =
        tokio::join!(TcpStream::connect(addr), listener.accept());
    let client_io = connected.unwrap();
    let (server_io, _) = accepted.unwrap();

    let client = SingleLaneSession::<TestProtocol, _, _>::client_io(client_io);
    let server = SingleLaneSession::<TestProtocol, _, _>::service_io(server_io);

    assert_eq!(client.capabilities().lanes, LaneSupport::One);
    assert_eq!(client.capabilities().identity, IdentityKind::None);
    assert!(!client.capabilities().datagrams);

    let mut lane = client.open_lane().await.unwrap();
    let mut served = server.accept_lane().await.unwrap();

    // r[impl jetstream.lane.delivery-order]
    for n in 0..4 {
        lane.send(frame(n)).await.unwrap();
    }
    for n in 0..4 {
        let got = served.next().await.unwrap().unwrap();
        assert_eq!(got.msg, Ping(n));
    }

    // r[impl jetstream.session.identity]
    // TCP authenticates nothing, so only the address is reported.
    let context = Contextual::context(&served);
    assert!(context.remote().is_some());
    assert!(context.peer().is_none());

    // r[impl jetstream.session.single-lane]
    assert!(matches!(
        client.open_lane().await,
        Err(SessionError::LaneLimitReached)
    ));
    assert!(matches!(
        server.accept_lane().await,
        Err(SessionError::LaneLimitReached)
    ));
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
// session that opened many short-lived lanes grew without bound. Lanes
// now hold a child cancellation token, which deregisters on drop.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn dropped_lanes_do_not_accumulate() {
    let pair = LocalSession::<TestProtocol>::pair();

    for _ in 0..64 {
        let lane = pair.client.open_lane().await.unwrap();
        let served = pair.server.accept_lane().await.unwrap();
        drop(lane);
        drop(served);
    }

    assert_eq!(pair.client.live_lanes(), 0);
    assert_eq!(pair.server.live_lanes(), 0);

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

// Regression: an `OrderedSender` held only a channel handle, so after
// its session closed it could still deliver to the peer.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn an_ordered_sender_stops_when_its_session_closes() {
    let pair = LocalSession::<TestProtocol>::pair();
    let lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();
    let sender = lane.ordered_sender();

    sender.send(frame(1)).await.unwrap();
    let got = served.next().await.unwrap().unwrap();
    assert_eq!(got.msg, Ping(1));

    pair.client.close().await;

    let err = timeout(Duration::from_secs(5), sender.send(frame(2)))
        .await
        .expect("a closed session must not leave the sender hanging")
        .expect_err("and the send must fail");
    assert!(matches!(err, SessionError::Closed));

    // Nothing reached the peer after the close.
    let quiet = timeout(Duration::from_millis(100), served.next()).await;
    match quiet {
        Err(_) => {}
        Ok(None) => {}
        Ok(Some(Err(_))) => {}
        Ok(Some(Ok(frame))) => {
            panic!("frame {:?} escaped a closed session", frame.msg)
        }
    }
}

// Regression: a pending ordered send used to stay parked when the
// session closed underneath it.
// r[impl jetstream.session.lifetime]
#[tokio::test(flavor = "multi_thread")]
async fn a_parked_ordered_send_gives_up_when_the_session_closes() {
    let pair = LocalSession::<TestProtocol>::pair_with_capacity(1);
    let lane = pair.client.open_lane().await.unwrap();
    let _served = pair.server.accept_lane().await.unwrap();
    let sender = lane.ordered_sender();

    // Fill the lane so the next send has to wait for room.
    sender.send(frame(0)).await.unwrap();

    let parked = {
        let sender = sender.clone();
        tokio::spawn(async move { sender.send(frame(1)).await })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;

    pair.client.close().await;

    let outcome = timeout(Duration::from_secs(5), parked)
        .await
        .expect("a parked send must give up when the session closes")
        .unwrap();
    assert!(matches!(outcome, Err(SessionError::Closed)));
}

// Regression: an accepted lane reported the framed stream's context, so
// identity the session established during a handshake the stream never
// saw — a TLS adapter's peer certificate — was lost before the handler.
// r[impl jetstream.session.identity]
#[tokio::test]
async fn an_accepted_lane_reports_the_session_identity() {
    use crate::context::{Context, Contextual, RemoteAddr};

    let pair = LocalSession::<TestProtocol>::pair();
    let _lane = pair.client.open_lane().await.unwrap();
    let inner = pair.server.accept_lane().await.unwrap();

    let session_context = Context::new(
        Some(RemoteAddr::IpAddr("203.0.113.7".parse().unwrap())),
        None,
    );
    let session = SingleLaneSession::<TestProtocol, _, _>::service(inner)
        .with_identity(IdentityKind::Certificate)
        .with_context(session_context.clone());

    let served = session.accept_lane().await.unwrap();
    assert_eq!(
        Contextual::context(&served).remote(),
        session_context.remote(),
        "the handler must see the identity the session established"
    );

    // Without a session identity the lane's own context still stands.
    let pair = LocalSession::<TestProtocol>::pair();
    let _lane = pair.client.open_lane().await.unwrap();
    let inner = pair.server.accept_lane().await.unwrap();
    let plain = SingleLaneSession::<TestProtocol, _, _>::service(inner);
    let served = plain.accept_lane().await.unwrap();
    assert!(Contextual::context(&served).remote().is_none());
}

// Regression: a lane handed out concurrently with `close` could register
// its token after `close` had cleared them, escaping cancellation.
// r[impl jetstream.session.lifetime]
#[tokio::test(flavor = "multi_thread")]
async fn a_lane_opened_while_closing_is_born_closed() {
    for _ in 0..64 {
        let pair = LocalSession::<TestProtocol>::pair();
        let client = pair.client.clone();

        let opening = tokio::spawn(async move { client.open_lane().await });
        pair.client.close().await;

        if let Ok(mut lane) = opening.await.unwrap() {
            // If a lane did come back, it must already be terminated
            // rather than still usable on a closed session.
            let outcome = timeout(Duration::from_secs(5), lane.next())
                .await
                .expect("a lane from a closed session must not hang");
            match outcome {
                Some(Err(err)) => {
                    assert_eq!(err.code(), Some("jetstream::session::closed"))
                }
                None => {}
                Some(Ok(frame)) => {
                    panic!(
                        "closed session yielded a live lane: {:?}",
                        frame.msg
                    )
                }
            }
        }
    }
}

// Regression: each end of a pair had its own token, so closing one side
// left the peer's accepted lane live and its reads pending, and let the
// closing side keep opening lanes nobody could ever accept.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn closing_one_end_terminates_the_peers_lanes() {
    let pair = LocalSession::<TestProtocol>::pair();
    let _lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();

    // The client lane is deliberately still held: its channel handles
    // alone used to keep the peer's read pending forever.
    pair.client.close().await;

    let peer = timeout(Duration::from_millis(500), served.next())
        .await
        .expect("the peer's lane must terminate, not hang");
    match peer {
        Some(Err(err)) => {
            assert_eq!(err.code(), Some("jetstream::session::closed"))
        }
        None => {}
        Some(Ok(frame)) => panic!("unexpected frame {:?}", frame.msg),
    }

    // And the association is over for the peer as well: it neither
    // accepts nor opens.
    assert!(matches!(
        pair.server.open_lane().await,
        Err(SessionError::Closed)
    ));
    assert!(matches!(
        timeout(Duration::from_millis(500), pair.server.accept_lane())
            .await
            .expect("accept must not hang after the peer closed"),
        Err(SessionError::Closed)
    ));
}

// Regression: closing the server used to leave the client happily
// opening lanes that could never be accepted.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn closing_the_server_stops_the_client_opening() {
    let pair = LocalSession::<TestProtocol>::pair();
    pair.server.close().await;

    assert!(matches!(
        pair.client.open_lane().await,
        Err(SessionError::Closed)
    ));
}

// Regression: `poll_ready` could say yes, the session close, and
// `start_send` then commit the write anyway and report success.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn a_write_committed_after_close_fails() {
    use futures::Sink;

    let pair = LocalSession::<TestProtocol>::pair();
    let mut lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();

    // Reserve capacity first, exactly as a caller would before writing.
    futures::future::poll_fn(|cx| {
        Sink::<Frame<Ping>>::poll_ready(Pin::new(&mut lane), cx)
    })
    .await
    .unwrap();

    // The session closes between the reservation and the write.
    pair.client.close().await;

    let committed =
        Sink::<Frame<Ping>>::start_send(Pin::new(&mut lane), frame(9));
    assert!(
        committed.is_err(),
        "a write committed after close must not succeed"
    );

    // ...and nothing reached the peer.
    let peer = timeout(Duration::from_millis(200), served.next()).await;
    if let Ok(Some(Ok(frame))) = peer {
        panic!("frame {:?} escaped a closed session", frame.msg);
    }
}

// Regression: dropping the last session handle left its lanes usable,
// because dropping a cancellation token does not cancel it.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn dropping_the_last_session_handle_terminates_its_lanes() {
    let pair = LocalSession::<TestProtocol>::pair();
    let mut lane = pair.client.open_lane().await.unwrap();
    let sender = lane.ordered_sender();

    let LocalSessionPair { client, server } = pair;
    drop(client);
    drop(server);

    let err = timeout(Duration::from_millis(500), lane.next())
        .await
        .expect("a lane whose session is gone must not hang")
        .expect("it reports the closure")
        .expect_err("as an error");
    assert_eq!(err.code(), Some("jetstream::session::closed"));

    // Handles derived from the lane are done too.
    let err = timeout(Duration::from_millis(500), sender.send(frame(1)))
        .await
        .expect("a derived sender must not hang either")
        .expect_err("and must fail");
    assert!(matches!(err, SessionError::Closed));
}

// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn dropping_a_single_lane_session_terminates_its_lane() {
    let pair = LocalSession::<TestProtocol>::pair();
    let inner = pair.client.open_lane().await.unwrap();
    let session = SingleLaneSession::<TestProtocol, _, _>::client(inner);
    let mut lane = session.open_lane().await.unwrap();

    drop(session);

    let err = timeout(Duration::from_millis(500), lane.next())
        .await
        .expect("a lane whose session is gone must not hang")
        .expect("it reports the closure")
        .expect_err("as an error");
    assert_eq!(err.code(), Some("jetstream::session::closed"));
}

// Regression: `LaneGuard::start_send` forwarded unconditionally. The
// round-three patch that was meant to add this check silently failed to
// apply — the two lane sinks got it, the guard did not, and the reply
// claiming otherwise was wrong.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn a_guarded_write_committed_after_close_fails() {
    use futures::Sink;

    let pair = LocalSession::<TestProtocol>::pair();
    let inner = pair.client.open_lane().await.unwrap();
    let session = SingleLaneSession::<TestProtocol, _, _>::client(inner);
    let mut lane = session.open_lane().await.unwrap();

    futures::future::poll_fn(|cx| {
        Sink::<Frame<Ping>>::poll_ready(Pin::new(&mut lane), cx)
    })
    .await
    .unwrap();

    session.close().await;

    assert!(
        Sink::<Frame<Ping>>::start_send(Pin::new(&mut lane), frame(9)).is_err(),
        "a guarded write committed after close must not succeed"
    );
}

// A send admitted before a close, delivered after it, must fail.
//
// Note what this does and does not cover: the added pre-check catches
// this case, so the test passes with or without the `select(closing,
// sending)` re-ordering that accompanies it. That re-ordering guards the
// residual window where the close lands between the pre-check and the
// poll, which is not reachable deterministically from here.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn an_ordered_send_admitted_before_a_close_fails() {
    let pair = LocalSession::<TestProtocol>::pair();
    let lane = pair.client.open_lane().await.unwrap();
    let mut served = pair.server.accept_lane().await.unwrap();
    let sender = lane.ordered_sender();

    let ticket = sender.admit();
    pair.client.close().await;

    let err = timeout(Duration::from_secs(5), sender.deliver(ticket, frame(1)))
        .await
        .expect("must not hang")
        .expect_err("a closed session must refuse the send");
    assert!(matches!(err, SessionError::Closed));

    let escaped = timeout(Duration::from_millis(100), served.next()).await;
    if let Ok(Some(Ok(frame))) = escaped {
        panic!("frame {:?} escaped a closed session", frame.msg);
    }
}

// r[impl jetstream.lane.backpressure]
#[test]
#[should_panic(expected = "at least one frame")]
fn a_zero_capacity_lane_is_rejected_at_construction() {
    // Zero capacity cannot apply backpressure, only deadlock, and the
    // channel underneath rejects it. Say so here rather than panicking
    // inside the first open.
    let _ = LocalSession::<TestProtocol>::pair_with_capacity(0);
}

// Regression: converting `SessionError::Transport` into `Error` returned
// the inner error untouched, so the code the docs promised was never
// there — a converted transport error was usually uninspectable.
// r[impl jetstream.session.single-lane]
#[test]
fn a_converted_transport_error_is_always_inspectable() {
    // An inner error with no code of its own gets the session's.
    let bare: Error = SessionError::Transport(Error::new("boom")).into();
    assert_eq!(bare.code(), Some(SessionError::TRANSPORT_CODE));

    // An inner error that names itself keeps its own code, which says
    // more than the session-level one would.
    let specific: Error =
        SessionError::Transport(Error::with_code("boom", "quic::reset")).into();
    assert_eq!(specific.code(), Some("quic::reset"));

    // Every other variant reports exactly its own code.
    let closed: Error = SessionError::Closed.into();
    assert_eq!(closed.code(), Some("jetstream::session::closed"));
}

// Regression: `service_io` only produced a usable session for the few
// stream types with a `Contextual` impl — a unix or TCP socket. A TLS
// stream, stdio, or an in-memory duplex got a session with no `Session`
// impl at all, so the advertised generic byte-stream server binding did
// not exist for them.
// r[impl jetstream.session.conformance.single-stream]
#[tokio::test]
async fn any_byte_stream_can_serve_a_lane() {
    use crate::context::{Context, Contextual, Peer, WebCredentials};

    // A duplex stream stands in for the streams that cannot report a
    // peer themselves: TLS before the handshake is handed over, stdio,
    // anything in memory.
    let (client_io, server_io) = tokio::io::duplex(4096);

    let client = SingleLaneSession::<TestProtocol, _, _>::client_io(client_io);

    // The caller states the identity the stream cannot.
    let identity = Context::new(
        None,
        Some(Peer::WebCredentials(WebCredentials(
            http::HeaderValue::from_static("peer-from-handshake"),
        ))),
    );
    let server =
        SingleLaneSession::<TestProtocol, _, _>::service_io_with_context(
            server_io,
            identity.clone(),
        );

    let mut lane = client.open_lane().await.unwrap();
    let mut served = server.accept_lane().await.unwrap();

    lane.send(frame(4)).await.unwrap();
    let got = served.next().await.unwrap().unwrap();
    assert_eq!(got.msg, Ping(4));

    // r[impl jetstream.session.identity]
    assert_eq!(
        Contextual::context(&served).peer(),
        identity.peer(),
        "the handler sees the identity the caller supplied"
    );

    // The session reports it too, so a caller can inspect the peer
    // before accepting a lane rather than only through the lane.
    assert_eq!(
        Session::context(&server).peer(),
        identity.peer(),
        "the session reports the identity it was constructed with"
    );
}

// Regression: `LaneGuard::poll_close` was the one sink method that did
// not consult the lifetime, so a close already parked on the wrapped
// lane never woke when the session closed. Every other method — ready,
// send, flush, next — did.
// r[impl jetstream.session.lifetime]
#[tokio::test]
async fn a_parked_guarded_close_gives_up_when_the_session_closes() {
    use std::task::Poll;

    use futures::Sink;

    use crate::session::lifetime::{LaneGuard, LaneLifetime};

    /// A lane whose close never finishes, standing in for a transport
    /// wedged behind a peer that has stopped reading.
    struct NeverCloses;

    impl Sink<Frame<Ping>> for NeverCloses {
        type Error = Error;

        fn poll_ready(
            self: Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(
            self: Pin<&mut Self>,
            _: Frame<Ping>,
        ) -> Result<(), Error> {
            Ok(())
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Error>> {
            Poll::Pending
        }
    }

    let token = CancellationToken::new();
    let live = Arc::new(AtomicUsize::new(0));
    let mut guard = LaneGuard::new(
        NeverCloses,
        LaneLifetime::new(token.child_token(), live),
    );

    // The close parks, since the lane never finishes closing.
    let parked = timeout(Duration::from_millis(50), guard.close()).await;
    assert!(parked.is_err(), "the close should still be waiting");

    // Closing the session has to reach it.
    token.cancel();

    let ended = timeout(Duration::from_secs(5), guard.close())
        .await
        .expect("a parked close must give up when its session closes");
    assert_eq!(
        ended.unwrap_err().code(),
        Some("jetstream::session::closed")
    );
}
