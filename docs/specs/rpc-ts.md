# JetStream TypeScript RPC Runtime Specification

This document specifies the TypeScript RPC runtime that provides multiplexed, tag-based request/response communication over binary frames. The runtime supports both upstream and downstream roles on a per-stream basis. **Upstream** points away from the keyboard (rack, cloud). **Downstream** points towards the keyboard (browser, local app). A client `Mux` sends request frames upstream and awaits responses. A handler receives incoming request frames, dispatches them, and sends response frames back to the caller.

## Frame Format

r[jetstream.rpc.ts.frame]
A `Frame<T>` encodes as `[size: u32 LE][type: u8][tag: u16 LE][payload]`. The `size` field includes itself, making the minimum frame size 7 bytes (4 + 1 + 2 + 0-byte payload). The `type` field identifies the message type. The `tag` field matches requests to responses.

## Framer Interface

r[jetstream.rpc.ts.framer]
The `Framer` interface requires: `messageType(): number` returning the u8 message type ID, `byteSize(): number` returning the encoded payload size, `encode(writer)` writing the payload bytes, and a static `decode(reader, type): T` that reads the payload given its type tag.

## Multiplexer

r[jetstream.rpc.ts.mux]
The client multiplexer maps allocated tags to pending `Promise` resolvers. When a request is sent, a tag is allocated and a Promise is created. When a response frame arrives, the multiplexer demultiplexes by tag, resolves the corresponding Promise, and recycles the tag.

## Transport

r[jetstream.rpc.ts.transport]
The `Transport` interface provides a `ReadableStream` for incoming frame bytes and a `WritableStream` for outgoing frame bytes. Implementations may wrap WebSocket, TCP, or other byte-oriented channels.

## Tag Pool

r[jetstream.rpc.ts.tag-pool]
The tag allocation pool manages `u16` tags in the range `1..max`. Tags are allocated for outgoing requests and recycled when the corresponding response is received. If no tags are available, the caller MUST wait until one is recycled.

## Message IDs

r[jetstream.rpc.ts.message-ids]
Message IDs start at `MESSAGE_ID_START = 102`. For each service method at index `i`, the request (Tmessage) ID is `102 + i * 2` and the response (Rmessage) ID is `103 + i * 2`. The error message type ID is `5`. IDs 100 and 101 are reserved for `TVERSION` and `RVERSION`.

## Error Frames

r[jetstream.rpc.ts.error-frame]
Error responses are encoded as `Rmessage::Error` with type ID `5`. The payload wraps a `JetStreamError` struct. When the multiplexer receives a frame with type `5`, it rejects the pending Promise for the corresponding tag with the decoded error.

## Protocol

r[jetstream.rpc.ts.protocol]
The `Protocol` interface defines the associated types and version for a service. It specifies `Request` and `Response` framer types, an `Error` type, and a `VERSION` string in the format `"rs.jetstream.proto/{service}/{version}-{digest}"`.

## Context

r[jetstream.rpc.ts.context]
The `Context` interface provides metadata about the RPC call, including the optional `remoteAddress` of the peer. Context is passed to handler methods and created by the caller when initiating requests.

## Client

r[jetstream.rpc.ts.client]
A generated client class wraps a `Mux` and exposes typed async methods matching the service trait. Each method constructs the appropriate `Tmessage` variant, sends it through the multiplexer, and returns the unwrapped `Rmessage` response or throws on error.

## Handler

r[jetstream.rpc.ts.handler]
A generated handler interface defines the methods a handler implementation must provide. Each method receives a `Context` and the decoded request parameters, and returns a `Promise` with the response value. This mirrors the Rust `Server` trait's `rpc()` dispatch pattern. The primary use case is handling upstream-pushed RPCs — like push notifications (APNS-style), cache invalidations, or real-time alerts — where upstream opens a stream and sends requests downstream.

```typescript
export interface NotificationHandler {
  notify(ctx: Context, title: string, body: string, badge: number): Promise<{ ack: boolean }>;
  invalidateCache(ctx: Context, keys: string[]): Promise<void>;
}
```

r[jetstream.rpc.ts.handler.dispatch]
A generated `dispatch` function takes a handler implementation and a `Frame<Tmessage>`, pattern-matches on the request message type, calls the corresponding handler method, wraps the result in the appropriate `Rmessage` variant, and returns a `Frame<Rmessage>`. On error, the result is wrapped as `Rmessage.Error` with type ID `5`. The response frame preserves the request's tag.

## Server Codec

r[jetstream.rpc.ts.server-codec]
The `ServerCodec<P>` decodes incoming bytes into `Frame<P.Request>` and encodes outgoing `Frame<P.Response>`. It reads the 4-byte `size` prefix to determine frame boundaries, then decodes the full frame. This is the TypeScript equivalent of the Rust `ServerCodec<P>` from `jetstream_rpc`.

## Server Loop

r[jetstream.rpc.ts.server-loop]
The server loop reads request frames from a `ReadableStream` (via `ServerCodec`), dispatches each to the handler's `dispatch` function, and writes response frames to a `WritableStream`. Requests are dispatched concurrently — each incoming frame is handed off immediately, and responses are sent back as they complete via a write queue. This matches the Rust pattern of `FramedRead` → concurrent `handler.rpc()` → `mpsc` channel → `FramedWrite`.

r[jetstream.rpc.ts.server-loop.transport]
The server loop accepts a `Transport` (the same interface used by the client `Mux`). On a WebTransport session, the browser listens for incoming bidi streams via `session.incomingBidirectionalStreams` and runs a server loop on each accepted stream. On other transports (WebSocket, TCP), the server loop runs on the single connection stream.

