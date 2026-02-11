+++
title = "JetStream Iroh"
description = "Peer-to-peer transport with built-in NAT traversal"
+++

# JetStream Iroh

[Iroh](https://iroh.computer/) is a peer-to-peer networking library that provides NAT traversal, hole punching, and relay fallback. JetStream integrates with Iroh to enable peer-to-peer RPC communication.

## Features

- **NAT Traversal**: Automatic hole punching for direct peer-to-peer connections
- **Relay Fallback**: Falls back to relay servers when direct connection isn't possible
- **Discovery**: Uses JetStream's discovery service at `discovery.jetstream.rs`
- **ALPN-based Protocol Negotiation**: Each service defines its own protocol version

## Example

Here's a complete example of an echo service using Iroh transport:

```rs
{{#include ../examples/iroh_echo.rs}}
```

## Server Setup

To create an Iroh server, use the `server_builder` function:

```rust
use jetstream_iroh::server_builder;

let router = server_builder(EchoService { inner: EchoServer {} })
    .await
    .unwrap();

// Get the node address to share with clients
let addr = router.endpoint().node_addr();
```

## Client Setup

To connect to an Iroh server, use the `client_builder` function:

```rust
use jetstream_iroh::client_builder;

let mut transport = client_builder::<EchoChannel>(addr)
    .await
    .unwrap();
```

## Discovery

JetStream uses a custom discovery service for Iroh nodes. The discovery URL is:

```
https://discovery.jetstream.rs
```

This allows nodes to find each other using their public keys without needing to exchange IP addresses directly.

## Feature Flag

To use Iroh transport, enable the `iroh` feature in your `Cargo.toml`:

```toml
[dependencies]
jetstream = { version = "10", features = ["iroh"] }
```

For more details, see the [jetstream_iroh API documentation](doc/jetstream_iroh/index.html).
