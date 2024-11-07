<img src="https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png" style="width: 200px">

# JetStream

[![crates.io](https://img.shields.io/crates/v/jetstream.svg)](https://crates.io/crates/jetstream) [![docs.rs](https://docs.rs/jetstream/badge.svg)](https://docs.rs/jetstream) <!--gh actions--> ![Build Status](https://github.com/sevki/jetstream/actions/workflows/rust.yml/badge.svg) ![Build Status](https://github.com/sevki/jetstream/actions/workflows/release.yml/badge.svg) [![crates.io downloads](https://img.shields.io/crates/d/jetstream.svg)](https://crates.io/crates/jetstream)

JetStream is an RPC framework built on top of [s2n-quic](https://crates.io/crates/s2n-quic) and [p9](https://crates.io/crates/p9). It's designed to be a high performance, low latency, secure, and reliable RPC framework.

Features:

- [Bidirectional streaming](https://datatracker.ietf.org/meeting/99/materials/slides-99-quic-sessb-quic-unidirectional-and-bidirectional-streams-01)
- [0-RTT](https://blog.cloudflare.com/even-faster-connection-establishment-with-quic-0-rtt-resumption/)
- [mTLS](https://github.com/aws/s2n-quic/tree/main/examples/s2n-mtls)
- [binary encoding](https://docs.rs/jetstream_wireformat/latest/jetstream_wireformat/)

## Motivation

Building remote filesystems over internet, is the main motivation behind JetStream.

## Ready?

JetStream is not ready for production use. It's still in the early stages of development.

## Alternatives

- [grpc](https://grpc.io/)
- [capnproto](https://capnproto.org/)
- [thrift](https://thrift.apache.org/)
- [jsonrpc](https://www.jsonrpc.org/)
- [tarpc](https://crates.io/crates/tarpc)

## Docs

- [API Documentation](https://docs.rs/jetstream)

## Examples

- [echo](examples/echo.rs)

## [License](LICENSE)

BSD-3-Clause
