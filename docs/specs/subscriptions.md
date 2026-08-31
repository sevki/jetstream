# JetStream Subscriptions Specification

This document specifies **subscriptions**: a named, ordered, cancellable stream of messages produced by one peer for another. It is the layer above `r[jetstream.session.overview]`, and it exists because sessions and lanes answered "what can open another stream" without answering "how does an application receive many messages it did not individually ask for".

Requirements only. No implementation, and one wire-visible consequence, stated in `r[jetstream.subscription.compat]`.

## Why

The session model made transports interchangeable *underneath* the RPC layer. It did not make them interchangeable *above* it, and today the RPC layer cannot push at all:

- `Mux` is unary. `RpcCall` wraps a `oneshot::Receiver`, so a tag admits exactly one response, and the demultiplexer removes the tag's waiter on the first frame bearing it. A second frame with that tag, or any frame with a tag the client did not issue, has no waiter to receive it.
- `jetstream_rpc::server::run` is `while let Some(frame) = next() { send(rpc(frame)) }` — one response per request, and no path by which a service emits anything unprompted.

The consequence is the thing this specification exists to remove: on a `LaneSupport::Many` transport a service can push by opening a lane and being the caller on it, per `r[jetstream.session.symmetric]`; on a `LaneSupport::One` transport — a WebSocket, a TCP connection, stdio — **it cannot push at all**. An application that must work on both therefore writes two implementations, and "not tied to one transport" becomes a property of the transport layer that the application layer does not inherit.

`r[jetstream.rcp.multiplexing.selection]` already says a client on a many-lane transport SHOULD let the caller choose a strategy. This specification says the same thing about receiving, and requires that the choice not be visible in the application's code.

### The motivating shape

The target is an addressable, single-threaded, stateful object — a cell in celld, in the manner of a Durable Object — with clients attached to it over whichever transport they happen to have. A chat room is the smallest complete instance of it, and it exercises every requirement below:

| The room needs | Which is |
| --- | --- |
| Each subscriber sees messages in the order the room assigned them | `r[jetstream.subscription.ordering]` |
| A browser on a WebSocket and an edge node on iroh both work, from one implementation | `r[jetstream.subscription.realisation]` |
| One client in three rooms uses one connection | `r[jetstream.subscription.endpoint]` |
| A slow subscriber does not stall the others, and the room finds out | `r[jetstream.subscription.backpressure]` |
| Typing indicators are dropped under load rather than queued behind a large upload | `r[jetstream.subscription.lossy]` |
| A client that reconnects continues from the message it had seen | `r[jetstream.subscription.resume]` |
| The room may be evicted from memory while its subscribers stay attached | `r[jetstream.subscription.detached]` |

## Overview

r[jetstream.subscription.overview]
A subscription is a **streaming response**: one request from the subscriber, many responses from the producer, sharing the request's tag, terminated explicitly. It is not a new channel type and not a second addressing scheme. Concretely:

1. A subscriber issues a request that the protocol declares to be streaming.
2. The producer emits zero or more items bearing that tag, for as long as the subscription lives.
3. The subscription ends by an explicit terminator from the producer, an explicit cancellation from the subscriber, or the termination of the lane carrying it.

r[jetstream.subscription.overview.additive]
This MUST NOT change the frame format, the meaning of `tag`, or the `ClientTransport`/`ServiceTransport` bounds. It does change the shape of the client and server RPC layers, which today assume one response per request; that is a source-level change to `jetstream_rpc`, and `r[jetstream.subscription.compat]` states what it may and may not break on the wire.

r[jetstream.subscription.overview.not-a-new-role]
A producer MUST NOT be required to open a lane, or to issue a request of its own, in order to deliver a subscription's items. Streaming the response reuses the direction that is already open, which is why the same application code works on `LaneSupport::One` and `LaneSupport::Many`. Making the producer the caller — a genuine reverse request — is the rejected alternative; see `r[jetstream.subscription.rationale.reverse-call]`.

## Subscriptions

r[jetstream.subscription.definition]
A subscription is identified by the tag of the request that created it, on the lane that carried it. It MUST remain in flight, in the sense of `r[jetstream.lane.tag-mux]`, for as long as the producer may still emit items: its tag MUST NOT be reused until the subscription has terminated. A subscription is therefore a long-lived call, not a sequence of short ones.

r[jetstream.subscription.ordering]
Items of one subscription MUST be delivered in the order the producer emitted them. This follows from `r[jetstream.lane.delivery-order]` when the subscription is realised on a lane, and MUST be preserved by any other realisation. There is no ordering between distinct subscriptions, whether or not they share a lane, a session, or a producer — `r[jetstream.lane.no-cross-lane-order]` applies unchanged. An application requiring two event streams to be mutually ordered MUST make them one subscription.

r[jetstream.subscription.termination]
A subscription MUST end explicitly, and a normal end MUST be distinguishable from a failure. A producer that has no more items MUST say so rather than falling silent, and a subscriber MUST be able to tell "the room closed" from "the connection dropped" without inspecting the transport.

r[jetstream.subscription.cancel]
A subscriber MUST be able to cancel a subscription, and cancellation MUST release the producer's obligation to it. Cancellation is a request bearing the subscription's tag, in the manner of `Tflush` — which the 9P protocols define and the JetStream protocol space does not, so it is new here. A producer MAY emit items that were already in flight when the cancellation was issued; a subscriber MUST tolerate them, and the cancellation's acknowledgement is the point after which no further item bearing that tag may arrive.

r[jetstream.subscription.fanout]
A producer serving many subscribers MUST NOT be required to deliver an item to all of them before delivering the next item to any of them. Per-subscription order is the only order; a global delivery order across subscribers is not required and MUST NOT be assumed by an application. Where an application needs a total order over events, it MUST carry that order in the items themselves — a sequence assigned by the producer — rather than inferring it from delivery.

## Realisation

r[jetstream.subscription.realisation]
A subscription MUST be realisable on any transport JetStream supports, including `LaneSupport::One`. An implementation on a `LaneSupport::Many` session SHOULD place a subscription on its own lane, giving it independent flow control per `r[jetstream.lane.independence]`; on a `LaneSupport::One` session it shares the single lane with every other call, tag-multiplexed per `r[jetstream.lane.tag-mux]`.

r[jetstream.subscription.realisation.opaque]
Which realisation was chosen MUST NOT be visible in the type an application holds, and MUST NOT require the application to branch on `Capability::ManyLanes`. This is the requirement that makes the model transport-independent in practice rather than in principle: a capability the application must branch on has not been abstracted, only reported.

r[jetstream.subscription.realisation.selection]
An implementation on a `LaneSupport::Many` session SHOULD allow the realisation to be chosen per subscription, since the trade-off is the caller's: a lane per subscription costs a stream and buys independence, and subscriptions that must be mutually ordered MUST share a lane, per `r[jetstream.subscription.ordering]`.

r[jetstream.subscription.backpressure]
A subscriber that stops consuming MUST NOT stall the producer's other subscriptions. On a `LaneSupport::Many` realisation this follows from `r[jetstream.lane.independence]`; on a shared lane it does not, and an implementation MUST bound what it buffers for a slow subscription rather than allowing it to consume the lane's window.

r[jetstream.subscription.backpressure.reporting]
When an implementation drops items, or terminates a subscription, because a subscriber is not keeping up, the producer MUST learn of it. A subscription that silently loses items is indistinguishable to the application from one that is merely quiet, and `r[jetstream.subscription.ordering]` becomes unverifiable. This is the reliable case; the deliberately lossy case is `r[jetstream.subscription.lossy]`.

## Addressing

r[jetstream.subscription.endpoint]
A subscription names an **endpoint** within the peer, not merely the peer. A session is an association with a node; an application-level producer — a room, an object, a cell — is addressed within it. Establishing a session MUST NOT be required per endpoint: a subscriber attached to several endpoints on one peer MUST be able to do so over one session.

r[jetstream.lane.addressing]
Where a lane is dedicated to one endpoint, the lane MUST be able to carry that endpoint's identifier before any service frame, alongside the version negotiation that `r[jetstream.session.version-scope]` already scopes per lane. Leaving this unspecified is what forces every transport binding and every application to invent its own first-frame convention.

r[jetstream.subscription.endpoint.identity]
The endpoint identifier is an application-level name and MUST NOT be conflated with the transport peer identity of `r[jetstream.session.identity]`. The two answer different questions — which object, and which principal — and a producer generally needs both: a chat room must know which room it is and which user is attached.

## Lossy subscriptions

r[jetstream.subscription.lossy]
A subscription MAY be declared **lossy**: its items may be dropped, and their loss is acceptable to the application. Presence, typing indicators, and cursor position are the motivating cases. A lossy subscription SHOULD be carried on the session's datagram channel where one exists, per `r[jetstream.session.datagrams]`.

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

## Conformance

r[jetstream.subscription.conformance]
Per transport, given the rows of `r[jetstream.session.conformance]`:

| Transport | Reliable subscription | Lossy subscription | Resumption |
| --- | --- | --- | --- |
| iroh | Lane per subscription | Datagram | Yes |
| QUIC | Lane per subscription | Datagram | Yes |
| WebTransport (H3) | Lane per subscription | Datagram | Yes |
| TCP / TLS / unix / stdio | Tag on the single lane | Dropped on the single lane | No |
| In-process | Lane per subscription | Dropped | n/a |

r[jetstream.subscription.conformance.single-lane]
A `LaneSupport::One` transport MUST support subscriptions. It is the row that motivates the whole specification: if the weakest transport cannot carry a subscription, every application that must reach a browser writes a second implementation, and the session model's transport independence stops at the RPC boundary.

r[jetstream.subscription.conformance.local]
An in-process session MUST realise subscriptions identically to a transport-backed one, per `r[jetstream.session.local]`, and MUST NOT serialise items to obtain ordering, per `r[jetstream.session.local.no-serialisation]`. Where an in-process subscription crosses to a remote peer, `r[jetstream.session.local.boundary]` applies: the ordering MUST continue across the boundary or the weakening MUST be reported.

## Compatibility

r[jetstream.subscription.compat]
Subscriptions MUST NOT change the frame format or the meaning of the `tag` field. A tag continues to identify one call; a streaming call simply remains in flight longer. The wire-visible consequence is that more than one response frame may bear a tag, which a peer that never issues a streaming request will never observe, and a peer that issues one has opted into.

r[jetstream.subscription.compat.existing-clients]
A client built before this specification issues only unary calls and MUST continue to work unchanged, per `r[jetstream.session.compat.existing-clients]`. A server MUST NOT stream a response to a request the protocol declares unary.

r[jetstream.subscription.compat.rpc-layer]
The client and server RPC layers assume one response per request today, and cannot express this without change. That change is source-level and is expected to break callers that name the unary result type directly; it MUST NOT require a protocol to be re-generated for its unary methods.

## Rationale

r[jetstream.subscription.rationale.reverse-call]
The rejected alternative is to make the producer a caller: the room issues requests to the subscriber. It is what `r[jetstream.session.symmetric]` already permits on a lane, and it needs no streaming responses. It was rejected because it does not reach the `LaneSupport::One` row. On a shared lane both peers would be issuing requests into one tag space, which needs the space partitioned by role and makes every existing single-lane deployment's tag allocation wrong. Streaming the response keeps one allocator, one direction of request, and one application implementation across every row of the table.

r[jetstream.subscription.rationale.not-a-queue]
A subscription is not a message queue and this specification does not require durability, at-least-once delivery, or acknowledgement of individual items. `r[jetstream.subscription.resume]` is the whole of what is offered, and it is deliberately weaker: the producer decides what it retains, and says so when it cannot meet a request. An application needing stronger guarantees builds them on the items, as it must anyway across a producer that may be evicted.
