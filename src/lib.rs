//! <img src="https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png" style="width: 200px">
//!
//! #  JetStream [![crates.io](https://img.shields.io/crates/v/jetstream.svg)](https://crates.io/crates/jetstream) [![docs.rs](https://docs.rs/jetstream/badge.svg)](https://docs.rs/jetstream) <!--gh actions--> ![Build Status](https://github.com/sevki/jetstream/actions/workflows/rust.yml/badge.svg) ![Build Status](https://github.com/sevki/jetstream/actions/workflows/release.yml/badge.svg)
//!
//!
//! JetStream is an RPC framework built on top of [s2n-quic](https://crates.io/crates/s2n-quic) and [p9](https://crates.io/crates/p9). It's designed to be a high performance, low latency, secure, and reliable RPC framework.
//!
//! Features:
//!
//! - Bidirectional streaming
//! - 0-RTT
//! - [mTLS](https://github.com/aws/s2n-quic/tree/main/examples/s2n-mtls)
//! - binary encoding
//!
//! ## Motivation
//!
//! Building remote filesystems over internet, is the main motivation behind JetStream.
//!
//! ## Ready?
//!
//! JetStream is not ready for production use. It's still in the early stages of development.
//!
//! ## Alternatives
//!
//! - [grpc](https://grpc.io/)
//! - [capnproto](https://capnproto.org/)
//! - [thrift](https://thrift.apache.org/)
//! - [jsonrpc](https://www.jsonrpc.org/)
//! - [tarpc](https://crates.io/crates/tarpc)
//!
//! ## [License](LICENSE)
//!
//! BSD-3-Clause

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub use jetstream_p9::protocol;

pub mod async_wire_format;
pub mod log;

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "filesystem")]
pub mod filesystem;
pub mod server;
pub mod service;
