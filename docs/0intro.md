<picture >
  <source media="(max-width:200px),(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream-dark.png">
  <img width="200px" alt="Fallback image description" src="https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png">
</picture>

# JetStream

[![crates.io](https://img.shields.io/crates/v/jetstream.svg)](https://crates.io/crates/jetstream) [![docs.rs](https://docs.rs/jetstream/badge.svg)](https://docs.rs/jetstream) <!--gh actions--> ![Build Status](https://github.com/sevki/jetstream/actions/workflows/rust.yml/badge.svg) [![Release Please🙏!](https://github.com/sevki/jetstream/actions/workflows/release-please.yml/badge.svg)](https://github.com/sevki/jetstream/actions/workflows/release-please.yml) [![benchmark pull requests](https://github.com/sevki/jetstream/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/sevki/jetstream/actions/workflows/benchmarks.yml) [![crates.io downloads](https://img.shields.io/crates/d/jetstream.svg)](https://crates.io/crates/jetstream)

JetStream is an RPC framework designed to be a high performance, low latency, secure, and reliable RPC framework.

## Transport Backends

JetStream supports multiple transport backends:

- **[quinn](https://crates.io/crates/quinn)** - QUIC transport with TLS/mTLS support
- **[iroh](https://crates.io/crates/iroh)** - P2P transport with built-in NAT traversal
- **[webtransport](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport)** - WebTransport transport for browser and server environments

## Features

- [Bidirectional streaming](https://datatracker.ietf.org/meeting/99/materials/slides-99-quic-sessb-quic-unidirectional-and-bidirectional-streams-01)
- [0-RTT](https://blog.cloudflare.com/even-faster-connection-establishment-with-quic-0-rtt-resumption/)
- [mTLS](https://docs.rs/jetstream_quic/latest/jetstream_quic/struct.Server.html#method.new_with_mtls)
- [Binary encoding](https://docs.rs/jetstream_wireformat/latest/jetstream_wireformat/)
- Cross-platform (Linux, macOS, Windows, WebAssembly)
- Cross-language — [TypeScript](typescript.md) and [Swift](swift.md) clients with wire-compatible codegen

For detailed API documentation, see the [rustdoc documentation](doc/jetstream/index.html).

## Examples

- [echo](https://github.com/sevki/jetstream/blob/main/examples/echo.rs) - Basic QUIC-based echo service example
- [iroh_echo](https://github.com/sevki/jetstream/blob/main/examples/iroh_echo.rs) - Echo service using iroh transport
- [wasm_example](https://github.com/sevki/jetstream/blob/main/examples/wasm_example.rs) - WebAssembly example
- [wasm_example_bindings](https://github.com/sevki/jetstream/blob/main/examples/wasm_example_bindings.rs) - WebAssembly bindings example
