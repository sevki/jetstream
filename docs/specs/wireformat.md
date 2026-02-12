# JetStream WireFormat Specification

This document specifies the binary wire format used by JetStream for encoding and decoding types. The wire format is based on the [9P2000.L](http://man.cat-v.org/plan_9/5/0intro) protocol and MUST be wire-compatible with 9P2000.L implementations. All implementations (Rust, TypeScript, Swift) MUST conform to these rules to ensure interoperability with each other and with any compliant 9P2000.L implementation.

## 9P2000.L Compatibility

r[jetstream.wireformat.9p-compat]
9P2000.L Wire Compatibility

The JetStream wire format is derived from the 9P2000.L protocol. All primitive encodings (integers, strings, byte buffers) follow 9P2000.L conventions:

- All multi-byte integers are little-endian.
- Strings are encoded as a `u16` byte count followed by UTF-8 data (the 9P "string[s]" encoding).
- Repeated elements (vectors) use a `u16` count prefix followed by sequential elements.
- Data buffers use a `u32` byte count prefix (the 9P "data[count]" encoding).

Types that extend beyond the base 9P2000.L protocol (e.g., Option, Enum variant tags, 128-bit integers, network addresses, error backtraces) use consistent conventions that are compatible with the 9P encoding style (little-endian, length-prefixed).

Any implementation that correctly encodes/decodes per this specification MUST be able to exchange messages with the existing Rust 9P-based JetStream implementation.

## General

r[jetstream.wireformat.trait]
WireFormat Trait

Every type that can be transmitted over the wire MUST implement the WireFormat trait (or equivalent interface/protocol), providing three operations:

1. `byte_size() -> u32` — Returns the number of bytes required to encode the value.
2. `encode(writer) -> Result` — Writes the encoded bytes to a writer/buffer.
3. `decode(reader) -> Result<Self>` — Reads and decodes a value from a reader/buffer.

r[jetstream.wireformat.byte-order]
Little-Endian Byte Order

All multi-byte integers and floating-point numbers MUST be encoded in **little-endian** byte order. This applies to all integer types (u16, u32, u64, u128, i16, i32, i64, i128), floating-point types (f32, f64), and any length prefixes (e.g., string lengths, vector counts).

## Primitive Types

r[jetstream.wireformat.u8]
Unsigned 8-bit Integer

Encoded as a single byte. `byte_size` = 1.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 1    | The u8 value |

r[jetstream.wireformat.u16]
Unsigned 16-bit Integer

Encoded as 2 bytes in little-endian order. `byte_size` = 2.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 2    | The u16 value (LE) |

r[jetstream.wireformat.u32]
Unsigned 32-bit Integer

Encoded as 4 bytes in little-endian order. `byte_size` = 4.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 4    | The u32 value (LE) |

r[jetstream.wireformat.u64]
Unsigned 64-bit Integer

Encoded as 8 bytes in little-endian order. `byte_size` = 8.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 8    | The u64 value (LE) |

r[jetstream.wireformat.u128]
Unsigned 128-bit Integer

Encoded as 16 bytes in little-endian order. `byte_size` = 16.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 16   | The u128 value (LE) |

r[jetstream.wireformat.i16]
Signed 16-bit Integer

Encoded as 2 bytes in little-endian order (two's complement). `byte_size` = 2.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 2    | The i16 value (LE, two's complement) |

r[jetstream.wireformat.i32]
Signed 32-bit Integer

Encoded as 4 bytes in little-endian order (two's complement). `byte_size` = 4.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 4    | The i32 value (LE, two's complement) |

r[jetstream.wireformat.i64]
Signed 64-bit Integer

Encoded as 8 bytes in little-endian order (two's complement). `byte_size` = 8.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 8    | The i64 value (LE, two's complement) |

r[jetstream.wireformat.i128]
Signed 128-bit Integer

Encoded as 16 bytes in little-endian order (two's complement). `byte_size` = 16.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 16   | The i128 value (LE, two's complement) |

r[jetstream.wireformat.f32]
32-bit Floating Point

Encoded as 4 bytes in little-endian order using IEEE 754 binary32 representation. `byte_size` = 4.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 4    | The f32 value (LE, IEEE 754) |

r[jetstream.wireformat.f64]
64-bit Floating Point

Encoded as 8 bytes in little-endian order using IEEE 754 binary64 representation. `byte_size` = 8.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 8    | The f64 value (LE, IEEE 754) |

r[jetstream.wireformat.bool]
Boolean

Encoded as a single byte. `false` = `0x00`, `true` = `0x01`. Any other value MUST be rejected as invalid. `byte_size` = 1.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 1    | 0x00 = false, 0x01 = true |

r[jetstream.wireformat.unit]
Unit Type

The unit type (void/empty) encodes as zero bytes. `byte_size` = 0. Nothing is written to or read from the wire.

## Strings

r[jetstream.wireformat.string]
String Encoding

Strings are encoded as a **u16 length prefix** followed by the UTF-8 encoded string bytes. The length prefix indicates the number of bytes (NOT characters) in the UTF-8 data. The maximum string length is 65,535 bytes (u16::MAX). Strings MUST be valid UTF-8.

`byte_size` = 2 + len(utf8_bytes).

| Offset | Size    | Description |
|--------|---------|-------------|
| 0      | 2       | Length N as u16 (LE) |
| 2      | N       | UTF-8 encoded string bytes |

**Encoding:**
1. If the string byte length exceeds 65,535, encoding MUST fail with an error.
2. Write the byte length as a u16 (LE).
3. Write the raw UTF-8 bytes.

**Decoding:**
1. Read a u16 length N.
2. Read exactly N bytes.
3. Validate that the bytes are valid UTF-8.

## Collections

r[jetstream.wireformat.vec]
Vector/Array Encoding

Vectors (arrays of elements) are encoded as a **u16 count prefix** followed by each element encoded sequentially. The maximum number of elements is 65,535 (u16::MAX).

`byte_size` = 2 + sum(element.byte_size() for each element).

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | 2        | Element count N as u16 (LE) |
| 2      | variable | N elements encoded sequentially |

**Encoding:**
1. If the number of elements exceeds 65,535, encoding MUST fail with an error.
2. Write the element count as a u16 (LE).
3. For each element, call encode on the element.

**Decoding:**
1. Read a u16 count N.
2. For i = 0 to N-1, decode one element.
3. Return the collected elements.

r[jetstream.wireformat.data]
Byte Buffer Encoding

The Data type encodes an arbitrary byte buffer with a **u32 length prefix**. This differs from `Vec<u8>` which uses a u16 prefix. The maximum length is 32 MB (33,554,432 bytes = 32 * 1024 * 1024).

`byte_size` = 4 + len(bytes).

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | 4        | Byte count N as u32 (LE) |
| 4      | N        | Raw bytes |

**Encoding:**
1. Write the byte length as a u32 (LE).
2. Write the raw bytes.

**Decoding:**
1. Read a u32 length N.
2. If N > 33,554,432 (32 MB), decoding MUST fail with an error.
3. Read exactly N bytes.
4. If fewer than N bytes are available, decoding MUST fail with an unexpected EOF error.

r[jetstream.wireformat.map]
Ordered Map Encoding

Ordered maps (BTreeMap) are encoded as a **u16 count prefix** followed by key-value pairs encoded sequentially. Keys MUST be ordered. The maximum number of entries is 65,535.

`byte_size` = 2 + sum(key.byte_size() + value.byte_size() for each entry).

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | 2        | Entry count N as u16 (LE) |
| 2      | variable | N key-value pairs, each key followed by its value |

**Encoding:**
1. Write the entry count as a u16 (LE).
2. For each entry in order, encode the key then the value.

**Decoding:**
1. Read a u16 count N.
2. For i = 0 to N-1, decode one key then one value, and insert into the map.

r[jetstream.wireformat.set]
Ordered Set Encoding

Ordered sets (BTreeSet) are encoded as a **u16 count prefix** followed by elements encoded sequentially. Elements MUST be ordered. The maximum number of elements is 65,535.

`byte_size` = 2 + sum(element.byte_size() for each element).

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | 2        | Element count N as u16 (LE) |
| 2      | variable | N elements encoded sequentially |

## Option Type

r[jetstream.wireformat.option]
Option/Optional Encoding

Optional values are encoded with a **u8 tag byte** followed by the value (if present). Tag `0` = None/absent, tag `1` = Some/present. Any other tag value MUST be rejected as invalid.

`byte_size` = 1 + (value.byte_size() if present, 0 if absent).

**None (absent):**

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 1    | Tag = 0x00 (None) |

**Some (present):**

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | 1        | Tag = 0x01 (Some) |
| 1      | variable | The encoded value |

## Composite Types

r[jetstream.wireformat.struct]
Struct Encoding

Structs are encoded by encoding each field **sequentially in declaration order**. There is no length prefix, no field count, and no field name on the wire. The struct's wire layout is determined entirely by the ordered list of its fields.

`byte_size` = sum(field.byte_size() for each field).

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | variable | Field 1 encoded |
| ...    | variable | Field 2 encoded |
| ...    | ...      | ... |
| ...    | variable | Field N encoded |

**Encoding:**
1. For each field in declaration order, call encode on the field.

**Decoding:**
1. For each field in declaration order, call decode to read the field value.
2. Construct the struct from the decoded field values.

**Special field attributes:**
- `skip`: The field is NOT encoded or decoded. On decode, it receives its type's default value.
- `with(Codec)`: The field uses a custom codec type for byte_size, encode, and decode instead of the standard WireFormat trait.

r[jetstream.wireformat.enum]
Enum Variant Encoding

Enums are encoded with a **u8 variant index** (0-based, assigned in declaration order) followed by the variant's fields encoded sequentially. The maximum number of variants is 256 (u8::MAX + 1).

`byte_size` = 1 + sum(field.byte_size() for each field in the active variant).

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | 1        | Variant index as u8 (0-based) |
| 1      | variable | Variant fields encoded sequentially |

**Encoding:**
1. Write the variant index as a u8.
2. For each field in the variant, call encode on the field.

**Decoding:**
1. Read a u8 variant index.
2. Match the index to the corresponding variant (0 = first variant, 1 = second, etc.).
3. For each field in the matched variant, call decode to read the field value.
4. If the index doesn't match any variant, decoding MUST fail with an "invalid variant index" error.

**Unit variants** (no fields) encode only the 1-byte variant index.

## Network Types

r[jetstream.wireformat.ipv4]
IPv4 Address

Encoded as exactly 4 bytes (the raw octets). `byte_size` = 4.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 4    | IPv4 octets (network byte order, e.g., 192.168.1.1 = [192, 168, 1, 1]) |

r[jetstream.wireformat.ipv6]
IPv6 Address

Encoded as exactly 16 bytes (the raw octets). `byte_size` = 16.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 16   | IPv6 octets (network byte order) |

r[jetstream.wireformat.ipaddr]
IP Address with Type Tag

An IP address is encoded with a **u8 type tag** followed by the address bytes. Tag `4` = IPv4 (4 bytes follow), tag `6` = IPv6 (16 bytes follow). Any other tag value MUST be rejected as invalid.

`byte_size` = 1 + (4 if IPv4, 16 if IPv6).

**IPv4:**

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 1    | Tag = 0x04 (IPv4) |
| 1      | 4    | IPv4 octets |

**IPv6:**

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 1    | Tag = 0x06 (IPv6) |
| 1      | 16   | IPv6 octets |

r[jetstream.wireformat.sockaddr-v4]
Socket Address V4

A SocketAddrV4 is encoded as the IPv4 address (4 bytes) followed by the port as u16 (LE). `byte_size` = 6.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 4    | IPv4 octets |
| 4      | 2    | Port as u16 (LE) |

r[jetstream.wireformat.sockaddr-v6]
Socket Address V6

A SocketAddrV6 is encoded as the IPv6 address (16 bytes) followed by the port as u16 (LE). Flow info and scope ID are NOT encoded (set to 0 on decode). `byte_size` = 18.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 16   | IPv6 octets |
| 16     | 2    | Port as u16 (LE) |

r[jetstream.wireformat.sockaddr]
Socket Address with Type Tag

A SocketAddr is encoded with a **u8 type tag** followed by the typed socket address. Tag `4` = SocketAddrV4, tag `6` = SocketAddrV6. Any other tag value MUST be rejected as invalid.

`byte_size` = 1 + (6 if V4, 18 if V6).

**V4:**

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 1    | Tag = 0x04 |
| 1      | 4    | IPv4 octets |
| 5      | 2    | Port as u16 (LE) |

**V6:**

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 1    | Tag = 0x06 |
| 1      | 16   | IPv6 octets |
| 17     | 2    | Port as u16 (LE) |

## Time

r[jetstream.wireformat.systime]
System Time

SystemTime is encoded as a **u64** representing milliseconds since the Unix epoch (1970-01-01T00:00:00Z) in little-endian order. `byte_size` = 8.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 8    | Milliseconds since Unix epoch as u64 (LE) |

**Encoding:**
1. Calculate the duration since Unix epoch.
2. Convert to milliseconds and encode as u64 (LE).

**Decoding:**
1. Read a u64 value representing milliseconds.
2. Add the duration to the Unix epoch. If the result overflows, decoding MUST fail with a "timestamp overflow" error.

## Error Types

r[jetstream.wireformat.error-inner]
ErrorInner Struct

ErrorInner is a struct encoded per **WF-STRUCT** with the following fields in order:

1. `message: String` — encoded per **WF-STRING**
2. `code: Option<String>` — encoded per **WF-OPTION** containing a **WF-STRING**
3. `help: Option<String>` — encoded per **WF-OPTION** containing a **WF-STRING**
4. `url: Option<String>` — encoded per **WF-OPTION** containing a **WF-STRING**

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | variable | message (WF-STRING) |
| ...    | variable | code (WF-OPTION of WF-STRING) |
| ...    | variable | help (WF-OPTION of WF-STRING) |
| ...    | variable | url (WF-OPTION of WF-STRING) |

r[jetstream.wireformat.error]
Error Type

The top-level Error type encodes as:

1. `ErrorInner` — encoded per **WF-ERR-INNER**
2. `Backtrace` — encoded per **WF-BACKTRACE**

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | variable | ErrorInner (WF-ERR-INNER) |
| ...    | variable | Backtrace (WF-BACKTRACE) |

On decode, the span_trace field is set to None (the error originated remotely).

## Backtrace Types

r[jetstream.wireformat.backtrace]
Backtrace with Intern Table

The Backtrace struct is encoded per **WF-STRUCT** with the following fields in order:

1. `intern_table: Vec<String>` — encoded per **WF-VEC** of **WF-STRING**. This is a deduplicated string table; other fields reference strings by u16 index into this table. Index 0 is reserved for the empty string "".
2. `frames: Vec<Frame>` — encoded per **WF-VEC** of **WF-FRAME**

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | variable | intern_table (WF-VEC of WF-STRING) |
| ...    | variable | frames (WF-VEC of WF-FRAME) |

r[jetstream.wireformat.frame]
Frame Struct

Each Frame is encoded per **WF-STRUCT** with the following fields in order:

1. `msg: String` — the span name, encoded per **WF-STRING**
2. `name: u16` — index into intern_table, encoded per **WF-U16**
3. `target: u16` — index into intern_table, encoded per **WF-U16**
4. `module: u16` — index into intern_table, encoded per **WF-U16**
5. `file: u16` — index into intern_table, encoded per **WF-U16**
6. `line: u16` — source line number, encoded per **WF-U16**
7. `fields: Vec<FieldPair>` — encoded per **WF-VEC** of **WF-FIELDPAIR**
8. `level: Level` — encoded per **WF-LEVEL** (custom codec, NOT standard u8 WireFormat)

| Offset | Size     | Description |
|--------|----------|-------------|
| 0      | variable | msg (WF-STRING) |
| ...    | 2        | name (WF-U16) |
| ...    | 2        | target (WF-U16) |
| ...    | 2        | module (WF-U16) |
| ...    | 2        | file (WF-U16) |
| ...    | 2        | line (WF-U16) |
| ...    | variable | fields (WF-VEC of WF-FIELDPAIR) |
| ...    | 1        | level (WF-LEVEL) |

r[jetstream.wireformat.fieldpair]
Field Pair

A FieldPair encodes a key-value pair referencing the intern table. Encoded per **WF-STRUCT**:

1. `key: u16` — index into the backtrace's intern_table, encoded per **WF-U16**
2. `value: u16` — index into the backtrace's intern_table, encoded per **WF-U16**

`byte_size` = 4.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 2    | key (WF-U16, intern table index) |
| 2      | 2    | value (WF-U16, intern table index) |

r[jetstream.wireformat.level]
Tracing Level Encoding

The tracing Level is encoded as a single **u8** via a custom codec (LevelCodec). The mapping is:

| Value | Level |
|-------|-------|
| 0     | TRACE |
| 1     | DEBUG |
| 2     | INFO  |
| 3     | WARN  |
| 4     | ERROR |

Any other value MUST be rejected as invalid. `byte_size` = 1.

| Offset | Size | Description |
|--------|------|-------------|
| 0      | 1    | Level value (0-4) |

## Box and Wrapper Types

r[jetstream.wireformat.box]
Boxed Types

A `Box<T>` is encoded identically to `T`. The box is transparent on the wire. `byte_size` = inner.byte_size().

r[jetstream.wireformat.url]
URL Type

A URL is encoded as its string representation per **WF-STRING**. On decode, the string is parsed as a URL; if parsing fails, decoding MUST fail with an error. `byte_size` = string_representation.byte_size().
