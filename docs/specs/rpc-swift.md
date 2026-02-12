# JetStream Swift RPC Runtime Specification

This document specifies the Swift RPC runtime that provides multiplexed, tag-based request/response communication over binary frames using Swift concurrency. The runtime supports both upstream and downstream roles on a per-stream basis. **Upstream** points away from the keyboard (rack, cloud). **Downstream** points towards the keyboard (app, local device). A client `Mux` sends request frames upstream and awaits responses. A handler receives incoming request frames, dispatches them, and sends response frames back to the caller.

## Frame Format

r[jetstream.rpc.swift.frame]
A `Frame<T: Framer>` struct conforming to `WireFormat` encodes as `[size: u32 LE][type: u8][tag: u16 LE][payload]`. The `size` field includes itself, making the minimum frame size 7 bytes (4 + 1 + 2 + 0-byte payload). The `type` field identifies the message type. The `tag` field matches requests to responses.

## Framer Protocol

r[jetstream.rpc.swift.framer]
The `Framer` protocol requires: `messageType() -> UInt8` returning the message type ID, `byteSize() -> UInt32` returning the encoded payload size, `encode(writer:)` writing the payload bytes, and a static `decode(reader:type:) -> Self` that reads the payload given its type tag.

## Multiplexer

r[jetstream.rpc.swift.mux]
The client multiplexer uses Swift concurrency (`async/await` with `CheckedContinuation`) to map allocated tags to pending continuations. When a request is sent, a tag is allocated and an async suspension point is created. When a response frame arrives, the multiplexer demultiplexes by tag, resumes the corresponding continuation, and recycles the tag.

## Transport

r[jetstream.rpc.swift.transport]
The `Transport` protocol provides an `AsyncSequence` of incoming frames and an `async send` method for outgoing frames. Implementations may wrap NIO channels, URLSession WebSocket, or other async byte-oriented channels.

## Tag Pool

r[jetstream.rpc.swift.tag-pool]
The tag allocation pool uses actor isolation for thread safety. It manages `UInt16` tags in the range `1..max`. Tags are allocated for outgoing requests and recycled when the corresponding response is received. If no tags are available, the caller suspends until one is recycled.

## Message IDs

r[jetstream.rpc.swift.message-ids]
Message IDs start at `MESSAGE_ID_START = 102`. For each service method at index `i`, the request (Tmessage) ID is `102 + i * 2` and the response (Rmessage) ID is `103 + i * 2`. The error message type ID is `5`. IDs 100 and 101 are reserved for `TVERSION` and `RVERSION`. This scheme is identical to the Rust implementation.

## Error Frames

r[jetstream.rpc.swift.error-frame]
Error responses are encoded as `Rmessage.error(JetStreamError)` with type ID `5`. When the multiplexer receives a frame with type `5`, it resumes the pending continuation for the corresponding tag by throwing the decoded error.

## Protocol

r[jetstream.rpc.swift.protocol]
The `JetStreamProtocol` protocol defines the associated types and version for a service. It requires `Request: Framer` and `Response: Framer` associated types, an `Error` type conforming to `Error & Sendable`, and a static `VERSION: String` constant in the format `"rs.jetstream.proto/{service}/{version}-{digest}"`.

## Context

r[jetstream.rpc.swift.context]
The `Context` struct provides metadata about the RPC call, including the optional `remoteAddress` of the peer. Context is `Sendable` and passed to handler methods. A `default` static instance is provided for locally-initiated calls.

## Client

r[jetstream.rpc.swift.client]
A generated client class wraps a `Mux` and exposes typed async methods matching the service protocol. Each method constructs the appropriate `Tmessage` case, sends it through the multiplexer, and returns the unwrapped `Rmessage` response or throws the decoded error.

## Handler

r[jetstream.rpc.swift.handler]
A generated handler protocol defines the methods a handler implementation must provide. Each method receives a `Context` and the decoded request parameters, and returns the response value asynchronously. This mirrors the Rust `Server` trait's `rpc()` dispatch pattern. The primary use case is handling upstream-pushed RPCs — like push notifications (APNS-style), cache invalidations, or real-time alerts — where upstream opens a stream and sends requests downstream.

```swift
public protocol NotificationHandler: Sendable {
    func notify(ctx: Context, title: String, body: String, badge: Int32) async throws -> NotifyAck
    func invalidateCache(ctx: Context, keys: [String]) async throws
}
```

r[jetstream.rpc.swift.handler.dispatch]
A generated `dispatch` function takes a handler conforming to the protocol and a `Frame<Tmessage>`, switches on the request message case, calls the corresponding handler method, wraps the result in the appropriate `Rmessage` case, and returns a `Frame<Rmessage>`. On error, the result is wrapped as `Rmessage.error(JetStreamError)` with type ID `5`. The response frame preserves the request's tag.

## Server Codec

r[jetstream.rpc.swift.server-codec]
The `ServerCodec<P: JetStreamProtocol>` decodes incoming bytes from an `AsyncSequence` into `Frame<P.Request>` and encodes outgoing `Frame<P.Response>` for writing. It reads the 4-byte `size` prefix to determine frame boundaries, then decodes the full frame. This is the Swift equivalent of the Rust `ServerCodec<P>` from `jetstream_rpc`.

## Server Loop

r[jetstream.rpc.swift.server-loop]
The server loop reads request frames from the transport's `AsyncSequence` (via `ServerCodec`), dispatches each to the handler's `dispatch` function, and writes response frames via the transport's `send` method. Requests are dispatched concurrently using `TaskGroup` — each incoming frame spawns a child task, and responses are sent back as they complete. This matches the Rust pattern of `FramedRead` → concurrent `handler.rpc()` → `mpsc` channel → `FramedWrite`.

r[jetstream.rpc.swift.server-loop.transport]
The server loop accepts a `Transport` (the same protocol used by the client `Mux`). On an iroh connection, the server calls `connection.acceptBi()` to accept incoming bidi streams and runs a server loop on each. On other transports (NIO, URLSession), the server loop runs on the single connection stream.

