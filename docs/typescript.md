# TypeScript

JetStream provides TypeScript packages for wire format encoding and RPC communication, published to the GitHub Package Registry under `@sevki`.

## Installation

```bash
# Configure npm to use GitHub Package Registry for @sevki scope
echo "@sevki:registry=https://npm.pkg.github.com" >> .npmrc

pnpm add @sevki/jetstream-wireformat @sevki/jetstream-rpc
```

## WireFormat Codecs

The `@sevki/jetstream-wireformat` package provides binary codecs for all JetStream primitive and composite types. Every codec implements the `WireFormat<T>` interface:

```typescript
interface WireFormat<T> {
  byteSize(value: T): number;
  encode(value: T, writer: BinaryWriter): void;
  decode(reader: BinaryReader): T;
}
```

### Primitives

```typescript
import { BinaryReader, BinaryWriter, u32Codec, stringCodec, boolCodec } from '@sevki/jetstream-wireformat';

// Encode a u32
const writer = new BinaryWriter();
u32Codec.encode(42, writer);
const bytes = writer.toUint8Array();

// Decode it back
const reader = new BinaryReader(bytes);
const value = u32Codec.decode(reader); // 42
```

Available primitive codecs: `u8Codec`, `u16Codec`, `u32Codec`, `u64Codec`, `i16Codec`, `i32Codec`, `i64Codec`, `f32Codec`, `f64Codec`, `boolCodec`, `stringCodec`.

### Composite Types

Build codecs for collections and optional types:

```typescript
import { vecCodec, optionCodec, mapCodec, stringCodec, u32Codec } from '@sevki/jetstream-wireformat';

// Vec<String>
const tagsCodec = vecCodec(stringCodec);

// Option<u32>
const maybeIdCodec = optionCodec(u32Codec);

// HashMap<String, u32>
const scoresCodec = mapCodec(stringCodec, u32Codec);
```

### Structs and Enums

Use the `structCodec` and `enumCodec` helpers, or generate codecs from Rust types using `jetstream_codegen`:

```typescript
import { BinaryReader, BinaryWriter, u32Codec } from '@sevki/jetstream-wireformat';
import type { WireFormat } from '@sevki/jetstream-wireformat';

// A manually defined codec for a Point struct
interface Point {
  x: number;
  y: number;
}

const pointCodec: WireFormat<Point> = {
  byteSize(value: Point): number {
    return u32Codec.byteSize(value.x) + u32Codec.byteSize(value.y);
  },
  encode(value: Point, writer: BinaryWriter): void {
    u32Codec.encode(value.x, writer);
    u32Codec.encode(value.y, writer);
  },
  decode(reader: BinaryReader): Point {
    const x = u32Codec.decode(reader);
    const y = u32Codec.decode(reader);
    return { x, y };
  },
};
```

## Code Generation

Instead of writing codecs by hand, use `jetstream_codegen` to generate TypeScript types and codecs from Rust source files:

```bash
cargo run -p jetstream_codegen -- \
  --input src/types.rs \
  --ts-out generated/
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

The codegen produces TypeScript interfaces and `WireFormat<T>` codec objects that are wire-compatible with the Rust implementations.

## RPC

The `@sevki/jetstream-rpc` package provides the RPC runtime for multiplexed request/response communication.

### Generated Code

The codegen generates RPC client and handler types from `#[service]` trait definitions:

```rust
// Rust service definition
#[service]
pub trait EchoHttp {
    async fn ping(&mut self, message: String) -> Result<String>;
    async fn add(&mut self, a: i32, b: i32) -> Result<i32>;
}
```

```bash
cargo run -p jetstream_codegen -- \
  --input examples/http.rs \
  --ts-out generated/
```

This generates:

- **Request/response types**: `TPing`, `RPing`, `TAdd`, `RAdd` with codecs
- **Frame unions**: `Tmessage`, `Rmessage` discriminated unions with `FramerCodec` implementations
- **Framer wrappers**: `TmessageFramer`, `RmessageFramer` classes implementing the `Framer` interface
- **`rmessageDecode`**: A decoder function for use with `Mux` and `WebTransportTransport`
- **Protocol constants**: `PROTOCOL_NAME` (e.g., `'rs.jetstream.proto/echohttp'`) and `PROTOCOL_VERSION` (e.g., `'rs.jetstream.proto/echohttp/15.0.0+bfd7d20e'`)
- **`EchoHttpClient`**: A typed client class with async methods and version negotiation
- **`EchoHttpHandler`**: A handler interface for implementing server-side dispatch
- **`dispatchEchoHttp`**: A dispatch function that routes `Tmessage` frames to handler methods

### Version Negotiation

Before making RPC calls, clients must perform a Tversion/Rversion handshake to negotiate the protocol version and maximum message size. The generated client provides a static `negotiate` method:

```typescript
import { EchoHttpClient, rmessageDecode, PROTOCOL_NAME } from './generated/echohttp_rpc.js';

// Open a WebTransport session and bidi stream
const session = new WebTransport(`https://api.example.com:4433/${PROTOCOL_NAME}`);
await session.ready;
const stream = await session.createBidirectionalStream();

// Negotiate version on the raw stream before creating the Mux
const negotiated = await EchoHttpClient.negotiate(stream.readable, stream.writable);
console.log(`Negotiated: ${negotiated.version}, msize: ${negotiated.msize}`);
```

You can also call `negotiateVersion` directly from `@sevki/jetstream-rpc`:

```typescript
import { negotiateVersion } from '@sevki/jetstream-rpc';
import { PROTOCOL_VERSION } from './generated/echohttp_rpc.js';

const negotiated = await negotiateVersion(stream.readable, stream.writable, PROTOCOL_VERSION);
```

After negotiation, the stream is ready for Mux framing.

### Client Usage

```typescript
import { Mux } from '@sevki/jetstream-rpc';
import { EchoHttpClient, rmessageDecode, PROTOCOL_NAME } from './generated/echohttp_rpc.js';

// 1. Connect
const session = new WebTransport(`https://api.example.com:4433/${PROTOCOL_NAME}`);
await session.ready;
const stream = await session.createBidirectionalStream();

// 2. Negotiate version
await EchoHttpClient.negotiate(stream.readable, stream.writable);

// 3. Create transport and mux
const transport = new WebTransportTransport(stream, rmessageDecode);
const mux = new Mux(transport);
await mux.start();

// 4. Create client and make RPC calls
const client = new EchoHttpClient(mux);
const reply = await client.ping("hello");  // "hello"
const sum = await client.add(2, 3);        // 5

// 5. Cleanup
await mux.close();
session.close();
```

### Handler (Server-Side)

Implement the generated handler interface to serve RPCs:

```typescript
import { EchoHttpHandler, dispatchEchoHttp } from './generated/echohttp_rpc.js';

const handler: EchoHttpHandler = {
  async ping(ctx, message) {
    return message; // echo back
  },
  async add(ctx, a, b) {
    return a + b;
  },
};
```

### Frame Wire Format

RPC frames follow the format `[size:u32 LE][type:u8][tag:u16 LE][payload]` where size includes itself (minimum 7 bytes). The tag field enables multiplexing concurrent requests over a single connection.

Special message types:
- `TVERSION` (100) / `RVERSION` (101) — version negotiation
- `MESSAGE_ID_START` (102) — first service method ID
- `RJETSTREAMERROR` (5) — error response frames

### Mux

The `Mux` class handles tag allocation, request/response matching, and concurrent RPC dispatch:

```typescript
import { Mux } from '@sevki/jetstream-rpc';

const mux = new Mux(transport);
await mux.start();

// Each rpc() call acquires a tag, sends a frame, waits for the matching response, and releases the tag
const response = await mux.rpc(request);

await mux.close();
```

### Protocol Interface

Every generated service implements the `Protocol` interface:

```typescript
interface Protocol<TReq extends Framer, TRes extends Framer> {
  readonly VERSION: string;
  readonly NAME: string;
}
```

- `NAME` is the protocol name used for routing (e.g., URI path, ALPN)
- `VERSION` is the full version string used during Tversion/Rversion negotiation
