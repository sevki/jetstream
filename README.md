<picture >
  <source media="(max-width:200px),(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream-dark.png">
  <img width="200px" alt="Fallback image description" src="https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png">
</picture>

# JetStream

[![crates.io](https://img.shields.io/crates/v/jetstream.svg)](https://crates.io/crates/jetstream) [![docs.rs](https://docs.rs/jetstream/badge.svg)](https://docs.rs/jetstream) ![Build Status](https://github.com/sevki/jetstream/actions/workflows/rust.yml/badge.svg) [![Release Please🙏!](https://github.com/sevki/jetstream/actions/workflows/release-please.yml/badge.svg)](https://github.com/sevki/jetstream/actions/workflows/release-please.yml) [![benchmark pull requests](https://github.com/sevki/jetstream/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/sevki/jetstream/actions/workflows/benchmarks.yml) [![crates.io downloads](https://img.shields.io/crates/d/jetstream.svg)](https://crates.io/crates/jetstream) [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/sevki/jetstream)

JetStream is an RPC framework built on top of [s2n-quic](https://crates.io/crates/s2n-quic), [iroh](https://crates.io/crates/iroh), and [p9](https://crates.io/crates/p9). It's designed to be a high performance, low latency, secure, and reliable RPC framework.

Features:

- [Bidirectional streaming](https://datatracker.ietf.org/meeting/99/materials/slides-99-quic-sessb-quic-unidirectional-and-bidirectional-streams-01)
- [0-RTT](https://blog.cloudflare.com/even-faster-connection-establishment-with-quic-0-rtt-resumption/)
- [mTLS](https://github.com/aws/s2n-quic/tree/main/examples/s2n-mtls)
- [binary encoding](https://docs.rs/jetstream_wireformat/latest/jetstream_wireformat/)

## Motivation

Building remote filesystems over internet, is the main motivation behind JetStream.

## Ready?

JetStream is not ready for production use. It's still in the early stages of development.

## Docs

- [API Documentation](https://jetstream.rs)
- [Context Guide](https://sevki.github.io/jetstream/context.html) - Learn how to use Context for accessing connection metadata
- [Tracing Guide](https://sevki.github.io/jetstream/tracing.html) - Learn how to instrument your services with distributed tracing

## Examples

- [echo](examples/echo.rs) - Basic QUIC-based echo service example
- [echo_with_tracing](examples/echo_with_tracing.rs) - Echo service with distributed tracing instrumentation
- [iroh_echo](examples/iroh_echo.rs) - Echo service using iroh transport
- [wasm_example](examples/wasm_example.rs) - WebAssembly example
- [wasm_example_bindings](examples/wasm_example_bindings.rs) - WebAssembly bindings example

## [License](./LICENSE)

BSD-3-Clause
