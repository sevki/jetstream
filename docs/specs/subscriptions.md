# JetStream Subscriptions Specification

This document specifies **subscriptions**: a named, ordered, cancellable stream of messages produced by one peer for another. It is the layer above `r[jetstream.session.overview]`, and it exists because sessions and lanes answered "what can open another stream" without answering "how does an application receive many messages it did not individually ask for".

Requirements only. No implementation, and one wire-visible consequence, stated in `r[jetstream.subscription.compat]`.

## Why

The session model made transports interchangeable *underneath* the RPC layer. It did not make them interchangeable *above* it, and today the RPC layer cannot push at all:

- `Mux` is unary. `RpcCall` wraps a `oneshot::Receiver`, so a tag admits exactly one response, and the demultiplexer removes the tag's waiter on the first frame bearing it. A second frame with that tag, or any frame with a tag the client did not issue, has no waiter to receive it.
- `jetstream_rpc::server::run` is `while let Some(frame) = next() { send(rpc(frame)) }` — one response per request, and no path by which a service emits anything unprompted.

The consequence is the thing this specification exists to remove: on a `LaneSupport::Many` transport a service can push by opening a lane and being the caller on it, per `r[jetstream.session.symmetric]`; on a `LaneSupport::One` transport — a WebSocket, a TCP connection, stdio — **it cannot push at all**. An application that must work on both therefore writes two implementations, and "not tied to one transport" becomes a property of the transport layer that the application layer does not inherit.

`r[jetstream.rcp.multiplexing.selection]` already says a client on a many-lane transport SHOULD let the caller choose a strategy. This specification says the same thing about receiving, and requires that the choice not be visible in the application's code.

## Use cases

The target is an addressable, single-threaded, stateful object — a cell in celld, in the manner of a Durable Object — with clients attached over whichever transport they happen to have. What follows enumerates the shapes a subscription takes against it. They are not variations on one thing: each stresses a different requirement, and several would be satisfied by a design that fails the others.

This section is non-normative. It cites the rules; it does not add any.

### 1. Room fan-out

One producer, many subscribers, each seeing the same events in the order the producer assigned them. The smallest complete instance of the whole model, and the one every requirement below was written against:

| The room needs | Which is |
| --- | --- |
| Each subscriber sees messages in the order the room assigned them | `r[jetstream.subscription.ordering]` |
| A browser on a WebSocket and an edge node on iroh both work, from one implementation | `r[jetstream.subscription.realisation]` |
| One client in three rooms uses one connection | `r[jetstream.subscription.endpoint]` |
| A slow subscriber does not stall the others, and the room finds out | `r[jetstream.subscription.backpressure]` |
| Typing indicators are dropped under load rather than queued behind a large upload | `r[jetstream.subscription.lossy]` |
| A client that reconnects continues from the message it had seen | `r[jetstream.subscription.resume]` |
| The room may be evicted from memory while its subscribers stay attached | `r[jetstream.subscription.detached]` |

The room is the demanding case because it needs *all* of them at once. The shapes below each need a strict subset, which is what makes them useful for checking that no requirement is carrying weight it should not.

### 2. Presence, typing indicators, cursors

High frequency, latest-wins, worthless once stale. Loss is not a degradation but the correct behaviour: `r[jetstream.subscription.lossy]` and `r[jetstream.subscription.lossy.degradation]`. No ordering requirement beyond per-subscription, no resumption — a client that reconnects wants the *current* presence, not the presence it missed. This is the shape that would be actively harmed by a design treating every subscription as reliable.

### 3. Log and event tailing

One subscriber, unbounded, resumed from a cursor after arbitrary downtime, against a producer that retains a bounded window. This is the hardest test of `r[jetstream.subscription.resume.gap]`: the interesting case is not resuming successfully but the producer discovering it cannot, and having to say so rather than silently starting from the oldest record it still holds.

### 4. Progress on a long operation

A subscription whose lifetime is one operation's: an upload, a migration, a build. Bounded, and its *termination* carries the result — `r[jetstream.subscription.termination]` is the load-bearing rule, since "finished" and "the connection dropped" are different outcomes and a progress bar that cannot tell them apart is wrong in the case that matters. Cancellation, per `r[jetstream.subscription.cancel]`, is expected to cancel the operation and not merely stop reporting on it; that mapping is the application's to make, and it needs the cancellation to be observable at the producer.

### 5. State synchronisation

Snapshot followed by deltas — a replicated document, a materialised view, a cache. Resumption is not a convenience here but the entire point: a client that cannot resume must re-fetch the snapshot. Exercises `r[jetstream.subscription.resume]` together with `r[jetstream.subscription.detached]`, because the producer is idle between deltas and is exactly the kind of object a platform wants to evict.

### 6. Model output streaming

Tokens from an inference call. Unbounded until the producer stops or the subscriber does, and the two rules that matter are the ones a chat room barely exercises: `r[jetstream.subscription.cancel]`, because a user pressing stop must actually stop the work and not just the display, and `r[jetstream.subscription.backpressure]`, because a subscriber that cannot keep up must not be silently truncated — `r[jetstream.subscription.backpressure.reporting]`.

### 7. Cell to cell, in process

Two cells on one node, one subscribing to the other, with no transport between them. Exercises `r[jetstream.subscription.conformance.local]` and, through `r[jetstream.session.local.no-serialisation]`, the requirement that ordering not be obtained by encoding to bytes. When one cell later moves to another node the same subscription crosses a transport, and `r[jetstream.session.local.boundary]` decides whether that is allowed to weaken the ordering. This is the shape that makes the model worth having inside a single node rather than only between them.

### 8. Downstream push to a device

Notifications to a phone, cache invalidations, alerts. **This case already has a specified answer**: `r[jetstream.rpc.swift.handler]` describes a handler receiving upstream-initiated requests, which is the reverse-call shape, and it is a good fit where upstream can open a stream downstream and the messages are independent of any subscription the device holds. Subscriptions do not replace it — see `r[jetstream.subscription.rationale.coexists-with-push]` for which applies where.

## Overview

r[jetstream.subscription.overview]
A subscription is a **streaming response**: one request from the subscriber, many responses from the producer, sharing the request's tag, terminated explicitly. It is not a new channel type and not a second addressing scheme. Concretely:

1. A subscriber issues a request that the protocol declares to be streaming.
2. The producer emits zero or more items bearing that tag, for as long as the subscription lives.
3. The subscription ends by an explicit terminator from the producer, an explicit cancellation from the subscriber, or the termination of the lane carrying it.

r[jetstream.subscription.overview.additive]
This MUST NOT change the frame format, the meaning of `tag`, or the `ClientTransport`/`ServiceTransport` bounds. It does change the shape of the client and server RPC layers, which today assume one response per request; that is a source-level change to `jetstream_rpc`, and `r[jetstream.subscription.compat]` states what it may and may not break on the wire.

r[jetstream.subscription.overview.not-a-new-role]
A producer MUST NOT be *required* to open a lane, or to issue a request of its own, in order to deliver a subscription's items. Streaming the response reuses the direction that is already open, so the same application code works on `LaneSupport::One` and `LaneSupport::Many`, and a subscriber never has to become a server in order to receive. This constrains what an implementation may demand of a producer. It is not a claim that a reverse-call design cannot work, which `r[jetstream.subscription.rationale.reverse-call]` addresses on its merits.

## Subscriptions

r[jetstream.subscription.definition]
A subscription is identified by the tag of the request that created it, **on the lane that carried it**. It MUST remain in flight, in the sense of `r[jetstream.lane.tag-mux]`, for as long as the producer may still emit items: its tag MUST NOT be reused until the subscription has terminated. A subscription is therefore a long-lived call, not a sequence of short ones.

r[jetstream.subscription.identity]
The tag alone does not identify a subscription within a session, and this document has repeatedly been written as though it did. Tags are allocated per lane, so two concurrent subscriptions on distinct lanes of one session may hold the same numeric tag; and a tag released at termination may be rebound while an item of the previous binding is still in flight on an unordered channel.

Therefore: any mechanism that names a subscription from **outside its own lane** — the datagram channel, a cancellation not carried on that lane, any control path a future revision adds — MUST use a **binding identifier that is unique for the lifetime of the session and never reused**, not the tag and not an identifier recycled when a binding ends. A per-binding counter that only increases satisfies this; recycling does not, because `r[jetstream.session.datagrams]` gives no instant at which the transport is known to have drained, so "no longer live" is not a condition an implementation can test.

Within its own lane a subscription needs nothing further: the tag is unambiguous there, and `r[jetstream.lane.delivery-order]` orders what the lane carries.

r[jetstream.subscription.ordering]
Items of one subscription MUST be delivered in the order the producer emitted them. This follows from `r[jetstream.lane.delivery-order]` when the subscription is realised on a lane, and MUST be preserved by any other realisation. There is no ordering between distinct subscriptions, whether or not they share a lane, a session, or a producer — `r[jetstream.lane.no-cross-lane-order]` applies unchanged. An application requiring two event streams to be mutually ordered MUST make them one subscription.

r[jetstream.subscription.termination]
A subscription MUST end explicitly, and a normal end MUST be distinguishable from a failure. A producer that has no more items MUST say so rather than falling silent, and a subscriber MUST be able to tell "the room closed" from "the connection dropped" without inspecting the transport.

A protocol MAY give the normal end a typed payload, and where the subscription reports on an operation it SHOULD: use case 4 is an upload or a build whose *result* is the point, and a silent end forces the result to be sent as a final item — which is then indistinguishable from a final item followed by a connection that dropped before the terminator. Carrying the result in the terminator makes completion and result arrive together or not at all. A surface offering only an item type therefore cannot serve that use case, and `r[jetstream.subscription.surface.termination]` requires the distinction to be visible in the idiom.

r[jetstream.subscription.cancel]
A subscriber MUST be able to cancel a subscription, and cancellation MUST release the producer's obligation to it — *and* become observable to the producer itself. Releasing only the delivery obligation is not sufficient: an RPC layer that acknowledges a cancellation and discards what arrives afterwards, while the inference or upload or build carries on, satisfies the letter and defeats the purpose. Use cases 4 and 6 are the ones that care — a user pressing stop expects the work to stop, not the screen to — so an implementation MUST surface cancellation at the producer, whether by cancelling the source task or by making it observable in the producer surface `r[jetstream.subscription.surface.producer]` describes. Cancellation is a request of its own, bearing a **fresh tag** and naming the subscription's tag in its payload, in the manner of 9P's `Tflush` — which carries `oldtag` for exactly this reason. It MUST either travel on the subscription's own lane, where the target tag is unambiguous, or name the subscription by the binding identifier of `r[jetstream.subscription.identity]`; naming a bare tag from another lane is not sufficient, since a concurrent subscription elsewhere on the session may hold the same one. It MUST NOT be sent under the subscription's own tag: that tag is in flight for the subscription's life, per `r[jetstream.subscription.definition]`, so a second call bearing it would put two calls under one correlation key, contrary to `r[jetstream.lane.tag-mux]`. `Tflush` exists in the 9P protocols only; the JetStream protocol space has no such message today.

Wherever the *request* travels, the terminating *acknowledgement* MUST be delivered on the subscription's own lane, after every item the producer had already emitted there. Off-lane cancellation is otherwise unordered against those items: a subscriber could observe the acknowledgement, treat the subscription as ended, reuse its tag, and then receive a reliable item still in flight on the original lane — which would arrive under the new binding. `r[jetstream.subscription.identity]` does not rescue this case, because it governs naming from outside a lane and an in-lane item carries no binding identifier. Requiring the acknowledgement in-lane puts `r[jetstream.lane.delivery-order]` between the last item and the tag becoming free, which is the guarantee the identifier cannot supply here.

A fresh tag introduces a hazard of its own, and an implementation MUST NOT inherit it: cancellation MUST NOT depend on the ordinary tag pool. Long-lived subscriptions can occupy every allocatable tag, and a pool that makes an acquirer wait for a recycled one then deadlocks exactly when cancellation is most needed — the tag cannot be recycled until the subscription terminates, and the subscription cannot terminate without the cancellation. Control capacity MUST be reserved outside the pool, or the pool MUST admit a cancellation while saturated.

The same saturation starves ordinary calls, and that is a cost of correlating by tag rather than a defect to be waved away: each live subscription holds one tag for its whole life, so a client whose pool fills with subscriptions cannot issue a unary call at all — at `max_concurrent_requests = 1`, one room subscription blocks every `post` for its duration. An implementation MUST NOT let subscriptions exhaust the capacity available to ordinary calls; a quota, a reservation, or a pool that grows are all conforming. `r[jetstream.subscription.rationale.reverse-call]` prefers tag correlation with this cost counted against it.

The acknowledgement bounds **emission**, not arrival. After it the producer MUST NOT emit further items for that subscription; it cannot promise that none is still in flight, because an item carried out of band may outlive the acknowledgement that was carried in band. `r[jetstream.subscription.lossy.stale-item]` is what makes the boundary observable at the subscriber, by requiring a late item to be discarded rather than delivered.

r[jetstream.subscription.fanout]
A producer serving many subscribers MUST NOT be required to deliver an item to all of them before delivering the next item to any of them. Per-subscription order is the only order; a global delivery order across subscribers is not required and MUST NOT be assumed by an application. Where an application needs a total order over events, it MUST carry that order in the items themselves — a sequence assigned by the producer — rather than inferring it from delivery.

Freedom in the *order* of fan-out is not freedom in its *content*. A subscription that is not declared lossy MUST receive every item the producer emits for it. A subscriber falling behind is not licence to skip: an implementation either applies backpressure per `r[jetstream.subscription.backpressure]`, or terminates the subscription and reports it per `r[jetstream.subscription.backpressure.reporting]`. Omitting an item and carrying on is conforming only under `r[jetstream.subscription.lossy]`.

## Realisation

r[jetstream.subscription.realisation]
A subscription MUST be realisable on any transport JetStream supports, including `LaneSupport::One`. An implementation on a `LaneSupport::Many` session SHOULD place a subscription on its own lane, giving it independent flow control per `r[jetstream.lane.independence]`; on a `LaneSupport::One` session it shares the single lane with every other call, tag-multiplexed per `r[jetstream.lane.tag-mux]`.

r[jetstream.subscription.realisation.opaque]
Which realisation was chosen MUST NOT be visible in the type an application holds, and MUST NOT require the application to branch on `Capability::ManyLanes`. This is the requirement that makes the model transport-independent in practice rather than in principle: a capability the application must branch on has not been abstracted, only reported.

r[jetstream.subscription.realisation.selection]
An implementation on a `LaneSupport::Many` session SHOULD allow the realisation to be chosen per subscription, since the trade-off is the caller's: a lane per subscription costs a stream and buys independence under `r[jetstream.lane.independence]`, while sharing a lane spends that independence and buys nothing back.

In particular, sharing a lane does **not** buy ordering. Two subscriptions on one lane remain unordered with respect to each other, per `r[jetstream.subscription.ordering]`, and mutual ordering is obtained by making the events one subscription and by no other means. This is forced rather than chosen: `r[jetstream.subscription.realisation.opaque]` denies an application any way to learn which realisation it was given, so a guarantee that held on only one of them could not be relied upon even where it happened to hold.

r[jetstream.subscription.backpressure]
A subscriber that stops consuming MUST NOT stall the producer's other subscriptions. On a `LaneSupport::Many` realisation this follows from `r[jetstream.lane.independence]`; on a shared lane it does not, and an implementation MUST bound what it buffers for a slow subscription rather than allowing it to consume the lane's window.

r[jetstream.subscription.backpressure.reporting]
When an implementation drops items, or terminates a subscription, because a subscriber is not keeping up, the producer MUST learn of it. A subscription that silently loses items is indistinguishable to the application from one that is merely quiet, and `r[jetstream.subscription.ordering]` becomes unverifiable. This is the reliable case; the deliberately lossy case is `r[jetstream.subscription.lossy]`.

## Addressing

r[jetstream.subscription.endpoint]
A subscription names an **endpoint** within the peer, not merely the peer. A session is an association with a node; an application-level producer — a room, an object, a cell — is addressed within it. Establishing a session MUST NOT be required per endpoint: a subscriber attached to several endpoints on one peer MUST be able to do so over one session.

r[jetstream.lane.addressing]
An endpoint MUST be addressable on every session, whatever its lane support, and the mechanism MUST NOT be the one the application chooses between. The identifier MUST have a wire type this specification fixes rather than one each protocol invents, since codegen generates the client that carries it and cannot know an application's own naming: an opaque byte string, sized as the wire format's `Data` already is, with any structure being the application's to impose and no implementation's to interpret. Where a lane is dedicated to one endpoint, the lane MUST be able to carry that endpoint's identifier before any service frame, alongside the version negotiation that `r[jetstream.session.version-scope]` already scopes per lane. Where the lane is shared — always on `LaneSupport::One`, and whenever a `LaneSupport::Many` session tag-realises a subscription — the endpoint MUST instead be carried by the request that opens the subscription.

Both are required, because `r[jetstream.subscription.realisation.opaque]` forbids the application from knowing which it got. A specification offering only the dedicated-lane form would make `r[jetstream.subscription.endpoint]` — one client, three rooms, one connection — unimplementable on exactly the transports that motivate it. Leaving either unspecified is what forces every transport binding and every application to invent its own convention.

r[jetstream.subscription.endpoint.identity]
The endpoint identifier is an application-level name and MUST NOT be conflated with the transport peer identity of `r[jetstream.session.identity]`. The two answer different questions — which object, and which principal — and a producer generally needs both: a chat room must know which room it is and which user is attached.

## Lossy subscriptions

r[jetstream.subscription.lossy]
A subscription MAY be declared **lossy**: its items may be dropped, and their loss is acceptable to the application. Presence, typing indicators, and cursor position are the motivating cases. A lossy subscription SHOULD be carried on the session's datagram channel where one exists, per `r[jetstream.session.datagrams]`.

r[jetstream.subscription.lossy.stale-item]
An item carried out of band can outlive the termination carried in band. A
datagram emitted before a subscription's terminator or its cancellation
acknowledgement MAY arrive after it, because `r[jetstream.session.datagrams]`
orders datagrams neither among themselves nor against a lane. Once the tag has
been reused — which `r[jetstream.subscription.definition]` permits after
termination — that stale datagram is indistinguishable, by tag alone, from an
item of the new call.

An implementation MUST NOT deliver such an item as belonging to a later
subscription, nor to a *concurrent* one. The tag establishes neither: tags are
allocated per lane, so two subscriptions on distinct lanes of one session may
hold the same numeric tag at the same time, while the datagram channel is a
single session-wide queue whose frames carry no lane identity. A discriminator
that merely changed on rebind would leave those two indistinguishable and
deliver an item to the wrong stream.

So a datagram realisation MUST carry the binding identifier of
`r[jetstream.subscription.identity]` — unique for the session's lifetime and
never reused, which covers the concurrent case and the rebound one together —
and MUST discard an item whose identifier does not match the binding it would be
delivered to. Withholding the
tag from the pool until no in-flight datagram can still bear it is not an
alternative: no transport reports that instant, so the condition is not
decidable.

r[jetstream.subscription.lossy.ordering]
A discriminator that changes when a tag is rebound distinguishes *bindings*; it does not order items within one. Two items of the same lossy subscription reordered by the datagram transport both carry the current discriminator, so both would be accepted, and delivering them as they arrive would break `r[jetstream.subscription.ordering]` — which admits no exception for a lossy realisation.

Waiting for the earlier one is not available either: on a channel that may drop, it may never arrive. So a lossy realisation MUST carry a monotonically comparable per-item sequence, and MUST discard an item that is not newer than the newest already delivered on that subscription. The subscriber therefore never observes an inversion, at the cost of the older item — which is what a lossy subscription asked for. This is latest-wins made normative rather than left to the transport, and it is how `r[jetstream.subscription.ordering]` is satisfied on an unordered channel.

r[jetstream.subscription.lossy.degradation]
Where the session reports no datagram channel, a lossy subscription MUST degrade by **dropping**, not by queueing. This refines `r[jetstream.session.capabilities.degradation]`, which requires an absent capability to fail explicitly rather than be emulated: for a lossy subscription, dropping is not an emulation of datagrams but the same delivery contract met by other means, and it is the behaviour the application asked for. Carrying presence reliably is the wrong answer — a typing indicator queued behind a large upload arrives after the message it was announcing.

r[jetstream.subscription.lossy.declared]
Lossiness MUST be a property of the subscription declared by the protocol, not a runtime decision by the transport. A transport MUST NOT decide on its own that a reliable subscription may be dropped under load; that case is `r[jetstream.subscription.backpressure.reporting]`.

## Resumption

r[jetstream.session.resumption]
Whether a session can be resumed after its connection is lost MUST be reported as a capability, distinct from `Migration`.

r[jetstream.session.resumption.distinct-from-migration]
`Migration` means the association survives a change of network path with the connection intact — a QUIC endpoint changing address. Resumption means the connection was lost and a new one continues the same association. A transport may have either, both, or neither, and an application needs them separately: migration is invisible to it, resumption requires it to participate.

r[jetstream.subscription.resume]
A subscriber MUST be able to request that a subscription resume from a position it supplies, rather than from the beginning or from the present. The position is application-defined — only the application knows what an item is — and the producer MUST either resume from it or report that it cannot, per `r[jetstream.subscription.resume.gap]`.

r[jetstream.subscription.resume.gap]
A producer that cannot satisfy a requested position — because the items are no longer retained, or the position is unrecognised — MUST report the gap explicitly. It MUST NOT silently resume from the oldest item it still holds, nor from the present: both present a discontinuous stream as a continuous one, which is `r[jetstream.subscription.ordering]` violated in the one place an application cannot detect it.

r[jetstream.subscription.resume.scope]
Resumption is per subscription, not per session. A resumed session MUST NOT be assumed to carry its previous subscriptions; each is re-established with its own position.

## Detached producers

r[jetstream.subscription.detached]
A producer MAY be evicted between items while its subscriptions remain established, and reconstructed when there is work for it. This is the platform property the model exists to serve: an idle room should not occupy memory, and its subscribers should not notice.

r[jetstream.subscription.detached.state]
Where a producer may be evicted, a subscription's state MUST be reconstructible without it. An implementation MUST NOT require a subscription to hold a reference into a live producer, since that reference is what eviction invalidates. What a subscription retains across eviction MUST be an explicit, serialisable value.

r[jetstream.subscription.detached.transparency]
Eviction and reconstruction of a producer MUST NOT be observable to a subscriber as a subscription failure, a gap, or a reordering. A subscriber that cannot tell is what makes eviction a platform decision rather than an application concern.

## Surface

r[jetstream.subscription.surface.declared]
Whether a method is unary, streaming, or lossy MUST be declared in the service definition, not chosen at the call site. Codegen reads that declaration and emits the corresponding surface, so a protocol has one answer in every target language rather than one per runtime. The rules in this section are stated per language runtime because that is where they bind, in the manner of `r[jetstream.rpc.swift.mux]` and `r[jetstream.rpc.ts.mux]`; only the Rust surface is specified here, and a target language adopting subscriptions states its own against these requirements.

r[jetstream.subscription.surface.session]
A client offering subscriptions MUST be constructible from a session together with the endpoint it addresses, not only from a single lane. A session is an association with a peer that may host many endpoints, so a client given only the session cannot say which of them a subscription is for; the endpoint then reaches the wire by whichever means `r[jetstream.lane.addressing]` provides for the realisation actually chosen, which the application does not see. `ClientTransport` is a frame sink and stream; it cannot open a lane, so a client holding one has no realisation to choose between and the choice falls to whoever constructed it. That is `r[jetstream.subscription.realisation.opaque]` violated at the constructor rather than at the call — the application branches on the transport before it ever names a subscription.

Constructing a client from a single transport remains valid and MUST keep working, per `r[jetstream.subscription.compat.existing-clients]`; such a client realises every subscription as a tag, which is the `LaneSupport::One` behaviour on any transport.

r[jetstream.subscription.surface]
A subscription MUST be presented as the target language's idiomatic asynchronous sequence, not as a callback registration or a polling method. A caller that already knows how to consume a stream in its language MUST NOT have to learn a JetStream-specific shape to consume a subscription.

r[jetstream.subscription.surface.cancellation]
Cancellation MUST be bound to the language's own cancellation mechanism, so that abandoning a subscription in the ordinary way of the language satisfies `r[jetstream.subscription.cancel]`. A subscription that leaks because the caller returned early, or that requires an explicit teardown call the language does not otherwise need, does not conform.

r[jetstream.subscription.surface.terminal-value]
Where `r[jetstream.subscription.termination]` gives the normal end a payload, the surface MUST be able to deliver it. A type that is only the language's stream of items cannot: Rust's `Stream` ends with `None`, and there is nowhere for a result to go. Writing the usage is what shows this — a `Subscription<Event>` offers no `result()` and cannot be given one without changing what the type is.

An implementation MUST therefore either parameterise the subscription by its terminal type, or carry the end as a variant of what the sequence yields. Either is conforming; offering neither is not, and the choice is visible to callers so a target language states which it took.

r[jetstream.subscription.surface.composition]
Combining subscriptions MUST NOT erase which one ended, or what it ended with. The idiomatic merge in most languages — Rust's `select_all`, and its equivalents — yields items and silently drops a sequence when it finishes, so a caller merging several subscriptions can observe every item and no terminator. Use case 2 and use case 4 are each satisfiable alone and unsatisfiable together under such a merge.

This is a requirement on the surface, not on the caller: a subscription's terminal value MUST survive whatever combinator the language offers for merging, which in practice means the end is a value in the sequence rather than the absence of one.

r[jetstream.subscription.surface.termination]
The three outcomes of `r[jetstream.subscription.termination]` — the producer finished, the producer failed, the subscription could not be resumed — MUST be distinguishable in the idiom, and a gap MUST NOT be presented as a normal end. In languages where iteration ends silently, the failure cases MUST surface through that language's error channel rather than as an absent item.

r[jetstream.subscription.surface.producer]
The producer surface MUST carry cancellation to the producer, and MUST admit a source driven by a cursor rather than only a live sender held in memory.

Cancellation first, because it is the one the usage exposes: `r[jetstream.subscription.cancel]` requires that stopping a subscription stops the work, and a surface that hands the producer only a way to *send* gives it no way to *learn*. A producer loop written against such a surface compiles, runs, and continues its inference or upload for as long as the process lives. Whatever the surface hands a producer MUST therefore expose cancellation — as a signal it can await, a flag it can test, or a send that fails once cancelled — and a target language states which. A generated handler that can express a subscription *only* as "hold this sender and write to it" cannot satisfy `r[jetstream.subscription.detached.state]`, because the sender is precisely what eviction invalidates.

The remainder of this section is non-normative: a worked producer and consumer for the room of use case 1. `Seq` is the room's sequence number and `Event` its message type; both are ordinary generated wire types.

### Rust

r[jetstream.subscription.surface.rust]
Rust presents a subscription as a `Stream`, cancelled by dropping it. The `#[service]` macro gains attributes for the declaration required by `r[jetstream.subscription.surface.declared]`. Rust is the reference implementation, so this is the surface the other runtimes' own specifications are written against.

```rust
#[service]
pub trait Room {
    /// Unary. Unchanged, and MUST stay unchanged — r[jetstream.subscription.compat.existing-clients].
    async fn post(&self, ctx: Context, body: String) -> Result<Seq>;

    /// Streaming. Many `Event`s share the request's tag.
    #[subscription]
    fn events(&self, ctx: Context, from: Seq) -> Subscription<Event>;

    /// Lossy. Datagrams where the session has them, dropped where it does not.
    #[subscription(lossy)]
    fn presence(&self, ctx: Context) -> Subscription<Presence>;
}
```

Consuming it is consuming a stream. Nothing in the signature says whether the
subscription got its own lane or a tag on a shared one — that is
`r[jetstream.subscription.realisation.opaque]`:

```rust
// Built from the *session*, not from a lane. A channel handed a single
// transport can only ever tag-multiplex, which would make the caller's
// construction choice the realisation choice — the branch that
// r[jetstream.subscription.realisation.opaque] forbids.
let room = RoomChannel::over(session.clone(), RoomId("room-42"));
let mut events = room.events(Context::default(), Seq(412));

while let Some(item) = events.next().await {
    match item? {
        // The end is a value in the sequence, not the absence of one, so it
        // survives `select_all` and can carry a result —
        // r[jetstream.subscription.surface.terminal-value] and
        // r[jetstream.subscription.surface.composition].
        Item::Next(event) => render(event),
        Item::Done(summary) => return Ok(summary),
    }
}
// Dropping `events` cancels — r[jetstream.subscription.surface.cancellation].
```

Two producer shapes, and the difference is the whole of
`r[jetstream.subscription.detached.state]`. Note what the second does *not* do:
it captures no reference into the room. A closure borrowing `self.log` would
read as the cursor form while still holding exactly the state eviction
invalidates, and Rust's lifetimes make that the harder version to write rather
than the easier one — the language agreeing with the rule.

```rust
impl Room for ChatRoom {
    // Resident producer: holds the sender, writes as events happen.
    fn events(&self, _ctx: Context, from: Seq) -> Subscription<Event> {
        let (tx, sub) = Subscription::channel(64);
        self.subscribers.attach(from, tx);
        sub
    }
}

impl Room for EvictableRoom {
    // Evictable producer: holds nothing *and captures nothing*. `log` is an
    // owned locator, not a borrow of the room, so the platform can re-drive
    // the cursor once the room is gone and the subscriber cannot tell —
    // r[jetstream.subscription.detached.state], then
    // r[jetstream.subscription.detached.transparency].
    fn events(&self, _ctx: Context, from: Seq) -> Subscription<Event> {
        let log = self.log_id.clone();
        Subscription::from_cursor(from, move |cursor| {
            let log = log.clone();
            async move { LogStore::open(&log).read_after(cursor).await }
        })
    }
}
```

## Patterns

Non-normative. How subscriptions compose in Rust, and which rule each
composition depends on. These are where the ordering rules earn their keep:
most of the mistakes available here are ordering mistakes that type-check.

### Fan-out — one producer, many subscribers

Use case 1, shown above: each subscriber gets every item, in the order the
producer emitted them, and `r[jetstream.subscription.fanout]` deliberately does
not require the producer to deliver an item to all subscribers before moving on.
A subscriber that stalls therefore does not stall the room, which is
`r[jetstream.subscription.backpressure]`.

### Fan-in — many producers, one consumer

A client attached to several rooms merges their subscriptions. The rule that
shapes the code is `r[jetstream.subscription.ordering]`: there is **no order
between subscriptions**, so the merge must carry the source and the consumer
must not read the interleaving as meaningful.

```rust
use futures::stream::{select_all, StreamExt};

let mut inbox = select_all(rooms.iter().map(|room| {
    let id = room.id();
    room.events(Context::default(), cursor_for(id)).map(move |item| (id, item))
}));

while let Some((room_id, item)) = inbox.next().await {
    match item {
        Ok(event) => render(room_id, event),
        // One room failing is not all of them. Tearing down the merge on the
        // first error would make every room as available as the worst.
        Err(err) => drop_room(room_id, err),
    }
}
```

Whether those rooms live on one peer or several does not change this code.
That is `r[jetstream.subscription.endpoint]` paying off: several endpoints on
one peer are several subscriptions on **one** session, not one session each.

### Many writers, one order

The write side of a room, and the place where the ordering rules are easiest to
get wrong. The room assigns the order; the writers do not have one between them.
Two posts from the *same* client are ordered with respect to each other only if
they share a lane — `r[jetstream.lane.no-cross-lane-order]`.

```rust
// A lane kept for the session: posts arrive at the room in issue order.
let ordered = Session::<RoomChannel>::open_lane(&session).await?;
let room = RoomChannel::new(4, Box::new(ordered));
room.post(ctx.clone(), "first".into()).await?;
room.post(ctx.clone(), "second".into()).await?;

// A lane per call: independent, no head-of-line blocking, and no order.
// Correct for posts to *different* rooms, wrong for two posts to one.
let posts = messages.into_iter().map(|body| async {
    let lane = Session::<RoomChannel>::open_lane(&session).await?;
    RoomChannel::new(1, Box::new(lane)).post(Context::default(), body).await
});
```

`r[jetstream.rcp.multiplexing.selection]` is exactly this choice, and it belongs
to the caller because only the caller knows whether these two writes are related.

### Gossip — every peer both roles

A mesh where each peer subscribes to its neighbours and serves theirs.
`r[jetstream.session.symmetric]` is what makes it expressible: a session is not
fixed to a role, so one session per neighbour carries both directions.

```rust
async fn run_peer(me: PeerId, neighbours: Vec<GossipChannel>) -> Result<()> {
    let mut rumours = select_all(
        neighbours.iter().map(|n| n.rumours(Context::default(), Seq(0))),
    );

    while let Some(item) = rumours.next().await {
        let rumour = item?;
        // Seen-set, not ordering: gossip converges without a total order,
        // which is why r[jetstream.subscription.ordering] being per-subscription
        // costs nothing here.
        if me.seen.insert(rumour.id) {
            me.log.append(rumour).await?;   // this peer's own subscribers follow
        }
    }
    Ok(())
}
```

Two properties of the session model do real work in a mesh. Membership is a list
of identities and needs no locators, because iroh reports `IdentityKind::Key` and
`r[jetstream.session.identity.addressing]` makes that a capability rather than a
detail. And liveness heartbeats are `r[jetstream.subscription.lossy]` — a
heartbeat queued behind a large rumour has already told a lie by the time it
arrives.

### Pipeline — subscribe, transform, republish

A cell that consumes one subscription and produces another. The cursor source is
what makes this composable, per `r[jetstream.subscription.surface.producer]`:

```rust
fn summaries(&self, ctx: Context, from: Seq) -> Subscription<Summary> {
    // Owned upstream locator, not a borrow of this cell — the same
    // constraint as the room, and for the same reason. A middle cell that
    // captured `self.upstream` would not survive its own eviction.
    let upstream = self.upstream_id.clone();
    Subscription::from_cursor(from, move |cursor| {
        let (upstream, ctx) = (upstream.clone(), ctx.clone());
        async move {
            let batch = Upstream::open(&upstream).read_after(ctx, cursor).await?;
            Ok(batch.map(summarise))
        }
    })
}
```

Backpressure propagates the length of the chain for free: the next upstream read
happens only when the previous batch has been consumed, so a slow final consumer
slows every hop rather than accumulating between them. A live-sender pipeline
would instead hold the upstream subscription open and buffer, and would not
survive the middle cell being evicted — `r[jetstream.subscription.detached.state]`.

When one hop is in-process and the next is remote, `r[jetstream.session.local.boundary]`
decides whether the ordering survives the transition, and requires the weakening
to be reported if it does not.

### Competing consumers — what this is *not*

Two consumers on one subscription do **not** split the items between them.
`r[jetstream.subscription.fanout]` gives each subscriber every item, and
`r[jetstream.subscription.rationale.not-a-queue]` declines to offer per-item
acknowledgement, so nothing in the model decides who owns an item.

A work queue is built *on* subscriptions rather than *out of* them: the producer
partitions or leases, and the acknowledgement is an ordinary unary call.

```rust
// The producer decides who gets what; the subscription only carries it.
let mut work = queue.claims(Context::default(), Worker(id), Lease::secs(30));

while let Some(item) = work.next().await {
    let job = item?;
    process(&job).await?;
    queue.ack(Context::default(), job.id).await?;   // unary, not a subscription
}
```

Stating this is the point of `r[jetstream.subscription.rationale.not-a-queue]`:
the boundary is deliberate, and an application that needs at-least-once delivery
builds it on the items, as it must anyway across a producer that may be evicted.

## Conformance

r[jetstream.subscription.conformance]
Per transport, given the rows of `r[jetstream.session.conformance]`:

| Transport | Reliable subscription | Lossy subscription |
| --- | --- | --- |
| iroh | Lane per subscription | Datagram |
| QUIC | Lane per subscription | Datagram |
| WebTransport (H3) | Lane per subscription | Datagram |
| TCP / TLS / unix / stdio | Tag on the single lane | Dropped on the single lane |
| In-process | Lane per subscription | Dropped |

**Subscription** resumption, `r[jetstream.subscription.resume]`, is not a column because it does not vary: re-subscribing from a position is an ordinary request, and it can be issued over a freshly established single-lane session exactly as over a new QUIC one. Requiring otherwise would leak the transport choice into state-synchronisation logic that has no business knowing it. **Session** resumption is not a column either, for the different reason below.

r[jetstream.session.resumption.mechanism]
`r[jetstream.session.resumption]` requires the capability to be reported and says nothing about how an association is continued, and nothing in the session model supplies it: `Capabilities` carries migration, which is a path change with the connection intact, and no reconnect token or state-transfer handshake is defined anywhere. A per-transport table of this capability would therefore assert values that can be neither implemented nor tested, and would be wrong in both directions — a lost QUIC connection takes its streams and its session with it, while an application-level handshake sufficient to continue an association over QUIC would serve equally over TLS. Until the mechanism is specified, a session MUST NOT report the capability.

r[jetstream.subscription.conformance.single-lane]
A `LaneSupport::One` transport MUST support subscriptions, by the same means and behind the same surface as every other row. It is the row that motivates the whole specification: if reaching a browser needs a second application implementation, the session model's transport independence stops at the RPC boundary. The requirement is that the row be reachable *uniformly*, not that only one shape can reach it.

r[jetstream.subscription.conformance.local]
An in-process session MUST realise subscriptions identically to a transport-backed one, per `r[jetstream.session.local]`, and MUST NOT serialise items to obtain ordering, per `r[jetstream.session.local.no-serialisation]`. Where an in-process subscription crosses to a remote peer, `r[jetstream.session.local.boundary]` applies: the ordering MUST continue across the boundary or the weakening MUST be reported.

## Compatibility

r[jetstream.subscription.compat]
Subscriptions MUST NOT change the frame format or the meaning of the `tag` field. A tag continues to identify one call; a streaming call simply remains in flight longer. The wire-visible consequence is that more than one response frame may bear a tag, which a peer that never issues a streaming request will never observe, and a peer that issues one has opted into.

r[jetstream.subscription.compat.existing-clients]
A client built before this specification issues only unary calls and MUST continue to work unchanged, per `r[jetstream.session.compat.existing-clients]`. A server MUST NOT stream a response to a request the protocol declares unary.

r[jetstream.subscription.compat.rpc-layer]
The client and server RPC layers assume one response per request today, and neither can express this without change. The change is source-level, and it is **not symmetric**.

The server side can be additive: a streaming entry point alongside `Server::rpc`, with a unary default, leaves every existing implementation compiling untouched, and existing unary servers MUST remain source-compatible. The client side is the forced break: `RpcCall` publicly resolves to exactly one frame, so callers naming that type cannot be preserved. Neither side may require a protocol to be re-generated for its unary methods.

## Rationale

r[jetstream.subscription.rationale.reverse-call]
The alternative is to make the producer a caller: the room issues a request to the subscriber for each item, which `r[jetstream.session.symmetric]` already permits.

**It is not rejected as unworkable, and two earlier drafts of this rule were wrong to say so.** The first claimed reverse calls could not reach the `LaneSupport::One` row, because both peers issuing into one tag space would need it partitioned by role; in fact requests and responses take disjoint type bytes — `102 + 2i` against `103 + 2i` — so the type already carries the direction and each peer keeps its own allocator. The second claimed reverse calls carry no identity; in fact a subscribe request can return an application-level subscription id that every item, terminator, cancellation and resume then carries, and per-item completion supplies a backpressure boundary. Both premises were false, and each was defending the conclusion this document reaches anyway. That is the failure mode worth naming: a conclusion propped up by a bad argument is indistinguishable from a sound one under every consistency check the document can run on itself.

Streaming the response is therefore a **preference with reasons**, not an impossibility result:

- **The correlation key comes from the protocol rather than being reinvented in each one.** A streaming call's tag *is* the subscription, for as long as it is in flight. A payload-level id is defined per protocol, which puts it beyond codegen's reach and leaves `r[jetstream.subscription.surface.declared]` — one answer in every target language — for each protocol to arrange itself.
- **No round trip per item.** Making every item an acknowledged call is a stronger contract than `r[jetstream.subscription.rationale.not-a-queue]` offers, and costs an outstanding call per item per subscriber. A room fanning out pays it on every message.
- **The subscriber stays a client.** Reverse calls oblige it to implement a handler and dispatch, in the shape `r[jetstream.rpc.swift.handler]` describes. Streaming responses let a peer that only consumes remain a pure consumer.

An implementation MAY offer reverse-call push as well; `r[jetstream.subscription.rationale.coexists-with-push]` says where each fits. What it MUST NOT do is present one shape's guarantees as the other's.

r[jetstream.subscription.rationale.coexists-with-push]
Subscriptions do not supersede the reverse-call push that `r[jetstream.rpc.swift.handler]` already specifies, and the two answer different questions. A reverse call is right when the producer has something to say that no subscriber asked for and that is complete in itself — a notification, a cache invalidation — and when the transport lets the producer open a stream downstream. A subscription is right when the subscriber asked for the items, and when they form one sequence with a beginning the subscriber chose, an end it can observe, and a handle it can cancel or resume.

An implementation MAY offer both. What it MUST NOT do is present bare reverse-call push as satisfying `r[jetstream.subscription.conformance.single-lane]`: that row is satisfied by providing the subscription contract, not by being able to deliver messages. A reverse-call design that carries its own correlation id and honours termination, cancellation, resumption and backpressure does satisfy it — `r[jetstream.subscription.rationale.reverse-call]` explains why this document prefers the other shape, and the preference is not a conformance requirement.

r[jetstream.subscription.rationale.not-a-queue]
A subscription is not a message queue and this specification does not require durability, at-least-once delivery, or acknowledgement of individual items. `r[jetstream.subscription.resume]` is the whole of what is offered, and it is deliberately weaker: the producer decides what it retains, and says so when it cannot meet a request. An application needing stronger guarantees builds them on the items, as it must anyway across a producer that may be evicted.
