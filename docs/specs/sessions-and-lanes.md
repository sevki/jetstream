# JetStream Sessions and Lanes Specification

This document specifies the transport model underneath JetStream RPC: a **session** is an association with a peer, and a **lane** is one ordered frame sequence within it. Today `ClientTransport`/`ServiceTransport` describe a single stream and nothing describes the thing that can open another one, so transports that carry many streams (QUIC, WebTransport, iroh) are reachable only through whatever their adapter happened to open. This specifies the missing primitive and states, per transport, what is guaranteed.

The motivating asymmetry: servers already accept many streams (`r[jetstream.webtransport.bidi.multi-stream]`, `jetstream_over_quic`), while clients open exactly one and recover concurrency with tag multiplexing (`Mux`). Both are valid strategies. Neither should be implied by the type system when the other was wanted.

## Overview

r[jetstream.session.overview]
A JetStream peer association is modelled in three parts:

1. **Session** — the association with a peer. Carries identity and capabilities. Can open and accept lanes.
2. **Lane** — one ordered, reliable, flow-controlled sequence of frames. This is what `ClientTransport<P>` and `ServiceTransport<P>` already describe.
3. **Datagrams** — an optional unordered, unreliable channel belonging to the session, not to any lane.

Ordering is a property of a lane and only of a lane. A session orders nothing.

r[jetstream.session.overview.additive]
This model MUST NOT change the `ClientTransport<P>` or `ServiceTransport<P>` trait bounds. A lane *is* a value satisfying those bounds; what changes is that it becomes obtainable from a session rather than only from a transport-specific builder. Existing code that holds one transport and multiplexes by tag remains conforming.

## Sessions

r[jetstream.session.trait]
A session MUST expose: opening a new lane, accepting a peer-opened lane, and reporting its capabilities. Opening and accepting are fallible and asynchronous. A session MUST be usable concurrently from multiple tasks — opening a lane MUST NOT require exclusive access to the session.

r[jetstream.session.symmetric]
Sessions are symmetric where the transport is symmetric: either peer MAY open a lane, and the opener is the RPC caller on that lane. This generalises `r[jetstream.webtransport.upstream-initiated]` from WebTransport to every transport that supports it, and is what `r[jetstream.session.capabilities]` reports as bidirectional lane opening.

r[jetstream.session.lifetime]
A lane MUST NOT outlive its session. When a session closes, every lane opened on it MUST terminate, and in-flight calls on those lanes MUST fail rather than hang. A lane closing MUST NOT close its session.

r[jetstream.session.single-lane]
A transport that cannot open more than one lane (a plain byte stream: TCP, TLS, unix socket, stdio) MUST still be presented as a session. It reports `LaneSupport::One`, its first lane open succeeds, and subsequent opens MUST fail with a distinct, inspectable error rather than blocking or silently sharing the existing lane.

## Lanes

r[jetstream.lane.definition]
A lane is a bidirectional, reliable, ordered sequence of JetStream frames in the `[size:u32 LE][type:u8][tag:u16 LE][payload]` format. A lane satisfies `ClientTransport<P>` from one end and `ServiceTransport<P>` from the other.

r[jetstream.lane.delivery-order]
Frames written to a lane MUST be delivered to the peer in write order. This is the only ordering guarantee JetStream makes.

r[jetstream.lane.not-completion-order]
Delivery order is not completion order. Two requests delivered in order on one lane MAY complete in either order, and their responses MAY be written to the lane in either order — that is pipelining, and `r[jetstream.lane.tag-mux]` depends on it. Implementations MUST NOT serialise handler execution in order to preserve response order.

r[jetstream.lane.no-cross-lane-order]
There is no ordering between lanes. Frames on distinct lanes MAY be delivered in any relative order, including when both lanes belong to one session and one peer wrote them in a known order. Code requiring two calls to be ordered MUST place them on the same lane.

r[jetstream.lane.independence]
A lane MUST NOT be blocked by another lane on the same session. A large or stalled frame on one lane MUST NOT delay delivery on another. Transports whose underlying streams provide this (QUIC, WebTransport, iroh) MUST map lanes onto distinct streams rather than interleaving them on one.

r[jetstream.lane.backpressure]
A lane MUST propagate the peer's receive window to the writer: when the peer is not consuming, the `Sink` side MUST report pending rather than buffer without bound. A session MAY additionally apply a session-wide window across its lanes.

## Multiplexing Strategies

r[jetstream.rcp.multiplexing.strategies]
This refines `r[jetstream.rcp.multiplexing]`, which requires clients to support multiplexing without saying how. Two strategies are conforming, and they order differently:

| Strategy | Concurrency by | Requests ordered? | Head-of-line blocking |
| --- | --- | --- | --- |
| Tag mux | `tag` within one lane | Yes — one lane | Yes, at the frame level |
| Lane mux | one lane per call or per ordering domain | No — across lanes | No |

r[jetstream.lane.tag-mux]
Tag multiplexing assigns each in-flight call a distinct `u16` tag from a pool and demultiplexes responses by tag. All calls share one lane and are therefore delivered in issue order. A tag MUST NOT be reused while a call bearing it is in flight. Tag exhaustion MUST be reported as a distinct error, not by blocking indefinitely.

r[jetstream.lane.lane-mux]
Lane multiplexing places calls on distinct lanes, giving each independent delivery and flow control at the cost of any ordering between them. A client MAY open a lane per call, or per **ordering domain** — a set of calls that must be ordered with respect to each other but not with respect to anything else.

r[jetstream.rcp.multiplexing.selection]
A client MUST support tag multiplexing, because it is the only strategy available on `LaneSupport::One` transports. A client on a `LaneSupport::Many` transport SHOULD make the strategy selectable by the caller rather than fixing it, since the correct choice depends on the caller's ordering requirements and not on the transport.

## Capabilities

r[jetstream.session.capabilities]
A session MUST report its capabilities so that callers can select a multiplexing strategy without inspecting the concrete transport type. Capabilities MUST be reported, not inferred from a lowest common denominator. At minimum:

- **Lane support** — `One` or `Many`.
- **Datagrams** — whether the session has an unordered channel.
- **Identity** — the peer identity model, per `r[jetstream.session.identity]`.
- **Migration** — whether the session survives a change of network path.

r[jetstream.session.capabilities.degradation]
Code written against a capability it does not have MUST fail explicitly. An implementation MUST NOT emulate a missing capability silently — in particular, it MUST NOT emulate `Many` by interleaving lanes on one stream, which would violate `r[jetstream.lane.independence]` while appearing to satisfy it.

## Datagrams

r[jetstream.session.datagrams]
Where the transport supports them, a session MAY expose a datagram channel. Datagrams are unordered and unreliable: they MAY be dropped, MAY arrive in any order, and MUST NOT be retransmitted by JetStream. A datagram carries a complete frame or is discarded; frames exceeding the path datagram limit MUST be rejected at the send site rather than fragmented.

r[jetstream.session.datagrams.not-a-lane]
Datagrams MUST NOT be presented as a lane, and lane ordering requirements MUST NOT be read as applying to them. Requests requiring a response SHOULD use a lane; datagrams are for messages whose loss is acceptable.

## Identity

r[jetstream.session.identity]
A session MUST expose the peer's identity in the form its transport establishes:

- **Key** — a public key established by the transport itself. iroh, whose `EndpointAddr` is derived from a node's key and whose reachability is resolved by discovery rather than by a caller-supplied address.
- **Certificate** — a TLS peer certificate. QUIC and WebTransport.
- **None** — no transport-level peer authentication. Unix sockets, plain TCP, in-process.

r[jetstream.session.identity.addressing]
Where identity is `Key`, dialling a peer identity is sufficient to reach it and callers MUST NOT be required to supply a network address. Where identity is `Certificate` or `None`, an address is required and the identity only authenticates the peer once reached. This distinction is a capability, not an implementation detail: it determines whether a caller must carry placement information alongside identity.

## Version Negotiation

r[jetstream.session.version-scope]
Version negotiation is per lane, not per session, consistent with `r[jetstream.webtransport.router.per-stream-version]`. Each lane MUST complete its own `Tversion`/`Rversion` exchange before carrying service frames.

r[jetstream.session.version-scope.reset]
`r[jetstream.version.negotiation.reset]` specifies that a `Tversion` clunks all open fids and terminates pending I/O. With more than one lane per session, that reset MUST be scoped to the lane on which the `Tversion` arrived. A `Tversion` MUST NOT disturb state on any other lane of the same session.

r[jetstream.session.version-scope.msize]
The negotiated `msize` is a property of the lane that negotiated it. Distinct lanes on one session MAY negotiate different `msize` values, and an implementation MUST NOT apply one lane's negotiated maximum to another.

## Transport Conformance

r[jetstream.session.conformance]
Each transport MUST declare its capabilities and satisfy the lane requirements:

| Transport | Lanes | Datagrams | Identity | Migrates |
| --- | --- | --- | --- | --- |
| iroh | Many | Yes | Key | Yes |
| QUIC | Many | Yes | Certificate | Yes |
| WebTransport (H3) | Many | Yes | Certificate | Yes |
| TCP / TLS / unix / stdio | One | No | None | No |
| In-process | Many | No | None | n/a |

r[jetstream.session.conformance.iroh]
The iroh client MUST expose the iroh `Connection` as a session rather than opening a single bidirectional stream at connect time and discarding the ability to open more. Reporting `LaneSupport::Many` and `Identity::Key` is the point of the transport; a client that opens one lane eagerly satisfies neither.

r[jetstream.session.conformance.webtransport]
The existing WebTransport handler already satisfies the lane requirements on the accepting side — `r[jetstream.webtransport.bidi.multi-stream]` is `accept_lane` and `r[jetstream.webtransport.upstream-initiated]` is `open_lane`. It MUST be expressed as the session trait rather than as a bespoke path.

r[jetstream.session.conformance.single-stream]
A byte-stream transport MUST report `LaneSupport::One` and satisfy `r[jetstream.session.single-lane]`. Its single lane satisfies every lane requirement except `r[jetstream.lane.independence]`, which is vacuous with one lane.

## In-Process Sessions

r[jetstream.session.local]
Two peers in one process MUST be able to hold a session without a transport. An in-process session reports `LaneSupport::Many`, no datagrams, and `Identity::None`, and its lanes MUST satisfy `r[jetstream.lane.delivery-order]` and `r[jetstream.lane.no-cross-lane-order]` identically to a transport-backed session.

r[jetstream.session.local.no-serialisation]
An in-process lane MUST NOT encode frames to bytes to obtain ordering. Ordering is a property the lane provides; on a transport it is provided by the underlying stream, and in-process it MUST be provided by the lane implementation directly.

r[jetstream.session.local.order-handoff]
An in-process lane whose delivery is admitted asynchronously MUST hold the delivery order taken at the call site, not the order in which admission completes. If a frame is abandoned before delivery, its place MUST pass to the next frame on that lane rather than being released — otherwise a later frame can be delivered ahead of an earlier one still in flight.

r[jetstream.session.local.boundary]
Where an in-process lane and a transport-backed lane meet — a call that resolves to a remote peer — the lane MUST continue across the boundary rather than terminating at it. An implementation that drops the ordering at the boundary MUST report that it has done so; ordering silently weakening at a routing decision is a conformance failure, not a degradation.

> **Note.** celld's `CallOrder` (`crates/celld/js.rs`) is an existing implementation of `r[jetstream.session.local.order-handoff]`, built before this specification and keyed per callee rather than per lane. It documents dropping order at the remote boundary, which `r[jetstream.session.local.boundary]` makes reportable. celld is outside this specification's `impls` globs, so it is named here as the reconciliation target, not annotated against these requirements.

## Compatibility

r[jetstream.session.compat]
Introducing sessions MUST NOT be a wire-format change. Frames, tags, and the `Tversion`/`Rversion` exchange are unchanged; what changes is how many lanes a client may open and what it may assume about ordering between them.

r[jetstream.session.compat.existing-clients]
A client built before this specification opens one lane and tag-multiplexes on it. That behaviour remains conforming under `r[jetstream.rcp.multiplexing.selection]` and MUST continue to work against a session-aware server without change.
