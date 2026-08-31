# Subscriptions

A **subscription** is a streaming response: one request, many responses
sharing its tag, terminated explicitly.

That is the whole shape. It adds no new role — the subscriber is still the
caller, the producer is still the service — and no new transport
requirement: a subscription is realisable on any session JetStream
supports, including one with a single lane.

The normative rules live in
[the subscriptions specification](https://github.com/sevki/jetstream/blob/main/docs/specs/subscriptions.md).
This page is the guide to using them.

## Why not a reverse call

The obvious way to let a service push is to make it the caller — open a
lane the other way and let the service issue requests. JetStream does not,
and the reason is not that it cannot: it is that the subscriber then has
no correlation key to cancel by, no place to put backpressure that the
producer can see, and nothing that ends. A subscription keeps all three by
staying a call: the tag that identifies the request identifies the
subscription for as long as it lives, and the terminator that frees the
tag is the thing that ended.

The cost is that a live subscription holds a tag for its whole life. That
is real, and the specification counts it.

## Three things the shape has to get right

**The end is a value.** A subscription that reports on an operation — an
upload, a build, a room closing — has a *result*, and a sequence that ends
with the absence of an item has nowhere to put one. So the terminator is
an `Item::Done(_)` in the sequence rather than the sequence stopping:

```rust,ignore
while let Some(item) = events.next().await {
    match item? {
        Item::Next(event) => render(event),
        Item::Done(closed) => return Ok(closed.last_seq),
    }
}
```

This is also what makes fan-in work. `select_all` yields items and drops a
finished stream silently, so merging subscriptions whose end was `None`
shows every item and no ending at all. `subscription::merge` labels each
one, so a merged consumer can still say *which* ended and with what.

**Cancelling reaches the work.** Dropping a subscription is how a Rust
caller cancels, and it must stop the producer, not merely stop reading it.
Dropping sends a cancellation naming the subscription's tag; the
dispatcher cancels the token the producer holds; the producer's next
`send` fails even if it watches nothing else. A producer that does watch
can stop between items:

```rust,ignore
tokio::select! {
    _ = producer.cancelled() => break,
    next = expensive_inference() => producer.send(next).await?,
}
```

**A subscription does not take its lane.** A room that stays open is the
normal case, and a dispatcher that serves a subscription by consuming it
never reads that lane again — no second request served, and no
cancellation able to arrive. `server::run` serves the subscriptions in
flight *and* the requests still arriving, together.

## Declaring one

A subscription is a method on an ordinary `#[service]` trait, marked
`#[subscription]` and returning `Subscription<Item, Done>`:

```rust,ignore
#[service]
pub trait Room {
    /// Unary. Unchanged by any of this.
    async fn post(&self, ctx: Context, who: String, body: String)
        -> Result<u64>;

    /// Streaming: many `Event`s share the request's tag, and the end
    /// carries a `Closed`.
    #[subscription]
    fn events(&self, ctx: Context, from: u64) -> Subscription<Event, Closed>;
}
```

The declaration is the protocol's, not the call site's: a dispatcher has
to route on it before it decodes the payload, and a caller must not be
able to turn a unary method into a subscription by asking differently.

A streaming method costs no per-method message id. The terminator and the
cancellation take global ids below `MESSAGE_ID_START`, so `102 + 2 *
index` is untouched and no existing protocol is renumbered. That has one
consequence worth knowing: because `RDONE` is a single id, and a decoder
is handed the type byte without the tag, a terminator that carries a
typed payload names its method in the payload. One byte, and a protocol
can have as many subscription methods as it likes.

The method is not `async`, and the subscription opens when it is first
polled — which is what lets the signature be the plain `fn` it should be
while acquiring a tag and sending a request stay asynchronous. It also
means *nothing is on the wire until something reads*. Combined with
`jetstream.lane.no-cross-lane-order` — a subscription on its own lane is
unordered against a call on another — that makes "subscribe, then act,
then read" a race. The cursor parameter is the answer: `from` is not
decoration, it is what makes the sequence well-defined regardless of when
the request arrived. A producer that ignores it and only forwards live
events will drop messages, reliably.

## A room

The example is a chat room: one room, many subscribers, no transport in
the protocol. It runs over a `LocalSession`; the same code runs over QUIC,
iroh or WebTransport, because what it needs from a session is lanes and
nothing else.

```console
cargo run --example chat_room
```

```text
joined room-42
posted #1
  heard: ada says is anyone there?
  heard: ada says is anyone there?
  heard: ada says is anyone there?
producers alive: 3
producers alive after one left: 2
  grace heard: grace says here
  grace's subscription closed at #2
  ada heard: grace says here
  ada's subscription closed at #2
producers alive at the end: 0
```

Three lines of that are the three rules above. `posted #1` is answered
while all three subscriptions are open. `producers alive after one left:
2` is a dropped subscription stopping the work behind it, not just the
delivery. And `closed at #2` is a result carried out through a merge and
still attributable to the subscriber that received it. (Which of the two
subscribers the merge reports first is not fixed — there is no ordering
between distinct subscriptions, and the specification says so.)

```rust,ignore
{{#include ../examples/chat_room.rs}}
```

## Realisation

Nothing in the caller's type says whether a subscription got a lane of its
own or a tag on a lane it shares. On a session reporting
`LaneSupport::Many` the example opens one lane per subscription, for
independent flow control; on a session with one lane the same code
tag-multiplexes. That choice belongs to the session, and a caller that had
to branch on it would not have had the transport abstracted, only
reported.

Which is also why a channel is built over a **session** rather than over a
lane. A channel handed one transport can only ever tag-multiplex, so the
caller's construction would have made the realisation choice.
