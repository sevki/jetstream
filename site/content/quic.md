+++
title = "JetStream QUIC"
description = "QUIC transport with TLS/mTLS support"
+++

# JetStream QUIC

[QUIC](https://www.rfc-editor.org/rfc/rfc9000.html) is a modern transport protocol that provides multiplexed connections over UDP with built-in TLS 1.3 encryption. JetStream QUIC provides the transport layer using [quinn](https://crates.io/crates/quinn).

## Features

- **ALPN-based Routing**: Route connections to different handlers based on ALPN protocol negotiation
- **TLS 1.3**: Built-in secure transport with rustls
- **0-RTT**: Support for zero round-trip connection resumption
- **mTLS Support**: Mutual TLS authentication for client certificate verification
- **Peer Identity**: Extract client certificate information (CN, fingerprint, SANs) in request handlers

## Architecture

The crate is organized around these core components:

- **`Server`**: The main QUIC server that accepts incoming connections
- **`Router`**: Routes connections to protocol handlers based on ALPN
- **`ProtocolHandler`**: Trait for implementing custom protocol handlers
- **`Client`**: QUIC client for connecting to servers

For HTTP/3 support, see [jetstream_http](http.md).

## Example

Here's a complete echo service example:

```rust
{{#include ../examples/echo.rs}}
```

## Defining a Service

Use the `#[service]` macro to define an RPC service:

```rust
use jetstream::prelude::*;
use jetstream_macros::service;

#[service]
pub trait Echo {
    async fn ping(&mut self) -> Result<()>;
    async fn echo(&mut self, message: String) -> Result<String>;
}
```

The macro generates:
- `EchoChannel` - Client-side channel for calling methods
- `EchoService` - Server-side wrapper for your implementation

## Implementing the Service

```rust
#[derive(Clone)]
struct EchoImpl;

impl Echo for EchoImpl {
    async fn ping(&mut self) -> Result<()> {
        Ok(())
    }

    async fn echo(&mut self, message: String) -> Result<String> {
        Ok(message)
    }
}
```

## Server Setup

```rust
use std::sync::Arc;
use jetstream_quic::{Server, Router};

// Create the service
let echo_service = echo_protocol::EchoService { inner: EchoImpl {} };

// Register with router
let mut router = Router::new();
router.register(Arc::new(echo_service));

// Create and run server
let server = Server::new_with_addr(cert, key, addr, router);
server.run().await;
```

## Client Setup

```rust
use jetstream_quic::{Client, QuicTransport};
use jetstream_rpc::Protocol;

// Create client
let alpn = vec![EchoChannel::VERSION.as_bytes().to_vec()];
let client = Client::new_with_mtls(ca_cert, client_cert, client_key, alpn)?;

// Connect
let connection = client.connect(addr, "localhost").await?;

// Open stream and create channel
let (send, recv) = connection.open_bi().await?;
let transport: QuicTransport<EchoChannel> = (send, recv).into();
let mut chan = EchoChannel::new(10, Box::new(transport));

// Call methods
chan.ping().await?;
let response = chan.echo("Hello".to_string()).await?;
```

## TLS Certificates

Generate development certificates:

```bash
cd certs
./generate_certs.sh
```

This generates:
- `ca.pem` / `ca.key` - Certificate Authority
- `server.pem` / `server.key` - Server certificate
- `client.pem` / `client.key` - Client certificate
- `client.p12` - PKCS12 bundle for browser import

For production, use certificates from a trusted CA or Let's Encrypt.

## Mutual TLS (mTLS)

JetStream QUIC supports mutual TLS authentication:

```rust
// Build a client certificate verifier from a CA cert
let mut root_store = rustls::RootCertStore::empty();
root_store.add(ca_cert).expect("Failed to add CA cert");
let client_verifier =
    rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
        .allow_unauthenticated()
        .build()
        .expect("Failed to build client verifier");

let server = Server::new_with_mtls(
    server_cert,
    server_key,
    client_verifier,  // any Arc<dyn ClientCertVerifier>
    addr,
    router,
);
```

### Accessing Peer Certificate Info

```rust
use jetstream_rpc::context::{Context, Peer};

// In your service implementation or handler
if let Some(Peer::Tls(tls_peer)) = ctx.peer() {
    if let Some(leaf) = tls_peer.leaf() {
        println!("Client CN: {:?}", leaf.common_name);
        println!("Fingerprint: {}", leaf.fingerprint);
        println!("DNS SANs: {:?}", leaf.dns_names);
    }
}
```

## Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
jetstream = "15.1"
jetstream_quic = "15.1"
jetstream_macros = "15.1"
tokio = { version = "1", features = ["full"] }
```

For more details, see the [jetstream_quic API documentation](doc/jetstream_quic/index.html).
