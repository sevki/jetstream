<img src="https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png" align="right" width="200px" />

# JetStream

[![crates.io](https://img.shields.io/crates/v/jetstream.svg)](https://crates.io/crates/jetstream) [![docs.rs](https://docs.rs/jetstream/badge.svg)](https://docs.rs/jetstream) <!--gh actions--> ![Build Status](https://github.com/sevki/jetstream/actions/workflows/rust.yml/badge.svg) [![Release Please🙏!](https://github.com/sevki/jetstream/actions/workflows/release-please.yml/badge.svg)](https://github.com/sevki/jetstream/actions/workflows/release-please.yml) [![benchmark pull requests](https://github.com/sevki/jetstream/actions/workflows/benchmarks.yml/badge.svg)](https://github.com/sevki/jetstream/actions/workflows/benchmarks.yml) [![crates.io downloads](https://img.shields.io/crates/d/jetstream.svg)](https://crates.io/crates/jetstream)

JetStream is an RPC framework built on top of [s2n-quic](https://crates.io/crates/s2n-quic) and [p9](https://crates.io/crates/p9). It's designed to be a high performance, low latency, secure, and reliable RPC framework.

Features:

- [Bidirectional streaming](https://datatracker.ietf.org/meeting/99/materials/slides-99-quic-sessb-quic-unidirectional-and-bidirectional-streams-01)
- [0-RTT](https://blog.cloudflare.com/even-faster-connection-establishment-with-quic-0-rtt-resumption/)
- [mTLS](https://github.com/aws/s2n-quic/tree/main/examples/s2n-mtls)
- [binary encoding](https://docs.rs/jetstream_wireformat/latest/jetstream_wireformat/)

You can see the [crates](crates.html) for an overview of the crates in this repository.
