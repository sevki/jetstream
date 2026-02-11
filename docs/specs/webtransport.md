# JetStream WebTransport Server Specification

This document specifies the WebTransport support in the `jetstream_http` crate. It enables downstream peers (browsers, using the WebTransport API) to establish symmetric RPC sessions with upstream JetStream services over HTTP/3 Extended CONNECT. **Upstream** points away from the keyboard (the rack, the cloud). **Downstream** points towards the keyboard (the browser, the local app). Because WebTransport sessions are symmetric, either side can open bidirectional streams: downstream can call upstream services, and upstream can push RPCs downstream (e.g., APNS-style notifications).

## H3 Connection Handler

r[jetstream.webtransport.h3-handler]
`H3Service` implements `ProtocolHandler` with ALPN `"h3"`. It holds an Axum `Router` for standard HTTP/3 requests and a map of `WebTransportHandler` implementations keyed by protocol version string. The h3 server connection is built with `enable_webtransport(true)`, `enable_extended_connect(true)`, and `enable_datagram(true)`.

r[jetstream.webtransport.h3-handler.dispatch]
On each accepted request, `H3Service` checks for WebTransport upgrades: if the request method is `CONNECT` and the protocol extension is `WEB_TRANSPORT`, the URI path is used to look up the registered `WebTransportHandler` by protocol version from the `rpc_handlers` map. If no handler matches, the request is rejected with an error log. Non-WebTransport requests are forwarded to the Axum router as standard HTTP/3 requests. `H3Service` exposes a builder or registration method to populate the `rpc_handlers` map with service implementations keyed by their `Protocol::VERSION` string.

## Session Negotiation

r[jetstream.webtransport.session]
When a WebTransport CONNECT request matches a registered handler, `H3Service` calls `WebTransportSession::accept(req, stream, h3_conn)` to complete the upgrade. This consumes the h3 connection — no further h3-level requests are processed on it. The session is then passed to `handler.handle_session(session, ctx)`.

r[jetstream.webtransport.session.protocol-version]
The WebTransport CONNECT request URI path identifies the service protocol version (matching `Protocol::VERSION` from `jetstream_rpc`). This allows a single `H3Service` to route to different service implementations based on the requested protocol.

## WebTransportHandler Trait

r[jetstream.webtransport.handler-trait]
The `WebTransportHandler` trait defines `handle_session(&self, session, ctx) -> Result<()>`. A blanket implementation is provided for any `T: Server` from `jetstream_rpc`, so any generated service server automatically becomes a WebTransport handler.

## Bidirectional Stream RPC

r[jetstream.webtransport.bidi]
The blanket `WebTransportHandler` impl accepts bidirectional streams via `session.accept_bi()` in a loop. Each `AcceptedBi::BidiStream` is split into read and write halves via `quic::BidiStream::split()`, then wrapped in `FramedRead` and `FramedWrite` with `ServerCodec<T>`. This provides length-delimited frame decoding/encoding with the standard `[size:u32 LE][type:u8][tag:u16 LE][payload]` format.

r[jetstream.webtransport.bidi.concurrent]
Each bidi stream is handled in a spawned task. Within a stream, requests are read sequentially from `FramedRead` but dispatched concurrently: each request frame is handed to a spawned task that calls `handler.rpc(ctx, req)`. Responses are sent back through an `mpsc` channel to a dedicated writer task that serializes them onto `FramedWrite`. This matches the concurrency model of `jetstream_over_quic`.

r[jetstream.webtransport.bidi.multi-stream]
The session loop accepts multiple bidirectional streams concurrently. Each stream gets its own spawned task with independent `FramedRead`/`FramedWrite` pair and response channel. The handler and context are cloned per-stream.

## Upstream-Initiated Streams

r[jetstream.webtransport.upstream-initiated]
Upstream can open bidirectional streams to downstream via `session.open_bi()`. On these streams, upstream acts as the RPC caller (sending request frames) and downstream acts as the handler (processing requests and sending response frames). The frame format is identical — only the direction of who opens the stream differs.

r[jetstream.webtransport.upstream-initiated.use-case]
Upstream-initiated streams enable push patterns like APNS-style notifications: an upstream Rust service opens a stream to a connected downstream browser and sends RPC requests such as `notify(title, body, badge)` or `invalidate_cache(keys)`. The downstream handler (via `useHandler` in `@sevki/jetstream-react`) processes the request, updates reactive state, and returns an acknowledgement. This avoids polling and leverages the same typed RPC machinery in both directions.

## Error Handling

r[jetstream.webtransport.errors]
RPC handler errors are converted via `IntoError` and logged; they do not terminate the stream. Codec decode errors (malformed frames) break the read loop for that stream. The writer task finishes draining queued responses before exiting.

r[jetstream.webtransport.errors.session]
If `accept_bi()` returns an error, the session loop breaks and the handler exits. Spawned stream tasks terminate naturally when their stream halves close. If the WebTransport session itself closes (peer disconnect, network failure), pending `mpsc` senders are dropped, causing writer tasks to complete.

## Connection Lifecycle

r[jetstream.webtransport.lifecycle]
The handler runs for the lifetime of the WebTransport session. After session acceptance, the handler spawns a task that loops on `accept_bi()`. The loop exits on `Ok(None)` (session closed normally) or `Err` (session error). Stream handler tasks are not explicitly cancelled — they terminate naturally when their stream halves close.

r[jetstream.webtransport.lifecycle.h3-fallback]
`H3Service::accept()` processes h3 requests in a loop. Standard HTTP/3 requests are spawned as tasks and processed by the Axum router. Once a WebTransport session is accepted, the h3 connection is consumed and the function returns — no further h3-level requests are processed on that connection.

## Server Setup

r[jetstream.webtransport.registration]
`H3Service` is registered on the `jetstream_quic::Router` like any other `ProtocolHandler`. Because it advertises the `h3` ALPN, it handles both standard HTTP/3 requests and WebTransport upgrades on the same QUIC endpoint. HTTP/2 (TCP+TLS) and HTTP/3 (QUIC+UDP) run concurrently on the same port.

```rust
use std::sync::Arc;
use axum::{routing::get, Router};
use jetstream_http::{AltSvcLayer, H3Service};

// Axum router with Alt-Svc header to advertise HTTP/3 to HTTP/2 clients
let router = Router::new()
    .fallback(get(handle_request))
    .layer(AltSvcLayer::new(4433));

// H3 + WebTransport handler for QUIC connections
let h3_service = Arc::new(H3Service::new(router.clone()));

let mut quic_router = jetstream_quic::Router::new();
quic_router.register(h3_service);

let server = jetstream_quic::Server::new_with_addr(
    server_cert, server_key, addr, quic_router,
);

// Run HTTP/2 (TCP) and HTTP/3 (QUIC) concurrently on the same port
tokio::select! {
    _ = run_http2_server(addr, router, tls_acceptor) => {}
    _ = server.run() => {}
}
```

## End-to-End Example

r[jetstream.webtransport.example]
A downstream peer connects to the same QUIC endpoint that serves native clients. The `H3Service` negotiates HTTP/3, upgrades via Extended CONNECT, and establishes a symmetric WebTransport session. Either side can open bidirectional streams. From the downstream React side:

```tsx
import { useState } from 'react';
import { JetStreamProvider, useJetStream, useRPC, useHandler } from '@sevki/jetstream-react';
import { EchoClient } from './generated/echo_rpc.js';
import { NotificationHandler } from './generated/notification_rpc.js';

function App() {
  return (
    <JetStreamProvider url="https://api.example.com:4433">
      <EchoDemo />
    </JetStreamProvider>
  );
}

function EchoDemo() {
  const [message, setMessage] = useState('');
  const [sum, setSum] = useState<number>();

  // Upstream calls (downstream → upstream)
  const echo = useJetStream(EchoClient);
  const { data, error, isLoading } = useRPC(() => echo.ping(message), [message]);

  // Downstream handler (upstream → downstream push notifications)
  const { events } = useHandler(NotificationHandler, {
    async notify(ctx, title, body, badge) {
      return { ack: true };
    },
  });

  return (
    <div>
      <input value={message} onChange={e => setMessage(e.target.value)} />
      <p>{isLoading ? '...' : data}</p>
      {error && <p>Error: {error.message}</p>}
      <button onClick={async () => setSum(await echo.add(2, 3))}>Add 2 + 3</button>
      {sum !== undefined && <p>Sum: {sum}</p>}
      <h3>Notifications from upstream</h3>
      <ul>
        {events
          .filter(e => e.method === 'notify')
          .map((e, i) => <li key={i}>{e.args[0]}: {e.args[1]}</li>)}
      </ul>
    </div>
  );
}
```

The session is symmetric: downstream opens streams for upstream calls (`echo.ping()`) and upstream opens streams for downstream push (`notify(title, body, badge)`). Both directions use the same `[size:u32 LE][type:u8][tag:u16 LE][payload]` frame format.

## HTTP Example Update

r[jetstream.webtransport.http-example]
The existing `examples/http.rs` must be updated to serve a React client app alongside the Axum HTTP routes. The example should register an `EchoServer` (or similar generated service) as a `WebTransportHandler` on the `H3Service`, so browser clients can connect via WebTransport and invoke RPC methods through the React hooks. The Axum router serves the static React build assets, while WebTransport CONNECT requests are upgraded to RPC sessions.

## Verification

r[jetstream.webtransport.verify-playwright]
End-to-end verification uses the Playwright MCP server with Chromium launched with `--origin-to-force-quic-on=127.0.0.1:4433` to force QUIC on the local server address. The Playwright config is at `.playwright-mcp.json` in the repository root. The test flow is:

1. Start the `examples/http.rs` server (HTTP/2 + HTTP/3 on `127.0.0.1:4433`)
2. Playwright navigates Chromium to `https://127.0.0.1:4433`
3. The React app loads and the `JetStreamProvider` establishes a WebTransport session
4. Playwright interacts with the UI (types into input, clicks buttons) and asserts RPC responses appear

This verifies the full stack: Rust server (`jetstream_http` + `WebTransportHandler`) ↔ WebTransport ↔ browser (`@sevki/jetstream-react` hooks) ↔ generated TypeScript client.
