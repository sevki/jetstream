# Fixed-size collections (nightly)

`jetstream_wireformat` has an opt-in `nightly` feature that uses
[generic const args](https://blog.rust-lang.org/inside-rust/2026/10/02/generic-const-args-and-you/)
(GCA) to give fixed-size collections a wire format.

> **Experimental.** None of the GCA features are stable, so this API can change
> or break with any nightly. The feature is off by default and stable builds
> are unaffected.

## Enabling it

You need a nightly compiler and the new trait solver, which
`gca_const_items` requires:

```toml
[dependencies]
jetstream_wireformat = { version = "16", features = ["nightly"] }
```

```sh
RUSTFLAGS="-Znext-solver" cargo +nightly build
```

The module is not built for `wasm32`.

## Arrays on the wire

`[T; N]` implements `WireFormat`. Unlike `Vec<T>`, there is **no length
prefix**: the length is part of the type, so the encoding is exactly `N`
consecutive elements.

| Type        | Encoding                        |
|-------------|---------------------------------|
| `Vec<u32>`  | `u16` length, then the elements |
| `[u32; 3]`  | three `u32`s, 12 bytes          |

```rust,ignore
use jetstream_wireformat::WireFormat;

let v: [u32; 3] = [1, 2, 3];
let mut bytes = Vec::new();
v.encode(&mut bytes)?;
assert_eq!(bytes.len(), 12);
assert_eq!(<[u32; 3]>::decode(&mut &bytes[..])?, v);
```

Decoding fails with an I/O error if the input has fewer than `N` elements.

## `FixedWireFormat` and `encode_array`

`FixedWireFormat` marks types whose encoded size is known at compile time via
`WIRE_SIZE`. It is implemented for the fixed-width integers, floats, `bool`,
`()` and arrays of fixed-size types (`WIRE_SIZE = T::WIRE_SIZE * N`).

`EncodeFixed::encode_array` uses it to return a stack array sized by the type,
with no allocation:

```rust,ignore
use jetstream_wireformat::fixed::EncodeFixed;

let a: [u8; 4] = 0x0102_0304u32.encode_array()?;
assert_eq!(a, [4, 3, 2, 1]); // little endian

let b: [u8; 8] = [1u16, 2, 3, 4].encode_array()?;
```

The return type is `[u8; gca!(Self::WIRE_SIZE)]`. GCA doesn't allow inline
arithmetic inside `gca!(..)`, which is why the size lives in an associated
const rather than an expression such as `T::WIRE_SIZE * N`.

## Testing

```sh
RUSTFLAGS="-Znext-solver" cargo +nightly test -p jetstream_wireformat \
  --features nightly --lib fixed
```
