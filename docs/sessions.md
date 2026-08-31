# Sessions and Lanes

A **session** is the association with a peer. A **lane** is one ordered,
reliable, flow-controlled sequence of frames obtained from it.

That split is the whole model. A lane is exactly what `ClientTransport<P>`
already describes from one end and `ServiceTransport<P>` from the other —
sessions do not change those bounds, they add a way to obtain a lane
without naming the concrete transport. A session orders nothing; ordering
is a property of a lane and only of a lane.

The normative rules live in
[the sessions and lanes specification](https://github.com/sevki/jetstream/blob/main/docs/specs/sessions-and-lanes.md).
This page is the guide to using them.

## Why

`ClientTransport<P>` describes one stream. Until now nothing described the
thing that can open another one, and the result was uneven:

| Path | Streams before sessions |
| --- | --- |
| WebTransport server | many |
| QUIC server | many |
| iroh client | **one** — `open_bi()`, once, at connect |
| Any client | **one**, with concurrency recovered by tag |

Servers accepted many streams; clients opened one and multiplexed by tag.
The iroh case was the sharpest: a transport whose whole point is many
independent streams plus key-based addressing was reachable only as a flat
duplex.

Both strategies are legitimate and both are still supported. They differ
in what they guarantee:

- **Tag multiplexing** — many calls over one lane. Ordered with respect to
  each other, and subject to head-of-line blocking: one slow response
  delays the bytes behind it.
- **Lane multiplexing** — one lane per call, or per group of calls.
  Unordered with respect to each other, independent, no head-of-line
  blocking between them.

Pick per call site. `Mux` has not gone anywhere, and a client that holds a
single transport and multiplexes by tag keeps working unchanged.

## The trait

```rust
#[async_trait::async_trait]
pub trait Session<P: Protocol>: Send + Sync {
    /// The end of a lane this peer opened, driven as the RPC caller.
    type ClientLane: ClientTransport<P>;
    /// The end of a lane the peer opened, served as the RPC callee.
    type ServiceLane: ServiceTransport<P>;

    fn capabilities(&self) -> Capabilities;
    fn context(&self) -> Context;

    async fn open_lane(&self) -> Result<Self::ClientLane, SessionError>;
    async fn accept_lane(&self) -> Result<Self::ServiceLane, SessionError>;
    async fn close(&self);
}
```

Both directions take `&self`, so a session is usable from several tasks at
once and opening a lane never needs exclusive access.

The two lane types differ because the model is symmetric: where the
transport allows it either peer may open a lane, and **the opener is the
RPC caller on that lane**. `open_lane` yields the caller's end;
`accept_lane` yields the callee's. A session is not fixed to a role.

## In process, end to end

`LocalSession` implements the model with no network under it, which makes
it the shortest way to see the whole thing. A lane goes straight into
`jetstream_rpc::server::run` on one side and into a generated channel on
the other — there is no adapter in between, because a lane already *is* a
transport.

```rust
{{#include ../examples/session_lanes.rs}}
```

```console
cargo run --example session_lanes
```

## Over a real connection

The same code against an mTLS QUIC connection: capabilities read from the
live connection, peer identity from the handshake, several independent
lanes on one connection, and a datagram alongside them.

```rust
{{#include ../examples/quic_session.rs}}
```

```console
cargo run --example quic_session --features quic
```

`jetstream_iroh::IrohSession` is the same shape. The difference is the row
it reports: identity by public key rather than by certificate.

## Capabilities

Capabilities are **reported, not inferred** — not from the concrete
transport type, and not reduced to a lowest common denominator. A caller
picks a multiplexing strategy without knowing which transport it holds.

| Transport | Lanes | Datagrams | Identity | Migration | Type |
| --- | --- | --- | --- | --- | --- |
| iroh | many | yes | key | yes | `jetstream_iroh::IrohSession` |
| QUIC | many | yes | certificate | yes | `jetstream_quic::QuicSession` |
| WebTransport (H3) | many | no¹ | certificate | yes | `jetstream_http::WebTransportSession`³ |
| TCP / TLS / unix / stdio | one | no | none² | no | `SingleLaneSession::client_io` / `service_io` |
| in-process | many | no | none | no | `LocalSession::pair` |

¹ HTTP/3 carries datagrams, but this binding has no `Datagrams<P>` impl, so
it reports the capability absent rather than promising something a caller
cannot then use.

² Unless the caller supplies one — see [byte streams](#byte-streams) below.

³ The type works, and a caller running its own h3 accept loop can use it.
But `H3Service` owns the HTTP/3 connection and routes through a private
handler that calls `accept_bi` directly, so the session is not yet
reachable from the shipped server path. Reconciling the two wants an
untyped lane `RpcRouter` can consume, since the router negotiates a
version per stream and dispatches to whichever protocol answers, while
`Session<P>` is typed to one protocol.

Ask before you rely on something:

```rust
use jetstream_rpc::session::{Capability, Session};

// Branch on it...
if session.capabilities().supports(Capability::ManyLanes) {
    // a lane per call
} else {
    // one lane, multiplexed by tag
}

// ...or refuse to continue without it.
session.capabilities().require(Capability::Datagrams)?;
```

`require` fails with `SessionError::Unsupported(capability)`. That is
deliberate: the alternative — emulating a datagram channel over a lane —
would be silently wrong about ordering and delivery, so it is not the easy
path.

Two capabilities are read from the live connection rather than from the
row. A QUIC or iroh peer that never advertised DATAGRAM support is
detected on its own, and `without_datagrams()` lets a caller that disabled
them locally say so:

```rust
let session = QuicSession::<MyChannel>::new(connection).without_datagrams();
assert!(!Session::<MyChannel>::capabilities(&session).datagrams);
```

## Identity

`Session::context()` returns the peer in the form the transport
established it, and every lane the session hands out carries the same
context — a handler reading only its lane sees the same peer the session
does.

`IdentityKind` is the *model*, which is what a caller has to branch on:

- `IdentityKind::None` — no transport-level authentication.
- `IdentityKind::Certificate` — a TLS peer certificate. QUIC, WebTransport.
- `IdentityKind::Key` — a public key the transport itself established. iroh.

`IdentityKind::requires_address()` is the question worth asking. Under
`Key` the identity *is* the address, so a caller must not be made to carry
placement information alongside it; under the other two it must.

## Datagrams

The datagram channel belongs to the session, not to any lane. It is
unordered and unreliable by construction, so it is deliberately **not** a
`ClientTransport` — none of the lane ordering guarantees apply to it, and
the type system says so.

A transport implements three raw methods — `max_datagram_size`,
`send_datagram_bytes`, `recv_datagram_bytes` — and gets the framed ones
free:

```rust
// Sending names the direction, because a session is not fixed to a role.
session.send_request_datagram(frame).await?;   // as the caller
session.send_response_datagram(frame).await?;  // as the callee

// Receiving cannot: one queue takes one reader, so the caller decodes.
let frame: Frame<MyRequest> =
    decode_datagram(session.recv_datagram_bytes().await?)?;
```

A session that reports no datagram channel refuses traffic on it rather
than leaving the check to the caller: both `send_datagram_bytes` and
`recv_datagram_bytes` fail with `SessionError::Unsupported`. Receiving is
the reason — awaiting a datagram on an endpoint whose receive buffer is
switched off never yields, and parking forever is the one failure mode
the degradation rule exists to prevent.

A datagram carries a complete frame or it is discarded — there is nothing
to reassemble it from, and trailing bytes after the frame mean it is not a
complete frame either. `check_datagram_size` rejects a frame larger than
the path allows at the sender rather than fragmenting it.

## Lifetime

Closing a session ends every lane on it: calls in flight fail rather than
hang, and a later `open_lane` is refused with `SessionError::Closed`.
Closing a *lane* does not close its session.

`SessionError` distinguishes the cases a caller has to tell apart —
`Closed` for a deliberate close, `LaneClosed` for one lane ending under a
session that is still up, `LaneLimitReached` for a second open on a
one-lane transport, `Unsupported` for a capability the session does not
have, and `Transport` for an actual fault.

## Byte streams

A byte-stream transport needs no new type. A TCP client is already a
`Framed` value satisfying `ClientTransport`, so `SingleLaneSession` just
wraps it and reports `LaneSupport::One`: the first open succeeds and every
later one fails with `SessionError::LaneLimitReached`, rather than
blocking or silently sharing the lane that is already open.

```rust
use jetstream_rpc::session::SingleLaneSession;

// A stream that knows its own peer — TCP, a unix socket.
let session = SingleLaneSession::<MyChannel, _, _>::service_io(stream);

// One that does not — TLS, stdio, anything in memory. The caller states
// the identity the framed bytes never see.
let session = SingleLaneSession::<MyChannel, _, _>::service_io_with_context(
    stream, peer_context,
);
```

## Version negotiation

Version negotiation is scoped to a lane. A `Tversion` on one lane must not
disturb its siblings — see `r[jetstream.version.negotiation.reset]` and
the [version negotiation spec](https://github.com/sevki/jetstream/blob/main/docs/specs/version-negotiation.md).
