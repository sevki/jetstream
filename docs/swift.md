# Swift

JetStream provides Swift packages for wire format encoding and RPC communication via Swift Package Manager.

## Installation

Add the JetStream package to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/sevki/jetstream.git", from: "15.0.0"),
],
targets: [
    .target(
        name: "MyApp",
        dependencies: [
            .product(name: "JetStreamWireFormat", package: "jetstream"),
            .product(name: "JetStreamRpc", package: "jetstream"),
        ]
    ),
]
```

## WireFormat Protocol

The `JetStreamWireFormat` module defines the `WireFormat` protocol that all serializable types conform to:

```swift
public protocol WireFormat: Sized {
    func byteSize() -> UInt32
    func encode(writer: inout BinaryWriter) throws
    static func decode(reader: inout BinaryReader) throws -> Self
}
```

### Primitives

All Swift standard types (`UInt8`, `UInt16`, `UInt32`, `UInt64`, `Int16`, `Int32`, `Int64`, `Float`, `Double`, `Bool`, `String`) conform to `WireFormat`:

```swift
import JetStreamWireFormat

// Encode a UInt32
var writer = BinaryWriter()
try UInt32(42).encode(writer: &writer)
let bytes = writer.bytes

// Decode it back
var reader = BinaryReader(data: bytes)
let value = try UInt32.decode(reader: &reader) // 42
```

### Collections and Optionals

Arrays, Dictionaries, Sets, and Optionals conform to `WireFormat` when their elements do:

```swift
// Array<String>
let tags: [String] = ["hello", "world"]
try tags.encode(writer: &writer)

// Optional<UInt32>
let maybeId: UInt32? = 42
try maybeId.encode(writer: &writer)

// Dictionary<String, UInt32>
let scores: [String: UInt32] = ["alice": 100, "bob": 200]
try scores.encode(writer: &writer)
```

### Structs and Enums

Define custom types conforming to `WireFormat`, or generate them from Rust types using `jetstream_codegen`:

```swift
import JetStreamWireFormat

public struct Point: WireFormat {
    public var x: UInt32
    public var y: UInt32

    public func byteSize() -> UInt32 {
        return x.byteSize() + y.byteSize()
    }

    public func encode(writer: inout BinaryWriter) throws {
        try x.encode(writer: &writer)
        try y.encode(writer: &writer)
    }

    public static func decode(reader: inout BinaryReader) throws -> Point {
        let x = try UInt32.decode(reader: &reader)
        let y = try UInt32.decode(reader: &reader)
        return Point(x: x, y: y)
    }
}
```

## Code Generation

Instead of writing conformances by hand, use `jetstream_codegen` to generate Swift types from Rust source files:

```bash
cargo run -p jetstream_codegen -- \
  --input src/types.rs \
  --swift-out generated/
```

Given a Rust file:

```rust
#[derive(JetStreamWireFormat)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[derive(JetStreamWireFormat)]
pub enum Shape {
    Circle(u32),
    Rectangle { width: u32, height: u32 },
}
```

The codegen produces Swift structs and enums conforming to `WireFormat` that are wire-compatible with the Rust implementations.

## RPC

The `JetStreamRpc` module provides the RPC runtime for multiplexed request/response communication.

### Generated Client

The codegen generates RPC client classes and handler protocols from `#[service]` trait definitions:

```rust
// Rust service definition
#[service]
pub trait Echo {
    async fn ping(&mut self, message: String) -> Result<String>;
    async fn add(&mut self, a: u32, b: u32) -> Result<u32>;
}
```

```bash
cargo run -p jetstream_codegen -- \
  --input examples/echo.rs \
  --swift-out generated/
```

This generates a typed `EchoClient` class and an `EchoHandler` protocol:

```swift
// Generated client usage
let client = EchoClient(mux: mux)
let reply = try await client.ping(message: "hello")
let sum = try await client.add(a: 2, b: 3)
```

```swift
// Generated handler protocol — implement this for your server
public protocol EchoHandler {
    func ping(message: String) async throws -> String
    func add(a: UInt32, b: UInt32) async throws -> UInt32
}
```

### Framer Protocol

The `Framer` protocol handles message type dispatch for RPC envelopes:

```swift
public protocol Framer {
    func messageType() -> UInt8
    func byteSize() -> UInt32
    func encode(writer: inout BinaryWriter) throws
    static func decode(reader: inout BinaryReader, type: UInt8) throws -> Self
}
```

Generated `Tmessage` and `Rmessage` enums conform to `Framer` and dispatch encoding/decoding based on the message type byte.

### Frame Wire Format

RPC frames follow the format `[size:u32 LE][type:u8][tag:u16 LE][payload]` where size includes itself (minimum 7 bytes). The tag field enables multiplexing concurrent requests over a single connection.

### Mux

The `Mux` class handles tag allocation, request/response matching, and concurrent RPC dispatch:

```swift
let mux = Mux(transport: transport)

// Each rpc() call acquires a tag, sends a frame, waits for the matching response, and releases the tag
let response = try await mux.rpc(request)
```
