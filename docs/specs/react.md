# JetStream React Specification

This document specifies `@sevki/jetstream-react`, a React hooks library for bidirectional RPC over WebTransport.

**Upstream** points away from the keyboard — the rack, the cloud, the remote service. **Downstream** points towards the keyboard — the browser, the local app. Because WebTransport sessions are symmetric, either side can open bidirectional streams and send RPC requests. The downstream can call upstream services (`useJetStream` + `useRPC`), and upstream can push RPCs downstream — like APNS-style notifications — which downstream handles and renders reactively (`useHandler`).

## WebTransport Connection

r[jetstream.react.webtransport]
`WebTransportConnection` wraps the browser WebTransport API to establish a QUIC session with an upstream peer. The upstream counterpart is `jetstream_http`'s WebTransport handler which accepts sessions via HTTP/3 Extended CONNECT. The session is symmetric: either side can open bidirectional streams and send RPC frames on them.

r[jetstream.react.webtransport.stream]
For upstream RPCs (downstream → upstream), a bidirectional stream is opened via `transport.createBidirectionalStream()`. The writable side sends binary-encoded request frames. The readable side receives binary-encoded response frames. Frame format is `[size:u32 LE][type:u8][tag:u16 LE][payload]`, identical to `jetstream_rpc::Frame<T>`.

r[jetstream.react.webtransport.accept]
For downstream RPCs (upstream → downstream), the connection listens for incoming bidirectional streams opened by upstream via `session.incomingBidirectionalStreams`. Each incoming stream is handled by a registered handler which reads request frames, dispatches them, and writes response frames back — the mirror image of what the Rust `WebTransportHandler` does for downstream-opened streams.

r[jetstream.react.webtransport.mux]
Each bidirectional stream (whether opened locally or accepted from upstream) creates a `Mux` over its read/write pair. The `Mux` allocates tags from a `TagPool`, sends request frames, and demultiplexes response frames by tag — matching the Rust `jetstream_rpc::Mux` behavior. Multiple concurrent RPCs share a single stream.

r[jetstream.react.webtransport.lifecycle]
The connection supports reconnection. If the WebTransport session closes (network error, server restart), the connection transitions to a `disconnected` state. Pending RPCs are rejected with a transport error. The provider can be configured to auto-reconnect with backoff.

r[jetstream.react.webtransport.errors]
Transport-level errors (connection refused, stream reset, timeout) are distinct from RPC-level errors (error frames with type `5`). RPC errors are decoded as `JetStreamError` from the error frame payload. Transport errors create a `JetStreamError` with a `transport` error code.

## Provider

r[jetstream.react.provider]
`JetStreamProvider` is a React context provider that manages the WebTransport session lifecycle. It accepts `url` (required — the upstream QUIC address), and optional `maxConcurrentRequests` (default 256, matching `TagPool` size). The provider establishes the session on mount and closes it on unmount. It also runs an accept loop on `session.incomingBidirectionalStreams` to dispatch incoming upstream streams to registered downstream handlers (see `useHandler`).

```tsx
<JetStreamProvider url="https://api.example.com:4433">
  <App />
</JetStreamProvider>
```

r[jetstream.react.provider.connection-state]
The provider exposes connection state (`connecting`, `connected`, `disconnected`, `error`) via context. Components can read connection state via `useJetStreamStatus()` to show connection indicators or disable UI while disconnected.

## Upstream Hooks (downstream → upstream)

### useJetStream

r[jetstream.react.use-jetstream]
`useJetStream` takes a generated client constructor and returns a memoized client instance for calling upstream services. The generated client constructor accepts a `Mux` (e.g., `new EchoClient(mux)`), and the hook supplies the mux from the provider context.

```tsx
import { EchoClient } from './generated/echo_rpc.js';

function MyComponent() {
  const echo = useJetStream(EchoClient);
  // echo is typed as EchoClient
  // call upstream methods: await echo.ping("hello")
}
```

r[jetstream.react.use-jetstream.type-safety]
The hook infers the return type from the client constructor. `useJetStream(EchoClient)` returns `EchoClient`. This preserves full type safety for all generated service methods and their parameter/return types.

r[jetstream.react.use-jetstream.identity]
The returned client instance is stable across re-renders (referentially equal) as long as the provider context and constructor remain the same. This makes it safe to pass as a dependency to `useEffect`, `useCallback`, etc.

### useRPC

r[jetstream.react.use-rpc]
`useRPC` provides a reactive wrapper around a single upstream RPC call. It returns `{ data, error, isLoading, refetch }` and triggers the call on mount or when dependencies change.

```tsx
function SearchResults() {
  const [query, setQuery] = useState('');
  const search = useJetStream(SearchClient);
  const { data, error, isLoading } = useRPC(() => search.find(query), [query]);

  return (
    <div>
      <input value={query} onChange={e => setQuery(e.target.value)} />
      {isLoading && <div>Loading...</div>}
      {error && <div>Error: {error.message}</div>}
      {data && <div>{data}</div>}
    </div>
  );
}
```

r[jetstream.react.use-rpc.deps]
The second argument is a dependency array (like `useEffect`). The RPC call is re-executed when any dependency changes. If the dependency array is empty, the call runs once on mount.

r[jetstream.react.use-rpc.stale]
When dependencies change, the previous in-flight request is ignored (its result is discarded). Only the latest request's result is reflected in the returned state. This prevents stale responses from overwriting newer data.

r[jetstream.react.use-rpc.refetch]
The returned `refetch` function re-executes the RPC call with the current dependencies. It can be called imperatively (e.g., on button click) to refresh data.

## Downstream Hooks (upstream → downstream)

### useHandler

r[jetstream.react.use-handler]
`useHandler` registers a handler for incoming upstream RPCs and provides reactive state for the data they push. When upstream opens a bidirectional stream and sends request frames (e.g., APNS-style push notifications), the handler's methods are invoked. The hook returns reactive state that updates on each incoming call, so components re-render when upstream pushes new data.

```tsx
import { NotificationHandler } from './generated/notification_rpc.js';

function NotificationBadge() {
  const { events, error } = useHandler(NotificationHandler, {
    async notify(ctx, title, body, badge) {
      // Return value is sent back upstream as the response frame
      return { ack: true };
    },
    async invalidateCache(ctx, keys) {
      queryClient.invalidateQueries({ queryKey: keys });
    },
  });

  // events is a reactive array of all upstream calls received
  const unread = events.filter(e => e.method === 'notify').length;

  return <span>{unread > 0 && `${unread} new`}</span>;
}
```

r[jetstream.react.use-handler.state]
The hook returns `{ events, error }`. `events` is a reactive array of objects `{ method: string, args: unknown[], result: unknown, timestamp: number }` representing each upstream RPC received. The array updates on every incoming call, triggering a re-render. `error` holds the last transport or dispatch error, if any.

r[jetstream.react.use-handler.type-safety]
The hook takes a generated handler interface (e.g., `NotificationHandler`) and an implementation object. TypeScript enforces that the implementation satisfies all methods defined in the handler interface with correct parameter and return types.

r[jetstream.react.use-handler.lifecycle]
The handler is registered on mount and unregistered on unmount. While registered, incoming streams matching the handler's protocol are dispatched to it. Multiple `useHandler` hooks can coexist for different service protocols on the same session.

r[jetstream.react.use-handler.symmetric]
The same generated service definition produces both a client class (for upstream RPCs) and a handler interface (for downstream RPCs). A single WebTransport session can use both simultaneously — downstream calls upstream via `useJetStream(EchoClient)` while handling calls from upstream via `useHandler(NotificationHandler, impl)`.

## State Management

r[jetstream.react.state-management]
`@sevki/jetstream-react` provides reactive state for both directions: `useRPC` for upstream queries and `useHandler` for downstream push events. For imperative upstream calls (mutations, actions), the library does not provide its own state management — generated clients return plain `Promise`s that integrate directly with battle-tested libraries.

r[jetstream.react.state-management.tanstack]
The recommended approach for imperative upstream calls is TanStack Query (`@tanstack/react-query`), which provides `useMutation` with built-in loading/error state, retries, cache invalidation, and optimistic updates:

```tsx
import { useMutation } from '@tanstack/react-query';

function AddForm() {
  const echo = useJetStream(EchoClient);
  const { mutate, data, isPending } = useMutation({
    mutationFn: ({ a, b }: { a: number; b: number }) => echo.add(a, b),
  });

  return (
    <div>
      <button onClick={() => mutate({ a: 2, b: 3 })} disabled={isPending}>
        Add 2 + 3
      </button>
      {data !== undefined && <p>Sum: {data}</p>}
    </div>
  );
}
```

r[jetstream.react.state-management.plain]
For simple cases, the client can be called directly without any wrapper — it returns a standard `Promise`:

```tsx
function AddButton() {
  const echo = useJetStream(EchoClient);
  const [sum, setSum] = useState<number>();

  return (
    <button onClick={async () => setSum(await echo.add(2, 3))}>
      Add 2 + 3 {sum !== undefined && `= ${sum}`}
    </button>
  );
}
```

## Error Handling

r[jetstream.react.error-handling]
Errors from RPC calls are `JetStreamError` instances, preserving the error code, severity, message, and labels from upstream. `useRPC` exposes errors through the `error` field. For imperative calls, errors propagate as rejected promises and can be handled with `try/catch` or the external state library's error handling (e.g., TanStack Query's `onError` callback).

r[jetstream.react.error-boundary]
An optional `JetStreamErrorBoundary` component catches unhandled JetStream errors thrown during rendering (e.g., from Suspense) and renders a fallback UI. It receives the `JetStreamError` with full diagnostic information.

## Suspense Support

r[jetstream.react.suspense]
`useRPC` supports React Suspense when the `suspense: true` option is passed. In suspense mode, the hook throws a Promise while loading (triggering the nearest Suspense boundary) and throws the error directly on failure (triggering the nearest Error Boundary).

```tsx
function PingResult({ message }: { message: string }) {
  const echo = useJetStream(EchoClient);
  const data = useRPC(() => echo.ping(message), [message], { suspense: true });
  // data is always defined here — loading/error handled by boundaries
  return <div>{data}</div>;
}

// Usage: message is dynamic state from a parent component
<Suspense fallback={<div>Loading...</div>}>
  <ErrorBoundary>
    <PingResult message={userInput} />
  </ErrorBoundary>
</Suspense>
```

## Code Generation

r[jetstream.react.codegen]
The `jetstream_codegen` CLI generates an optional React hooks file per `#[service]` definition when `--react-out` is specified. This file re-exports a typed `use{Service}` hook (wrapping `useJetStream`) and re-exports the handler interface for `useHandler`.

```typescript
// Generated: use-echo.ts
import { useJetStream } from '@sevki/jetstream-react';
import { EchoClient } from './echo_rpc.js';
export { EchoHandler } from './echo_rpc.js';

export function useEcho() {
  return useJetStream(EchoClient);
}
```

## End-to-End Example

r[jetstream.react.example]
A complete setup requires a `jetstream_http` server with the `WebTransportHandler` registered (see `jetstream.webtransport` spec), and a React app using the provider and hooks.

Upstream (Rust — see `jetstream.webtransport` spec for full details):

```rust
use std::sync::Arc;
use axum::{routing::get, Router};
use jetstream_http::{AltSvcLayer, H3Service};

let router = Router::new()
    .fallback(get(handle_request))
    .layer(AltSvcLayer::new(4433));

let h3_service = Arc::new(H3Service::new(router.clone()));

let mut quic_router = jetstream_quic::Router::new();
quic_router.register(h3_service);

let server = jetstream_quic::Server::new_with_addr(
    server_cert, server_key, addr, quic_router,
);

tokio::select! {
    _ = run_http2_server(addr, router, tls_acceptor) => {}
    _ = server.run() => {}
}
```

Downstream (React):

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

The `JetStreamProvider` opens a symmetric WebTransport session. Downstream opens streams for upstream calls (`echo.ping()`). Upstream opens streams for downstream push (`notify()`). Both directions use the same `[size:u32 LE][type:u8][tag:u16 LE][payload]` frame format.

## Verification

r[jetstream.react.verify-playwright]
End-to-end verification of the React hooks uses the Playwright MCP server with Chromium launched with `--origin-to-force-quic-on=127.0.0.1:4433`. The `examples/http.rs` server is started with a registered `WebTransportHandler`, Playwright navigates to the app, and interacts with the UI to verify that `useRPC` and direct client calls produce correct results from live RPC calls over WebTransport. See `jetstream.webtransport.verify-playwright` for the full test flow.

## Package Structure

r[jetstream.react.package]
The package is published as `@sevki/jetstream-react` and depends on `@sevki/jetstream-wireformat`, `@sevki/jetstream-rpc`, and `react` (peer dependency). It lives at `packages/jetstream_react/` in the monorepo workspace.
