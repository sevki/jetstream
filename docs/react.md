# React

`@sevki/jetstream-react` provides React hooks for bidirectional RPC over WebTransport. Downstream (browser) can call upstream services, and upstream can push RPCs downstream.

## Installation

```bash
pnpm add @sevki/jetstream-react @sevki/jetstream-rpc @sevki/jetstream-wireformat
```

`react` (>=18) is a peer dependency.

## Quick Start

```tsx
import { JetStreamProvider, useJetStream, useJetStreamStatus, useRPC } from '@sevki/jetstream-react';
import { EchoHttpClient, rmessageDecode, PROTOCOL_VERSION, PROTOCOL_NAME } from './generated/echohttp_rpc.js';

function App() {
  return (
    <JetStreamProvider url={`https://api.example.com:4433/${PROTOCOL_NAME}`}>
      <EchoDemo />
    </JetStreamProvider>
  );
}

function EchoDemo() {
  const status = useJetStreamStatus();
  const echo = useJetStream(EchoHttpClient, rmessageDecode, PROTOCOL_VERSION);

  const { data, error, isLoading } = useRPC(
    () => (echo ? echo.ping('hello') : Promise.resolve('')),
    [echo],
  );

  return (
    <div>
      <p>Connection: {status}</p>
      {isLoading && <p>Loading...</p>}
      {error && <p>Error: {error.message}</p>}
      {data && <p>Echo: {data}</p>}
    </div>
  );
}
```

## Provider

`JetStreamProvider` manages the WebTransport session lifecycle. It establishes the session on mount and closes it on unmount.

```tsx
<JetStreamProvider url="https://api.example.com:4433/rs.jetstream.proto/echohttp">
  <App />
</JetStreamProvider>
```

**Props:**
- `url` (required) — the upstream WebTransport URL, typically `https://host:port/{PROTOCOL_NAME}`
- `children` — React children

The provider exposes connection state and protocol version through context:
- `session` — the `WebTransport` instance (or `null` before connected)
- `state` — `'connecting'` | `'connected'` | `'disconnected'` | `'error'`
- `protocolVersion` — the negotiated protocol version string (or `null` before negotiation)

### useJetStreamStatus

Read the current connection state:

```tsx
function StatusIndicator() {
  const status = useJetStreamStatus();
  return <span className={status}>{status}</span>;
}
```

## useJetStream

Creates a memoized RPC client for calling upstream services. Handles stream creation, version negotiation, and Mux setup automatically.

```tsx
const client = useJetStream(ClientClass, responseDecode, protocolVersion);
```

**Parameters:**
- `ClientClass` — the generated client constructor (e.g., `EchoHttpClient`)
- `responseDecode` — the generated response decoder function (e.g., `rmessageDecode`)
- `protocolVersion` — the protocol version string for Tversion negotiation (e.g., `PROTOCOL_VERSION`)

**Returns:** the client instance (or `null` while connecting).

The hook:
1. Opens a bidirectional stream on the WebTransport session
2. Performs Tversion/Rversion negotiation on the raw stream
3. Stores the negotiated version in the provider context
4. Creates a `WebTransportTransport` and `Mux` over the stream
5. Constructs and returns the client

The returned client is stable across re-renders as long as the session and constructor remain the same.

```tsx
import { EchoHttpClient, rmessageDecode, PROTOCOL_VERSION } from './generated/echohttp_rpc.js';

function MyComponent() {
  const echo = useJetStream(EchoHttpClient, rmessageDecode, PROTOCOL_VERSION);

  if (!echo) return <p>Connecting...</p>;

  return <button onClick={() => echo.ping('hi')}>Ping</button>;
}
```

## useRPC

Reactive wrapper around a single RPC call. Re-executes when dependencies change.

```tsx
const { data, error, isLoading, refetch } = useRPC(fn, deps);
```

**Parameters:**
- `fn` — a function returning a `Promise<T>` (the RPC call)
- `deps` — dependency array (like `useEffect`); the call re-runs when dependencies change

**Returns:**
- `data: T | undefined` — the resolved value
- `error: Error | undefined` — the rejection reason
- `isLoading: boolean` — whether a request is in flight
- `refetch: () => void` — manually re-execute the call

When dependencies change, the previous in-flight request is discarded (stale results are never applied).

```tsx
function SearchResults() {
  const [query, setQuery] = useState('');
  const search = useJetStream(SearchClient, rmessageDecode, PROTOCOL_VERSION);

  const { data, error, isLoading } = useRPC(
    () => (search ? search.find(query) : Promise.resolve([])),
    [search, query],
  );

  return (
    <div>
      <input value={query} onChange={e => setQuery(e.target.value)} />
      {isLoading && <p>Loading...</p>}
      {error && <p>Error: {error.message}</p>}
      {data?.map(item => <div key={item.id}>{item.name}</div>)}
    </div>
  );
}
```

## useHandler

Registers a handler for incoming upstream-initiated RPCs (push notifications, cache invalidation, etc.).

```tsx
import { NotificationHandler } from './generated/notification_rpc.js';

function NotificationBadge() {
  const { events, error } = useHandler(NotificationHandler, {
    async notify(ctx, title, body) {
      return { ack: true };
    },
  });

  const unread = events.filter(e => e.method === 'notify').length;
  return <span>{unread > 0 && `${unread} new`}</span>;
}
```

The handler is registered on mount and unregistered on unmount. Incoming bidirectional streams from upstream are dispatched to the matching handler.

## Imperative Calls and Mutations

For imperative operations (mutations, fire-and-forget), call the client directly. The generated client methods return plain `Promise`s:

```tsx
function AddButton() {
  const echo = useJetStream(EchoHttpClient, rmessageDecode, PROTOCOL_VERSION);
  const [sum, setSum] = useState<number>();

  return (
    <button
      disabled={!echo}
      onClick={async () => {
        if (echo) setSum(await echo.add(2, 3));
      }}
    >
      Add 2 + 3 {sum !== undefined && `= ${sum}`}
    </button>
  );
}
```

For richer mutation state (loading, error, retries, cache invalidation), use TanStack Query:

```tsx
import { useMutation } from '@tanstack/react-query';

function AddForm() {
  const echo = useJetStream(EchoHttpClient, rmessageDecode, PROTOCOL_VERSION);

  const { mutate, data, isPending } = useMutation({
    mutationFn: ({ a, b }: { a: number; b: number }) => echo!.add(a, b),
  });

  return (
    <div>
      <button onClick={() => mutate({ a: 2, b: 3 })} disabled={isPending || !echo}>
        Add 2 + 3
      </button>
      {data !== undefined && <p>Sum: {data}</p>}
    </div>
  );
}
```

## Full Example

```tsx
import { useState } from 'react';
import {
  JetStreamProvider,
  useJetStream,
  useJetStreamStatus,
  useRPC,
} from '@sevki/jetstream-react';
import {
  EchoHttpClient,
  rmessageDecode,
  PROTOCOL_VERSION,
  PROTOCOL_NAME,
} from './generated/echohttp_rpc.js';

const SERVER_URL = `https://127.0.0.1:4433/${PROTOCOL_NAME}`;

function EchoDemo() {
  const status = useJetStreamStatus();
  const echo = useJetStream(EchoHttpClient, rmessageDecode, PROTOCOL_VERSION);
  const [message, setMessage] = useState('hello');
  const [sum, setSum] = useState<number | null>(null);

  const { data, error, isLoading } = useRPC(
    () => (echo ? echo.ping(message) : Promise.resolve('')),
    [echo, message],
  );

  return (
    <div>
      <h1>JetStream Echo Demo</h1>
      <p>Connection: {status}</p>
      <p>Protocol: {PROTOCOL_VERSION}</p>

      <h2>Ping</h2>
      <input
        value={message}
        onChange={e => setMessage(e.target.value)}
        placeholder="Type a message..."
      />
      {isLoading && <p>Loading...</p>}
      {error && <p>Error: {error.message}</p>}
      {data && <p>Echo: {data}</p>}

      <h2>Add</h2>
      <button
        disabled={!echo}
        onClick={async () => {
          if (echo) setSum(await echo.add(2, 3));
        }}
      >
        Add 2 + 3
      </button>
      {sum !== null && <p>Sum: {sum}</p>}
    </div>
  );
}

export default function App() {
  return (
    <JetStreamProvider url={SERVER_URL}>
      <EchoDemo />
    </JetStreamProvider>
  );
}
```

## Package Structure

| Package | Description |
|---------|-------------|
| `@sevki/jetstream-wireformat` | Binary codecs for primitives and composite types |
| `@sevki/jetstream-rpc` | RPC runtime: `Mux`, `TagPool`, framing, version negotiation |
| `@sevki/jetstream-react` | React hooks: `JetStreamProvider`, `useJetStream`, `useRPC`, `useHandler` |
