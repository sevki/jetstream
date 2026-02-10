# Swift WireFormat Implementation Spec

This specification defines the rules for implementing the JetStream WireFormat in Swift. The wire format is based on the [9P2000.L](http://man.cat-v.org/plan_9/5/0intro) protocol. All rules reference the base wireformat specification in `wireformat.md`. The implementation MUST produce byte-identical output to the Rust implementation for all types and MUST be wire-compatible with any compliant 9P2000.L implementation.

r[jetstream.wireformat.swift.9p-compat]
9P2000.L Wire Compatibility

The Swift implementation MUST produce wire output that is byte-identical to the Rust 9P-based JetStream implementation. This means:
- All multi-byte integers MUST use little-endian byte order (matching 9P2000.L conventions).
- Strings MUST use the 9P "string[s]" encoding: a UInt16 byte count followed by UTF-8 data.
- Data buffers MUST use the 9P "data[count]" encoding: a UInt32 byte count followed by raw bytes.
- The Swift implementation MUST be interoperable with the Rust `jetstream_wireformat` crate.

## Core Abstractions

r[jetstream.wireformat.swift.protocol]
WireFormat Protocol

Implement a `WireFormat` protocol with three required methods:

```swift
protocol WireFormat {
    func byteSize() -> UInt32
    func encode(writer: inout BinaryWriter) throws
    static func decode(reader: inout BinaryReader) throws -> Self
}
```

All type implementations MUST conform to this protocol. `encode` and `decode` MUST throw on I/O errors or invalid data.

r[jetstream.wireformat.swift.reader]
BinaryReader Struct

Implement a `BinaryReader` struct that reads from a `Data` object with a cursor position:

```swift
struct BinaryReader {
    private let data: Data
    private(set) var offset: Int

    init(data: Data)
    mutating func readBytes(count: Int) throws -> Data
    mutating func readU8() throws -> UInt8
    mutating func readU16() throws -> UInt16  // little-endian
    mutating func readU32() throws -> UInt32  // little-endian
    mutating func readU64() throws -> UInt64  // little-endian
    mutating func readI16() throws -> Int16   // little-endian
    mutating func readI32() throws -> Int32   // little-endian
    mutating func readI64() throws -> Int64   // little-endian
    mutating func readF32() throws -> Float   // little-endian
    mutating func readF64() throws -> Double  // little-endian
    var remaining: Int { get }
}
```

The reader MUST throw if attempting to read beyond the data. Use `withUnsafeBytes` and `littleEndian` conversions for multi-byte reads.

r[jetstream.wireformat.swift.writer]
BinaryWriter Struct

Implement a `BinaryWriter` struct that writes to a growable `Data` buffer:

```swift
struct BinaryWriter {
    private(set) var data: Data

    init(capacity: Int = 256)
    mutating func writeBytes(_ bytes: Data)
    mutating func writeU8(_ value: UInt8)
    mutating func writeU16(_ value: UInt16)  // little-endian
    mutating func writeU32(_ value: UInt32)  // little-endian
    mutating func writeU64(_ value: UInt64)  // little-endian
    mutating func writeI16(_ value: Int16)   // little-endian
    mutating func writeI32(_ value: Int32)   // little-endian
    mutating func writeI64(_ value: Int64)   // little-endian
    mutating func writeF32(_ value: Float)   // little-endian
    mutating func writeF64(_ value: Double)  // little-endian
}
```

Use `withUnsafeBytes(of: value.littleEndian)` for converting multi-byte values to little-endian bytes before appending to data.

## Primitive Types

r[jetstream.wireformat.swift.u8]
Encode/Decode UInt8

Encode as a single byte. Decode by reading one byte. Maps to Swift `UInt8`. Implements **WF-U8**.

r[jetstream.wireformat.swift.u16]
Encode/Decode UInt16

Encode using `UInt16.littleEndian`. Decode using `UInt16(littleEndian:)`. Maps to Swift `UInt16`. Implements **WF-U16**.

r[jetstream.wireformat.swift.u32]
Encode/Decode UInt32

Encode using `UInt32.littleEndian`. Decode using `UInt32(littleEndian:)`. Maps to Swift `UInt32`. Implements **WF-U32**.

r[jetstream.wireformat.swift.u64]
Encode/Decode UInt64

Encode using `UInt64.littleEndian`. Decode using `UInt64(littleEndian:)`. Maps to Swift `UInt64`. Implements **WF-U64**.

r[jetstream.wireformat.swift.u128]
Encode/Decode UInt128

Swift does not have a native UInt128 type (prior to Swift 6). Implement using a custom struct or tuple `(low: UInt64, high: UInt64)`. Encode the lower 64 bits first (bytes 0-7) as UInt64 LE, then the upper 64 bits (bytes 8-15) as UInt64 LE. On decode, read low then high UInt64 values. Implements **WF-U128**.

Note: If targeting Swift 6+, the built-in `UInt128` type may be used directly with `littleEndian` conversion.

r[jetstream.wireformat.swift.i16]
Encode/Decode Int16

Encode using `Int16.littleEndian`. Decode using `Int16(littleEndian:)`. Maps to Swift `Int16`. Implements **WF-I16**.

r[jetstream.wireformat.swift.i32]
Encode/Decode Int32

Encode using `Int32.littleEndian`. Decode using `Int32(littleEndian:)`. Maps to Swift `Int32`. Implements **WF-I32**.

r[jetstream.wireformat.swift.i64]
Encode/Decode Int64

Encode using `Int64.littleEndian`. Decode using `Int64(littleEndian:)`. Maps to Swift `Int64`. Implements **WF-I64**.

r[jetstream.wireformat.swift.i128]
Encode/Decode Int128

Similar to UInt128, use a custom struct or `(low: UInt64, high: Int64)` representation. Encode: write lower 64 bits as UInt64 LE, then upper 64 bits as Int64 LE (preserving the sign). Decode: read UInt64 low, Int64 high. Implements **WF-I128**.

Note: If targeting Swift 6+, the built-in `Int128` type may be used.

r[jetstream.wireformat.swift.f32]
Encode/Decode Float

Encode: convert `Float` to its bit pattern via `Float.bitPattern` (UInt32), encode as UInt32 LE. Decode: read UInt32 LE, convert to Float via `Float(bitPattern:)`. Maps to Swift `Float`. Implements **WF-F32**.

r[jetstream.wireformat.swift.f64]
Encode/Decode Double

Encode: convert `Double` to its bit pattern via `Double.bitPattern` (UInt64), encode as UInt64 LE. Decode: read UInt64 LE, convert to Double via `Double(bitPattern:)`. Maps to Swift `Double`. Implements **WF-F64**.

r[jetstream.wireformat.swift.bool]
Encode/Decode Bool

Encode `false` as `0x00` and `true` as `0x01`. Decode MUST reject any value other than 0 or 1 by throwing an error. Maps to Swift `Bool`. Implements **WF-BOOL**.

r[jetstream.wireformat.swift.unit]
Encode/Decode Void

The unit type writes and reads zero bytes. May be represented as `Void` or an empty struct in Swift. Implements **WF-UNIT**.

## String

r[jetstream.wireformat.swift.string]
Encode/Decode String

Encode: convert the `String` to UTF-8 bytes via `.utf8`. If the byte count exceeds 65,535, throw an error. Write the byte count as UInt16 LE, then write the UTF-8 bytes.

Decode: read UInt16 length N, read N bytes, decode as UTF-8 via `String(data:encoding:.utf8)`. If decoding fails, throw an invalid data error. Maps to Swift `String`. Implements **WF-STRING**.

## Collections

r[jetstream.wireformat.swift.array]
Encode/Decode Array

Use Swift `Array<T>` (`[T]`). Encode: if count > 65,535, throw. Write count as UInt16 LE, encode each element. Decode: read UInt16 count, decode that many elements into an array. Implements **WF-VEC**.

r[jetstream.wireformat.swift.data]
Encode/Decode Data (byte buffer)

Use Swift `Data` for the Data type. Encode: write byte count as UInt32 LE, then write raw bytes. Decode: read UInt32 length, reject if > 33,554,432 (32 MB), read that many bytes. If fewer than N bytes are available, throw an unexpected EOF error. Implements **WF-DATA**.

r[jetstream.wireformat.swift.dict]
Encode/Decode Dictionary (ordered)

Since Swift `Dictionary` is unordered, use a sorted array of key-value tuples or a custom `OrderedDictionary` type that maintains sorted order. Encode: if count > 65,535, throw. Write count as UInt16 LE, encode key-value pairs in sorted key order. Decode: read UInt16 count, decode key-value pairs, insert into collection. Entries MUST be emitted in sorted key order when encoding. Implements **WF-MAP**.

r[jetstream.wireformat.swift.set]
Encode/Decode Set (ordered)

Since Swift `Set` is unordered, encode elements in sorted order. May use a custom `OrderedSet` type or sort before encoding. Encode: if count > 65,535, throw. Write count as UInt16 LE, encode elements in sorted order. Decode: read UInt16 count, decode elements. Implements **WF-SET**.

## Option Type

r[jetstream.wireformat.swift.optional]
Encode/Decode Optional

Use Swift `Optional<T>` (`T?`). Encode: if `nil`, write `0x00`; if `.some(value)`, write `0x01` followed by the encoded value. Decode: read UInt8 tag; if 0, return `nil`; if 1, decode and return the value; otherwise throw an error. Implements **WF-OPTION**.

## Composite Types

r[jetstream.wireformat.swift.struct]
Encode/Decode Structs

Represent structs as Swift structs conforming to the `WireFormat` protocol. Fields MUST be encoded and decoded sequentially in declaration order (matching the Rust struct field order). There is no length prefix or field name on the wire. Implements **WF-STRUCT**.

r[jetstream.wireformat.swift.enum]
Encode/Decode Enums

Use Swift enums with associated values. Each variant is assigned a UInt8 index (0-based) in declaration order. Encode: write the variant index as UInt8, then encode the variant's associated values. Decode: read UInt8 variant index, decode the variant's associated values, construct the enum case. Throw on unknown variant index. Implements **WF-ENUM**.

Example:
```swift
enum Message: WireFormat {
    case ping                      // variant 0
    case text(content: String)     // variant 1
    case binary(data: Data)        // variant 2
}
```

## Network Types

r[jetstream.wireformat.swift.ipv4]
Encode/Decode IPv4

Represent using a struct with 4 bytes or use `in_addr` from `Darwin`/`Glibc`. Encode: write the 4 octets directly. Decode: read 4 bytes. May provide convenience conversion to/from string representation. Implements **WF-IPV4**.

r[jetstream.wireformat.swift.ipv6]
Encode/Decode IPv6

Represent using a struct with 16 bytes or use `in6_addr` from `Darwin`/`Glibc`. Encode: write the 16 octets directly. Decode: read 16 bytes. Implements **WF-IPV6**.

r[jetstream.wireformat.swift.ipaddr]
Encode/Decode IpAddr

Represent as a Swift enum:
```swift
enum IpAddr: WireFormat {
    case v4(IPv4Address)   // tag = 4
    case v6(IPv6Address)   // tag = 6
}
```

Note: the tag values are `4` and `6` (NOT 0 and 1 like standard enum encoding). Encode: write UInt8 tag (4 or 6), then write address bytes. Decode: read UInt8 tag, read address bytes. Throw on invalid tag. Implements **WF-IPADDR**.

r[jetstream.wireformat.swift.sockaddr]
Encode/Decode SocketAddr

Represent as a Swift enum:
```swift
enum SocketAddr: WireFormat {
    case v4(ip: IPv4Address, port: UInt16)   // tag = 4
    case v6(ip: IPv6Address, port: UInt16)   // tag = 6
}
```

Encode: write UInt8 tag (4 or 6), encode IP address, encode port as UInt16 LE. Decode: read tag, decode address, decode port. Note: SocketAddrV6's flow info and scope ID are NOT on the wire (set to 0 on decode). Throw on invalid tag. Implements **WF-SOCKADDR**.

## Time

r[jetstream.wireformat.swift.systime]
Encode/Decode SystemTime

Represent as Swift `Date`. Encode: compute milliseconds since Unix epoch via `Date.timeIntervalSince1970 * 1000`, cast to UInt64, encode as UInt64 LE. Decode: read UInt64, construct `Date(timeIntervalSince1970: Double(millis) / 1000.0)`. Implements **WF-SYSTIME**.

## Error Types

r[jetstream.wireformat.swift.error-inner]
ErrorInner Struct

Implement an `ErrorInner` struct conforming to `WireFormat`:
- `message: String` — encoded as WF-STRING
- `code: String?` — encoded as WF-OPTION of WF-STRING
- `help: String?` — encoded as WF-OPTION of WF-STRING
- `url: String?` — encoded as WF-OPTION of WF-STRING

Encode and decode these fields sequentially in the order listed. Implements **WF-ERR-INNER**.

r[jetstream.wireformat.swift.backtrace]
Backtrace Struct

Implement a `Backtrace` struct conforming to `WireFormat`:
- `internTable: [String]` — encoded as WF-VEC of WF-STRING
- `frames: [Frame]` — encoded as WF-VEC of WF-FRAME

Index 0 of the intern table is reserved for the empty string "". Implements **WF-BACKTRACE**.

r[jetstream.wireformat.swift.frame]
Frame Struct

Implement a `Frame` struct conforming to `WireFormat` with fields encoded in this exact order:
1. `msg: String` — WF-STRING
2. `name: UInt16` — WF-U16 (intern table index)
3. `target: UInt16` — WF-U16 (intern table index)
4. `module: UInt16` — WF-U16 (intern table index)
5. `file: UInt16` — WF-U16 (intern table index)
6. `line: UInt16` — WF-U16
7. `fields: [FieldPair]` — WF-VEC of WF-FIELDPAIR
8. `level: Level` — WF-LEVEL (custom codec)

Implements **WF-FRAME**.

r[jetstream.wireformat.swift.fieldpair]
FieldPair Struct

Implement a `FieldPair` struct conforming to `WireFormat`:
- `key: UInt16` — WF-U16 (intern table index)
- `value: UInt16` — WF-U16 (intern table index)

Implements **WF-FIELDPAIR**.

r[jetstream.wireformat.swift.level]
Level Enum

Define a `Level` enum with raw UInt8 values:

```swift
enum Level: UInt8, WireFormat {
    case trace = 0
    case debug = 1
    case info = 2
    case warn = 3
    case error = 4
}
```

Encode as UInt8. Decode: read UInt8, convert to Level via `Level(rawValue:)`, throw on invalid value. Implements **WF-LEVEL**.

r[jetstream.wireformat.swift.error]
JetStreamError

Implement a `JetStreamError` class/struct conforming to both `WireFormat` and Swift's `Error` protocol:
- `inner: ErrorInner` — encoded per SW-WF-ERR-INNER
- `backtrace: Backtrace` — encoded per SW-WF-BACKTRACE

Encode: encode inner, then backtrace. Decode: decode inner, then backtrace. Implements **WF-ERROR**.

## Package Structure

r[jetstream.wireformat.swift.package]
Swift Package

The Swift implementation MUST be organized as a Swift Package with:
- Source files under `Sources/JetStreamWireFormat/`
- Test files under `Tests/JetStreamWireFormatTests/`
- A `Package.swift` manifest

The package MUST support Apple platforms (iOS 15+, macOS 12+) and Linux.

## Testing Requirements

r[jetstream.wireformat.swift.test-roundtrip]
Round-trip Testing

Every type implementation MUST have tests that verify: encode(value) followed by decode(encodedBytes) produces an equal value. Tests MUST cover edge cases including empty strings, empty arrays, maximum lengths, nil values, and boundary values for numeric types.

r[jetstream.wireformat.swift.test-compat]
Cross-language Compatibility Testing

The Swift implementation MUST be tested against known byte sequences produced by the Rust implementation to verify binary compatibility. Test vectors SHOULD be generated from the Rust tests and verified in Swift.

r[jetstream.wireformat.swift.test-error]
Error Handling Tests

Tests MUST verify that decoding invalid data throws appropriate errors:
- Invalid bool byte (not 0 or 1)
- Invalid Optional tag (not 0 or 1)
- Invalid enum variant index (out of range)
- Invalid Level byte (> 4)
- String length exceeding 65,535
- Data length exceeding 32 MB
- Unexpected EOF (truncated data)
- Invalid UTF-8 in strings
- Invalid IP address tag (not 4 or 6)
