//! A chat room, the way a Durable Object would host one.
//!
//! One room, many subscribers, and no transport anywhere in the
//! protocol. The room is served over a `LocalSession` here; the same
//! code serves over QUIC, iroh or WebTransport, because what it needs
//! from a session is lanes and nothing else.
//!
//! A **subscription** is one request and many responses sharing its tag,
//! terminated explicitly. The three things that makes hard, and that
//! this example is really about:
//!
//! * the end is a *value* — `Item::Done(Closed { last_seq })` — so a
//!   subscription can report a result, and so the end survives a merge;
//! * cancelling reaches the *producer*, not just the delivery, so
//!   dropping a subscription stops the work behind it;
//! * a subscription does not take the lane it is served on, so `post`
//!   still works while every subscriber is listening.
//!
//! ```console
//! cargo run --example chat_room
//! ```

use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering::SeqCst},
    Arc,
};

use futures::StreamExt;
use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_rpc::session::{LocalSession, Session};
use tokio::sync::broadcast;

use crate::room_protocol::{RoomChannel, RoomService};

#[derive(Debug, Clone, JetStreamWireFormat)]
pub struct Event {
    pub seq: u64,
    pub who: String,
    pub body: String,
}

/// What the room says when a subscription ends normally. This is the
/// value a plain `Stream` has nowhere to put.
#[derive(Debug, Clone, JetStreamWireFormat)]
pub struct Closed {
    pub last_seq: u64,
}

#[service(uses(super::{Closed, Event}))]
pub trait Room {
    /// Unary, and unchanged by any of this.
    async fn post(
        &self,
        ctx: Context,
        who: String,
        body: String,
    ) -> Result<u64>;

    /// Streaming: every event from `from` onwards, until the subscriber
    /// leaves or the room closes.
    #[subscription]
    fn events(&self, ctx: Context, from: u64) -> Subscription<Event, Closed>;
}

#[derive(Clone)]
struct ChatRoom {
    events: broadcast::Sender<Event>,
    /// Everything said so far.
    ///
    /// r[impl jetstream.subscription.surface.producer]
    /// A producer driven by a *cursor*, not only by a live sender. This
    /// is not decoration: a subscription opens on its own lane, and
    /// `jetstream.lane.no-cross-lane-order` means it is unordered
    /// against a `post` on another one. Without a log to replay from,
    /// "subscribe, then post, then read" is a race that the in-process
    /// session loses reliably — which is how this example found it.
    log: Arc<std::sync::Mutex<Vec<Event>>>,
    seq: Arc<AtomicU64>,
    /// Cancelled when the room itself closes, which is the *other* way a
    /// subscription ends — and the one that carries a result.
    closing: subscription::CancellationToken,
    /// How many producers are alive. The example prints it to show that
    /// cancelling a subscription stops the work, rather than leaving it
    /// running with nowhere to deliver.
    producers: Arc<AtomicUsize>,
}

impl Room for ChatRoom {
    async fn post(
        &self,
        _ctx: Context,
        who: String,
        body: String,
    ) -> Result<u64> {
        let seq = self.seq.fetch_add(1, SeqCst) + 1;
        let event = Event { seq, who, body };
        self.log.lock().unwrap().push(event.clone());
        // Nobody listening is not an error: the room does not know or
        // care who is.
        let _ = self.events.send(event);
        Ok(seq)
    }

    fn events(&self, _ctx: Context, from: u64) -> Subscription<Event, Closed> {
        // Subscribe to the live feed *before* reading the log, so
        // nothing said in between is missed. Anything the replay
        // already covered is skipped by sequence below.
        let mut feed = self.events.subscribe();
        let backlog: Vec<Event> = self
            .log
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.seq >= from)
            .cloned()
            .collect();
        let seq = self.seq.clone();
        let closing = self.closing.clone();
        let producers = self.producers.clone();

        // The producer can send *and* learn that its subscriber has
        // gone — the half a bare `Sender` cannot do.
        Subscription::producing(64, move |producer| async move {
            producers.fetch_add(1, SeqCst);
            let mut sent_through = from.saturating_sub(1);
            for event in backlog {
                sent_through = event.seq;
                if producer.send(event).await.is_err() {
                    producers.fetch_sub(1, SeqCst);
                    return;
                }
            }
            let ending = loop {
                tokio::select! {
                    // Biased, and in this order, because
                    // r[jetstream.subscription.cancel] puts the
                    // terminator *after* every item already emitted.
                    // Left to chance, a subscriber can miss the last
                    // message because the room closed in the same
                    // instant — which is what happened before this line
                    // was here.
                    biased;

                    // The subscriber left. Stop the work; there is no
                    // result to report to someone who is not there, and
                    // nothing to drain for them either.
                    _ = producer.cancelled() => break None,
                    got = feed.recv() => match got {
                        Ok(event) => {
                            // Skip what the replay already covered.
                            if event.seq > sent_through {
                                sent_through = event.seq;
                                if producer.send(event).await.is_err() {
                                    break None;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => {
                            break Some(Closed { last_seq: seq.load(SeqCst) })
                        }
                    },
                    // The room closed. That *is* a result — and it is
                    // reached only once the feed has nothing left.
                    _ = closing.cancelled() => {
                        break Some(Closed { last_seq: seq.load(SeqCst) });
                    }
                }
            };
            if let Some(closed) = ending {
                producer.finish(closed).await;
            }
            producers.fetch_sub(1, SeqCst);
        })
    }
}

/// Built over the **session**, not over a lane: a channel handed one
/// transport can only tag-multiplex, which would make the caller's
/// construction the realisation choice.
///
/// On this session — `LaneSupport::Many` — every subscription gets a
/// lane of its own, for independent flow control. On a session with one
/// lane the same code tag-multiplexes, and the caller's types do not
/// change. That is `jetstream.subscription.realisation.opaque`.
struct RoomOn {
    session: LocalSession<RoomChannel>,
    endpoint: subscription::Endpoint,
}

impl RoomOn {
    async fn lane(&self) -> Result<RoomChannel> {
        let lane = Session::<RoomChannel>::open_lane(&self.session)
            .await
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(RoomChannel::new(64, Box::new(lane)))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let pair = LocalSession::<RoomChannel>::pair();

    let room = ChatRoom {
        events: broadcast::channel(256).0,
        log: Arc::new(std::sync::Mutex::new(Vec::new())),
        seq: Arc::new(AtomicU64::new(0)),
        closing: subscription::CancellationToken::new(),
        producers: Arc::new(AtomicUsize::new(0)),
    };
    let producers = room.producers.clone();
    let closing = room.closing.clone();

    // One task per lane. A lane is a `ServiceTransport`, so it goes
    // straight into the RPC loop with no adapter in between.
    let serving = {
        let server = pair.server.clone();
        tokio::spawn(async move {
            while let Ok(lane) =
                Session::<RoomChannel>::accept_lane(&server).await
            {
                let inner = room.clone();
                tokio::spawn(async move {
                    let mut service = RoomService { inner };
                    let _ =
                        jetstream_rpc::server::run(&mut service, lane).await;
                });
            }
        })
    };

    let on = RoomOn {
        session: pair.client.clone(),
        endpoint: subscription::Endpoint::from("room-42"),
    };
    println!("joined {}", String::from_utf8_lossy(on.endpoint.as_bytes()));

    let poster = on.lane().await?;
    // Ada subscribes on the **same lane** the posts below go out on.
    // With every subscription on a lane of its own, `posted #1` would
    // print even with the dispatcher serving a subscription to
    // exhaustion — the subscription lanes would hang and the unary lane
    // would answer regardless. That is what made an earlier version of
    // this example decorative on its central claim.
    let mut ada = poster.events(Context::default(), 0);
    // Grace and the one who leaves get lanes of their own, which is the
    // other realisation and equally invisible in the types.
    let mut grace = on.lane().await?.events(Context::default(), 0);
    let mut leaving = on.lane().await?.events(Context::default(), 0);

    // A subscription opens when it is first read, so say "now" instead.
    // Necessary, and *not* sufficient: each of these is on its own lane
    // and the post below is on another, and there is no ordering
    // between lanes. That is what `from` is for, and why the room keeps
    // a log — the two together are what make this deterministic.
    for who in [&mut ada, &mut grace, &mut leaving] {
        who.establish().await;
    }

    let seq = poster
        .post(Context::default(), "ada".into(), "is anyone there?".into())
        .await?;
    println!("posted #{seq}");

    for who in [&mut ada, &mut grace, &mut leaving] {
        match who.next().await.expect("an event")? {
            Item::Next(event) => {
                println!("  heard: {} says {}", event.who, event.body)
            }
            Item::Done(_) => panic!("not yet"),
        }
    }
    settle().await;
    println!("producers alive: {}", producers.load(SeqCst));

    // Dropping a subscription cancels it, and cancelling reaches the
    // work: the producer behind it stops, rather than carrying on with
    // nowhere to deliver.
    drop(leaving);
    settle().await;
    println!("producers alive after one left: {}", producers.load(SeqCst));

    // A subscription is one call among the lane's others, so posting
    // works while every subscriber is listening.
    poster
        .post(Context::default(), "grace".into(), "here".into())
        .await?;
    // Closing the room ends every subscription with a *result*.
    closing.cancel();

    // Fan-in: merged, and still able to say which subscription ended and
    // with what. `select_all` over bare item streams would show every
    // event and no ending at all.
    let mut merged = subscription::merge([("ada", ada), ("grace", grace)]);
    while let Some(next) = merged.next().await {
        match next {
            (who, Ok(Item::Next(event))) => {
                println!("  {who} heard: {} says {}", event.who, event.body)
            }
            (who, Ok(Item::Done(closed))) => {
                println!(
                    "  {who}'s subscription closed at #{}",
                    closed.last_seq
                )
            }
            // The key survives a failure too, so a fan-in can say which
            // room went away rather than only that one did.
            (who, Err(e)) => println!("  {who} failed: {e}"),
        }
    }

    settle().await;
    println!("producers alive at the end: {}", producers.load(SeqCst));
    serving.abort();
    Ok(())
}

/// Let the spawned halves catch up. Everything here is in-process, so
/// this is a scheduling nudge rather than a timeout.
async fn settle() {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}
