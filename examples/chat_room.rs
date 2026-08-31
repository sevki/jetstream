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
//! The protocol here is written by hand. `#[service]` will generate all
//! of it — this is what it has to generate.
//!
//! ```console
//! cargo run --example chat_room
//! ```

use std::{
    io,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering::SeqCst},
        Arc,
    },
};

use futures::StreamExt;
use jetstream::prelude::*;
use jetstream_rpc::{
    server::ResponseStream,
    session::{LocalSession, Session},
    subscription::{
        self, CancellationToken, Rcancel, Tcancel, Terminator, RCANCEL, RDONE,
        TCANCEL,
    },
    Mux,
};
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// The wire types. Two methods, so two request ids in the per-method
// space: 102 + 2 * index. The terminator and the cancellation take the
// global ids, which is what keeps that arithmetic intact.

const TPOST: u8 = 102;
const RPOST: u8 = 103;
const TEVENTS: u8 = 104;
const REVENT: u8 = 105;

#[derive(Debug, Clone, JetStreamWireFormat)]
pub struct Event {
    pub seq: u64,
    pub who: String,
    pub body: String,
}

/// What the room says when a subscription ends normally. This is the
/// value that has nowhere to live in a plain `Stream`.
#[derive(Debug, Clone, JetStreamWireFormat)]
pub struct Closed {
    pub last_seq: u64,
}

#[derive(Debug, Clone, JetStreamWireFormat)]
pub struct Post {
    pub who: String,
    pub body: String,
}

#[derive(Debug)]
pub enum Ask {
    Post(Post),
    Events(u64),
    Cancel(Tcancel),
}

#[derive(Debug)]
pub enum Say {
    Posted(u64),
    Event(Event),
    Done(Terminator<Closed>),
    Ack(Rcancel),
}

impl Framer for Ask {
    fn message_type(&self) -> u8 {
        match self {
            Ask::Post(_) => TPOST,
            Ask::Events(_) => TEVENTS,
            Ask::Cancel(_) => TCANCEL,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            Ask::Post(p) => p.byte_size(),
            Ask::Events(f) => f.byte_size(),
            Ask::Cancel(c) => c.byte_size(),
        }
    }

    fn encode<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            Ask::Post(p) => p.encode(w),
            Ask::Events(f) => f.encode(w),
            Ask::Cancel(c) => c.encode(w),
        }
    }

    fn decode<R: io::Read>(r: &mut R, ty: u8) -> io::Result<Self> {
        match ty {
            TPOST => Ok(Ask::Post(WireFormat::decode(r)?)),
            TEVENTS => Ok(Ask::Events(WireFormat::decode(r)?)),
            TCANCEL => Ok(Ask::Cancel(WireFormat::decode(r)?)),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown request type {other}"),
            )),
        }
    }
}

impl Framer for Say {
    fn message_type(&self) -> u8 {
        match self {
            Say::Posted(_) => RPOST,
            Say::Event(_) => REVENT,
            Say::Done(_) => RDONE,
            Say::Ack(_) => RCANCEL,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            Say::Posted(s) => s.byte_size(),
            Say::Event(e) => e.byte_size(),
            Say::Done(d) => d.byte_size(),
            Say::Ack(a) => a.byte_size(),
        }
    }

    fn encode<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            Say::Posted(s) => s.encode(w),
            Say::Event(e) => e.encode(w),
            Say::Done(d) => d.encode(w),
            Say::Ack(a) => a.encode(w),
        }
    }

    fn decode<R: io::Read>(r: &mut R, ty: u8) -> io::Result<Self> {
        match ty {
            RPOST => Ok(Say::Posted(WireFormat::decode(r)?)),
            REVENT => Ok(Say::Event(WireFormat::decode(r)?)),
            // The terminator's payload names its method, because `RDONE`
            // is one global id and a decoder never sees the tag. With
            // one subscription method there is nothing to tell apart
            // yet; with two there would be.
            RDONE => Ok(Say::Done(WireFormat::decode(r)?)),
            RCANCEL => Ok(Say::Ack(WireFormat::decode(r)?)),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown response type {other}"),
            )),
        }
    }
}

#[derive(Clone)]
pub struct Room;

impl Protocol for Room {
    type Error = Error;
    type Request = Ask;
    type Response = Say;

    const NAME: &'static str = "room";
    const VERSION: &'static str = "rs.jetstream.proto/room/0.1.0";

    fn tcancel(oldtag: u16, binding: u64) -> Option<Ask> {
        Some(Ask::Cancel(Tcancel { oldtag, binding }))
    }
}

// ---------------------------------------------------------------------------
// The service.

#[derive(Clone)]
struct ChatRoom {
    events: broadcast::Sender<Event>,
    seq: Arc<AtomicU64>,
    /// Cancelled when the room itself closes, which is the *other* way a
    /// subscription ends — and the one that carries a result.
    closing: CancellationToken,
    /// How many producers are alive. The example prints it to show that
    /// cancelling a subscription stops the work, rather than leaving it
    /// running with nowhere to deliver.
    producers: Arc<AtomicUsize>,
}

impl Protocol for ChatRoom {
    type Error = Error;
    type Request = Ask;
    type Response = Say;

    const NAME: &'static str = Room::NAME;
    const VERSION: &'static str = Room::VERSION;

    fn tcancel(oldtag: u16, binding: u64) -> Option<Ask> {
        Room::tcancel(oldtag, binding)
    }
}

impl Server for ChatRoom {
    fn is_streaming(message_type: u8) -> bool {
        message_type == TEVENTS
    }

    fn cancel_target(frame: &Frame<Ask>) -> Option<u16> {
        match &frame.msg {
            Ask::Cancel(c) => Some(c.oldtag),
            _ => None,
        }
    }

    fn cancel_ack(oldtag: u16) -> Option<Say> {
        Some(Say::Ack(Rcancel { oldtag }))
    }

    fn cancelled_terminator() -> Option<Say> {
        Some(Say::Done(Terminator {
            method: TEVENTS,
            value: Closed { last_seq: 0 },
        }))
    }

    async fn rpc(
        &mut self,
        _ctx: Context,
        frame: Frame<Ask>,
    ) -> std::result::Result<Frame<Say>, Error> {
        match frame.msg {
            Ask::Post(post) => {
                let seq = self.seq.fetch_add(1, SeqCst) + 1;
                // A subscriber that has gone is not an error here: the
                // room does not know or care who is listening.
                let _ = self.events.send(Event {
                    seq,
                    who: post.who,
                    body: post.body,
                });
                Ok(Frame {
                    tag: frame.tag,
                    msg: Say::Posted(seq),
                })
            }
            other => Err(Error::new(format!("not a unary method: {other:?}"))),
        }
    }

    async fn rpc_stream(
        &mut self,
        _ctx: Context,
        frame: Frame<Ask>,
        cancel: CancellationToken,
    ) -> ResponseStream<Self> {
        let tag = frame.tag;
        let from = match frame.msg {
            Ask::Events(from) => from,
            _ => 0,
        };

        // The producer's end. It can send *and* learn that its
        // subscriber has gone — the half a bare `Sender` cannot do.
        let (producer, items) =
            subscription::channel::<Event, Closed>(64, cancel);
        let stop = producer.cancellation();
        let mut feed = self.events.subscribe();
        let seq = self.seq.clone();
        let closing = self.closing.clone();
        let producers = self.producers.clone();
        producers.fetch_add(1, SeqCst);

        tokio::spawn(async move {
            let ending = loop {
                tokio::select! {
                    // The subscriber went away. Stop the work; there is
                    // no result to report to someone who is not there.
                    _ = stop.cancelled() => break None,
                    // The room closed. That *is* a result.
                    _ = closing.cancelled() => {
                        break Some(Closed { last_seq: seq.load(SeqCst) });
                    }
                    got = feed.recv() => match got {
                        Ok(event) => {
                            if event.seq >= from
                                && producer.send(event).await.is_err()
                            {
                                break None;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => {
                            break Some(Closed { last_seq: seq.load(SeqCst) })
                        }
                    },
                }
            };
            if let Some(closed) = ending {
                producer.finish(closed).await;
            }
            producers.fetch_sub(1, SeqCst);
        });

        items.into_frames::<Self, _>(tag, move |item| match item {
            subscription::Item::Next(event) => Say::Event(event),
            subscription::Item::Done(closed) => Say::Done(Terminator {
                method: TEVENTS,
                value: closed,
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// The client.

/// Built over the **session**, not over a lane — a channel handed one
/// transport can only tag-multiplex, which would make the caller's
/// construction the realisation choice.
struct RoomChannel {
    session: LocalSession<Room>,
    endpoint: Endpoint,
    unary: Mux<Room>,
}

impl RoomChannel {
    async fn over(
        session: LocalSession<Room>,
        endpoint: Endpoint,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let lane = Session::<Room>::open_lane(&session).await?;
        Ok(RoomChannel {
            session,
            endpoint,
            unary: Mux::new(64, Box::new(lane)),
        })
    }

    async fn post(
        &self,
        who: &str,
        body: &str,
    ) -> std::result::Result<u64, Error> {
        let answer = self
            .unary
            .rpc(
                Context::default(),
                Ask::Post(Post {
                    who: who.to_string(),
                    body: body.to_string(),
                }),
            )
            .await
            .await?;
        match answer.msg {
            Say::Posted(seq) => Ok(seq),
            other => Err(Error::new(format!("unexpected answer: {other:?}"))),
        }
    }

    /// A lane of its own, because this session has many — the choice
    /// `Capability::ManyLanes` licenses and that the caller never sees.
    /// On a session with one lane this would be a tag on the shared one,
    /// and the type below would be the same.
    async fn events(
        &self,
        from: u64,
    ) -> std::result::Result<
        Subscription<Event, Closed>,
        Box<dyn std::error::Error>,
    > {
        let lane = Session::<Room>::open_lane(&self.session).await?;
        let mux = Mux::<Room>::new(64, Box::new(lane));
        let frames = mux
            .rpc_stream(Context::default(), Ask::Events(from), 64)
            .await;
        Ok(Subscription::from_frames(frames, move |frame| {
            // The lane's multiplexer has to outlive the subscription
            // reading through it, so it rides along in this closure.
            let _ = &mux;
            match frame.msg {
                Say::Event(event) => Ok(Item::Next(event)),
                Say::Done(done) => Ok(Item::Done(done.value)),
                other => Err(Error::new(format!("unexpected item: {other:?}"))),
            }
        }))
    }

    fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
}

// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let pair = LocalSession::<Room>::pair();

    let room = ChatRoom {
        events: broadcast::channel(256).0,
        seq: Arc::new(AtomicU64::new(0)),
        closing: CancellationToken::new(),
        producers: Arc::new(AtomicUsize::new(0)),
    };
    let producers = room.producers.clone();
    let closing = room.closing.clone();

    // One task per lane. A lane is a `ServiceTransport`, so it goes
    // straight into the RPC loop with no adapter in between.
    let serving = {
        let server = pair.server.clone();
        tokio::spawn(async move {
            while let Ok(lane) = Session::<Room>::accept_lane(&server).await {
                let mut room = room.clone();
                tokio::spawn(async move {
                    let _ = jetstream_rpc::server::run(&mut room, lane).await;
                });
            }
        })
    };

    let channel =
        RoomChannel::over(pair.client.clone(), Endpoint::from("room-42"))
            .await?;
    println!(
        "joined {}",
        String::from_utf8_lossy(channel.endpoint().as_bytes())
    );

    // Two subscribers, and a third that leaves early.
    let mut ada = channel.events(0).await?;
    let mut grace = channel.events(0).await?;
    let leaving = channel.events(0).await?;
    settle().await;
    println!("producers alive: {}", producers.load(SeqCst));

    // A subscription is one call among the lane's others. This used to
    // hang: the dispatcher served a subscription by consuming it, and
    // never read the lane again.
    let seq = channel.post("ada", "is anyone there?").await?;
    println!("posted #{seq}");

    for who in [&mut ada, &mut grace] {
        match who.next().await.expect("an event")? {
            Item::Next(event) => {
                println!("  heard: {} says {}", event.who, event.body)
            }
            Item::Done(_) => panic!("not yet"),
        }
    }

    // Dropping a subscription cancels it, and cancelling reaches the
    // work: the producer behind it stops, rather than carrying on with
    // nowhere to deliver.
    drop(leaving);
    settle().await;
    println!("producers alive after one left: {}", producers.load(SeqCst));

    // Closing the room ends every subscription with a *result*. This is
    // the value a plain `Stream` has nowhere to put.
    channel.post("grace", "here").await?;
    closing.cancel();

    // Fan-in: merged, and still able to say which subscription ended and
    // with what. `select_all` over bare item streams would show every
    // event and no ending at all.
    let mut merged = subscription::merge([("ada", ada), ("grace", grace)]);
    while let Some(next) = merged.next().await {
        match next? {
            (who, Item::Next(event)) => {
                println!("  {who} heard: {} says {}", event.who, event.body)
            }
            (who, Item::Done(closed)) => {
                println!(
                    "  {who}'s subscription closed at #{}",
                    closed.last_seq
                )
            }
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
