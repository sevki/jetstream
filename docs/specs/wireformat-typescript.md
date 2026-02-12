# TypeScript WireFormat Implementation Spec

This specification defines the rules for implementing the JetStream WireFormat in TypeScript. The wire format is based on the [9P2000.L](http://man.cat-v.org/plan_9/5/0intro) protocol. All rules reference the base wireformat specification in `wireformat.md`. The implementation MUST produce byte-identical output to the Rust implementation for all types and MUST be wire-compatible with any compliant 9P2000.L implementation.

r[jetstream.wireformat.ts.9p-compat]
9P2000.L Wire Compatibility

The TypeScript implementation MUST produce wire output that is byte-identical to the Rust 9P-based JetStream implementation. This means:
- All multi-byte integers MUST use little-endian byte order (matching 9P2000.L conventions).
- Strings MUST use the 9P "string[s]" encoding: a u16 byte count followed by UTF-8 data.
- Data buffers MUST use the 9P "data[count]" encoding: a u32 byte count followed by raw bytes.
- The TypeScript implementation MUST be interoperable with the Rust `jetstream_wireformat` crate.

## Core Abstractions

r[jetstream.wireformat.ts.interface]
WireFormat Interface

Implement a `WireFormat<T>` interface with three methods:

```typescript
interface WireFormat<T> {
  byteSize(value: T): number;
  encode(value: T, writer: BinaryWriter): void;
  decode(reader: BinaryReader): T;
}
```

All type implementations MUST conform to this interface. The `byteSize` method MUST return a `number` (u32 equivalent). The `encode` method MUST throw on I/O errors. The `decode` method MUST throw on invalid data or unexpected EOF.

r[jetstream.wireformat.ts.reader]
BinaryReader Class

Implement a `BinaryReader` class that reads from a `Uint8Array` (or Node.js `Buffer`) with a cursor position:

```typescript
class BinaryReader {
  private buffer: Uint8Array;
  private offset: number;

  constructor(buffer: Uint8Array);
  readBytes(count: number): Uint8Array;
  readU8(): number;
  readU16(): number;  // little-endian
  readU32(): number;  // little-endian
  readU64(): bigint;  // little-endian, BigInt
  readI16(): number;  // little-endian, signed
  readI32(): number;  // little-endian, signed
  readI64(): bigint;  // little-endian, signed BigInt
  readF32(): number;  // little-endian, IEEE 754
  readF64(): number;  // little-endian, IEEE 754
  readonly remaining: number;
}
```

The reader MUST throw if attempting to read beyond the buffer. Use `DataView` with `littleEndian = true` for multi-byte reads.

r[jetstream.wireformat.ts.writer]
BinaryWriter Class

Implement a `BinaryWriter` class that writes to a growable buffer:

```typescript
class BinaryWriter {
  private buffer: Uint8Array;
  private offset: number;

  constructor(initialCapacity?: number);
  writeBytes(bytes: Uint8Array): void;
  writeU8(value: number): void;
  writeU16(value: number): void;  // little-endian
  writeU32(value: number): void;  // little-endian
  writeU64(value: bigint): void;  // little-endian, BigInt
  writeI16(value: number): void;  // little-endian, signed
  writeI32(value: number): void;  // little-endian, signed
  writeI64(value: bigint): void;  // little-endian, signed BigInt
  writeF32(value: number): void;  // little-endian, IEEE 754
  writeF64(value: number): void;  // little-endian, IEEE 754
  toUint8Array(): Uint8Array;
}
```

The writer MUST grow its internal buffer when capacity is exceeded. Use `DataView` with `littleEndian = true` for multi-byte writes.

## Primitive Types

r[jetstream.wireformat.ts.u8]
Encode/Decode u8

Encode as a single byte using `DataView.setUint8()`. Decode using `DataView.getUint8()`. Maps to JavaScript `number`. Implements **WF-U8**.

r[jetstream.wireformat.ts.u16]
Encode/Decode u16

Encode as 2 bytes LE using `DataView.setUint16(offset, value, true)`. Decode using `DataView.getUint16(offset, true)`. Maps to JavaScript `number`. Implements **WF-U16**.

r[jetstream.wireformat.ts.u32]
Encode/Decode u32

Encode as 4 bytes LE using `DataView.setUint32(offset, value, true)`. Decode using `DataView.getUint32(offset, true)`. Maps to JavaScript `number`. Implements **WF-U32**.

r[jetstream.wireformat.ts.u64]
Encode/Decode u64

Encode as 8 bytes LE using `DataView.setBigUint64(offset, value, true)`. Decode using `DataView.getBigUint64(offset, true)`. MUST use JavaScript `BigInt` type. Implements **WF-U64**.

r[jetstream.wireformat.ts.u128]
Encode/Decode u128

Encode as 16 bytes LE. Since JavaScript has no native u128, encode as two `BigInt` values: write the lower 64 bits first (bytes 0-7), then the upper 64 bits (bytes 8-15), both as little-endian u64. On decode, read lower 64 bits and upper 64 bits and combine: `upper << 64n | lower`. Use `BigInt` for the combined value. Implements **WF-U128**.

r[jetstream.wireformat.ts.i16]
Encode/Decode i16

Encode as 2 bytes LE using `DataView.setInt16(offset, value, true)`. Decode using `DataView.getInt16(offset, true)`. Maps to JavaScript `number`. Implements **WF-I16**.

r[jetstream.wireformat.ts.i32]
Encode/Decode i32

Encode as 4 bytes LE using `DataView.setInt32(offset, value, true)`. Decode using `DataView.getInt32(offset, true)`. Maps to JavaScript `number`. Implements **WF-I32**.

r[jetstream.wireformat.ts.i64]
Encode/Decode i64

Encode as 8 bytes LE using `DataView.setBigInt64(offset, value, true)`. Decode using `DataView.getBigInt64(offset, true)`. MUST use JavaScript `BigInt` type. Implements **WF-I64**.

r[jetstream.wireformat.ts.i128]
Encode/Decode i128

Encode as 16 bytes LE. Write the lower 64 bits as unsigned u64 LE first (bytes 0-7), then the upper 64 bits as signed i64 LE (bytes 8-15). On decode, read lower u64 and upper i64 (signed) and combine: `upper << 64n | (lower & 0xFFFFFFFFFFFFFFFFn)`. Use `BigInt` for the combined value. Implements **WF-I128**.

r[jetstream.wireformat.ts.f32]
Encode/Decode f32

Encode as 4 bytes LE using `DataView.setFloat32(offset, value, true)`. Decode using `DataView.getFloat32(offset, true)`. Maps to JavaScript `number`. Note: f32 has reduced precision compared to f64. Implements **WF-F32**.

r[jetstream.wireformat.ts.f64]
Encode/Decode f64

Encode as 8 bytes LE using `DataView.setFloat64(offset, value, true)`. Decode using `DataView.getFloat64(offset, true)`. Maps to JavaScript `number` (which is natively f64). Implements **WF-F64**.

r[jetstream.wireformat.ts.bool]
Encode/Decode boolean

Encode `false` as `0x00` and `true` as `0x01`. Decode MUST reject any value other than 0 or 1 by throwing an error. Maps to JavaScript `boolean`. Implements **WF-BOOL**.

r[jetstream.wireformat.ts.unit]
Encode/Decode unit

The unit type writes and reads zero bytes. May be represented as `undefined` or `null` in TypeScript. Implements **WF-UNIT**.

## String

r[jetstream.wireformat.ts.string]
Encode/Decode String

Encode: convert the string to UTF-8 bytes using `TextEncoder.encode()`. If the resulting byte length exceeds 65,535, throw an error. Write the byte length as u16 (LE), then write the UTF-8 bytes.

Decode: read a u16 length N, read N bytes, decode using `TextDecoder.decode()` (which validates UTF-8). Maps to JavaScript `string`. Implements **WF-STRING**.

## Collections

r[jetstream.wireformat.ts.array]
Encode/Decode Array

Use JavaScript `Array<T>`. Encode: if length > 65,535, throw an error. Write the length as u16 (LE), then encode each element. Decode: read u16 count, decode that many elements into an array. Implements **WF-VEC**.

r[jetstream.wireformat.ts.data]
Encode/Decode Data (byte buffer)

Use `Uint8Array` for the Data type. Encode: write byte length as u32 (LE), then write raw bytes. Decode: read u32 length, reject if > 33,554,432 (32 MB), read that many bytes. Implements **WF-DATA**.

r[jetstream.wireformat.ts.map]
Encode/Decode Map

Use JavaScript `Map<K, V>` for ordered maps. Encode: if size > 65,535, throw. Write size as u16 (LE), then for each entry encode key then value. Decode: read u16 count, decode key-value pairs, insert into Map. Note: entries MUST be emitted in sorted key order when encoding to match Rust's BTreeMap behavior. Implements **WF-MAP**.

r[jetstream.wireformat.ts.set]
Encode/Decode Set

Use JavaScript `Set<T>` for ordered sets. Encode: if size > 65,535, throw. Write size as u16 (LE), then encode each element in sorted order. Decode: read u16 count, decode elements, insert into Set. Implements **WF-SET**.

## Option Type

r[jetstream.wireformat.ts.option]
Encode/Decode Option

Represent `Option<T>` as `T | null` in TypeScript. Encode: if `null`, write `0x00`; otherwise write `0x01` followed by the encoded value. Decode: read u8 tag; if 0, return `null`; if 1, decode and return the value; otherwise throw an error. Implements **WF-OPTION**.

## Composite Types

r[jetstream.wireformat.ts.struct]
Encode/Decode Structs

Represent structs as TypeScript classes or interfaces. Each struct type MUST define its own `byteSize`, `encode`, and `decode` functions that process fields sequentially in declaration order (matching the Rust struct field order). There is no length prefix or field name on the wire. Implements **WF-STRUCT**.

r[jetstream.wireformat.ts.enum]
Encode/Decode Enums

Represent enums as discriminated unions using a `tag` property. Each variant MUST have a `type` discriminant string for TypeScript type narrowing, but the wire encoding uses a u8 index (0-based). Encode: write the variant index as u8, then encode the variant's fields. Decode: read u8 variant index, decode the variant's fields, construct the discriminated union object. Throw on unknown variant index. Implements **WF-ENUM**.

Example:
```typescript
type Message =
  | { type: "ping" }                            // variant 0
  | { type: "text"; content: string }           // variant 1
  | { type: "binary"; data: Uint8Array };       // variant 2
```

## Network Types

r[jetstream.wireformat.ts.ipv4]
Encode/Decode IPv4

Represent as a `Uint8Array` of 4 bytes or a string (e.g., "192.168.1.1"). Encode: write the 4 octets directly. Decode: read 4 bytes. Implements **WF-IPV4**.

r[jetstream.wireformat.ts.ipv6]
Encode/Decode IPv6

Represent as a `Uint8Array` of 16 bytes or a string. Encode: write the 16 octets directly. Decode: read 16 bytes. Implements **WF-IPV6**.

r[jetstream.wireformat.ts.ipaddr]
Encode/Decode IpAddr

Represent as a discriminated union `{ version: 4, addr: Uint8Array } | { version: 6, addr: Uint8Array }`. Encode: write u8 tag (4 or 6), then write the address bytes. Decode: read u8 tag, read address bytes (4 for IPv4, 16 for IPv6), throw on invalid tag. Implements **WF-IPADDR**.

r[jetstream.wireformat.ts.sockaddr]
Encode/Decode SocketAddr

Represent as a discriminated union with `ip` and `port` fields. Encode: write u8 tag (4 or 6), encode IP address (4 or 16 bytes), then encode port as u16 LE. Decode: read tag, decode address, decode port. Throw on invalid tag. Implements **WF-SOCKADDR**.

## Time

r[jetstream.wireformat.ts.systime]
Encode/Decode SystemTime

Represent as a JavaScript `Date` object or a `BigInt` milliseconds value. Encode: get the milliseconds since Unix epoch via `Date.getTime()` and encode as u64 LE. Decode: read u64 as BigInt, convert to Number (safe up to 2^53-1 ms, which is ~285,000 years), construct `new Date(Number(millis))`. Implements **WF-SYSTIME**.

## Error Types

r[jetstream.wireformat.ts.error-inner]
ErrorInner Class

Implement an `ErrorInner` class/interface with fields:
- `message: string` — encoded as WF-STRING
- `code: string | null` — encoded as WF-OPTION of WF-STRING
- `help: string | null` — encoded as WF-OPTION of WF-STRING
- `url: string | null` — encoded as WF-OPTION of WF-STRING

Encode and decode these fields sequentially in the order listed. Implements **WF-ERR-INNER**.

r[jetstream.wireformat.ts.backtrace]
Backtrace Class

Implement a `Backtrace` class with fields:
- `internTable: string[]` — encoded as WF-VEC of WF-STRING
- `frames: Frame[]` — encoded as WF-VEC of WF-FRAME

Index 0 of the intern table is reserved for the empty string "". Implements **WF-BACKTRACE**.

r[jetstream.wireformat.ts.frame]
Frame Class

Implement a `Frame` class with fields encoded in this exact order:
1. `msg: string` — WF-STRING
2. `name: number` — WF-U16 (intern table index)
3. `target: number` — WF-U16 (intern table index)
4. `module: number` — WF-U16 (intern table index)
5. `file: number` — WF-U16 (intern table index)
6. `line: number` — WF-U16
7. `fields: FieldPair[]` — WF-VEC of WF-FIELDPAIR
8. `level: Level` — WF-LEVEL (custom codec)

Implements **WF-FRAME**.

r[jetstream.wireformat.ts.fieldpair]
FieldPair Class

Implement a `FieldPair` class with:
- `key: number` — WF-U16 (intern table index)
- `value: number` — WF-U16 (intern table index)

Implements **WF-FIELDPAIR**.

r[jetstream.wireformat.ts.level]
Level Enum

Define a `Level` enum:
```typescript
enum Level {
  TRACE = 0,
  DEBUG = 1,
  INFO = 2,
  WARN = 3,
  ERROR = 4,
}
```

Encode as u8. Decode: read u8, map to enum, throw on invalid value (> 4). Implements **WF-LEVEL**.

r[jetstream.wireformat.ts.error]
JetStreamError Class

Implement a `JetStreamError` class that extends `Error` with:
- `inner: ErrorInner` — encoded per TS-WF-ERR-INNER
- `backtrace: Backtrace` — encoded per TS-WF-BACKTRACE

Encode: encode inner, then backtrace. Decode: decode inner, then backtrace. Implements **WF-ERROR**.

## Testing Requirements

r[jetstream.wireformat.ts.test-roundtrip]
Round-trip Testing

Every type implementation MUST have tests that verify: encode(value) followed by decode(encodedBytes) produces an equal value. The tests MUST cover edge cases including empty strings, empty arrays, maximum lengths, None/null values, and boundary values for numeric types.

r[jetstream.wireformat.ts.test-compat]
Cross-language Compatibility Testing

The TypeScript implementation MUST be tested against known byte sequences produced by the Rust implementation to verify binary compatibility. Test vectors SHOULD be generated from the Rust tests and verified in TypeScript.

r[jetstream.wireformat.ts.test-error]
Error Handling Tests

Tests MUST verify that decoding invalid data throws appropriate errors:
- Invalid bool byte (not 0 or 1)
- Invalid Option tag (not 0 or 1)
- Invalid enum variant index (out of range)
- Invalid Level byte (> 4)
- String length exceeding 65,535
- Data length exceeding 32 MB
- Unexpected EOF (truncated data)
- Invalid UTF-8 in strings
- Invalid IP address tag (not 4 or 6)
