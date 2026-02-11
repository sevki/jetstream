+++
title = "JetStream HTTP"
description = "HTTP/2 and HTTP/3 server support"
+++

# JetStream HTTP

JetStream HTTP provides HTTP/2 and HTTP/3 server support with a unified [Axum](https://crates.io/crates/axum) router interface. It enables serving both protocols on the same port using TCP for HTTP/2 and UDP (QUIC) for HTTP/3.

## Features

- **HTTP/2 + HTTP/3**: Serve both protocols simultaneously on the same port
- **Axum Integration**: Use familiar Axum routers and handlers
- **Alt-Svc Header**: Automatically advertise HTTP/3 availability to HTTP/2 clients
- **Context Extractor**: Access connection metadata (peer certificates, remote address) in handlers
- **mTLS Support**: Mutual TLS authentication for both protocols

## Architecture

The crate provides these components:

- **`H3Service`**: HTTP/3 protocol handler for use with `jetstream_quic`
- **`AltSvcLayer`**: Tower layer that adds `Alt-Svc` header to advertise HTTP/3
- **`JetStreamContext`**: Axum extractor for accessing connection context

## Example

Here's a complete example serving HTTP/2 and HTTP/3:

```rust
{{#include ../examples/http.rs}}
```

## Quick Start

### 1. Create an Axum Router

```rust
use axum::{routing::get, Router};
use jetstream_http::{AltSvcLayer, JetStreamContext};

async fn handler(ctx: JetStreamContext) -> &'static str {
    println!("Request from: {:?}", ctx.remote());
    "Hello, World!"
}

let router = Router::new()
    .route("/", get(handler))
    .layer(AltSvcLayer::new(4433));
```

### 2. Set Up HTTP/2 Server (TCP + TLS)

```rust
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as HttpBuilder;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

let listener = TcpListener::bind("127.0.0.1:4433").await?;
let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

loop {
    let (stream, _) = listener.accept().await?;
    let tls_stream = tls_acceptor.accept(stream).await?;
    let io = TokioIo::new(tls_stream);
    let svc = TowerToHyperService::new(router.clone());
    
    HttpBuilder::new(TokioExecutor::new())
        .serve_connection(io, svc)
        .await?;
}
```

### 3. Set Up HTTP/3 Server (QUIC)

```rust
use jetstream_http::H3Service;
use jetstream_quic::{Router as QuicRouter, Server};

let h3_service = Arc::new(H3Service::new(router));

let mut quic_router = QuicRouter::new();
quic_router.register(h3_service);

let server = Server::new_with_addr(cert, key, addr, quic_router);
server.run().await;
```

## Alt-Svc Layer

The `AltSvcLayer` adds the `Alt-Svc` header to HTTP/2 responses, telling browsers that HTTP/3 is available:

```rust
use jetstream_http::AltSvcLayer;

// Advertise HTTP/3 on port 4433 with 24-hour max-age
let layer = AltSvcLayer::new(4433);
// Adds: Alt-Svc: h3=":4433"; ma=86400

// Or with custom value
let layer = AltSvcLayer::with_value("h3=\":443\"; ma=3600, h3-29=\":443\"");
```

When browsers receive this header over HTTP/2, they will attempt to upgrade to HTTP/3 on subsequent requests.

## Context Extractor

Use `JetStreamContext` to access connection metadata in your handlers:

```rust
use jetstream_http::JetStreamContext;
use jetstream_rpc::context::{Peer, RemoteAddr};

async fn handler(ctx: JetStreamContext) -> String {
    // Get remote address
    let addr = match ctx.remote() {
        Some(RemoteAddr::IpAddr(ip)) => ip.to_string(),
        _ => "unknown".to_string(),
    };

    // Get peer certificate info (for mTLS)
    if let Some(Peer::Tls(tls_peer)) = ctx.peer() {
        if let Some(leaf) = tls_peer.leaf() {
            return format!(
                "Hello {}! (CN: {:?})",
                addr,
                leaf.common_name
            );
        }
    }

    format!("Hello {}!", addr)
}
```

## TLS Configuration

### HTTP/2 TLS Config

```rust
let mut tls_config = rustls::ServerConfig::builder()
    .with_no_client_auth()
    .with_single_cert(certs, key)?;

// Important: set ALPN for HTTP/2
tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
```

### HTTP/3 (QUIC) TLS

QUIC/HTTP/3 TLS is handled automatically by `jetstream_quic::Server`.

## Certificates

Generate development certificates:

```bash
cd certs
./generate_certs.sh
```

Files created:
- `server.pem` / `server.key` - Server certificate
- `ca.pem` - CA certificate (for client trust)
- `client.pem` / `client.key` - Client certificate (for mTLS)
- `client.p12` - PKCS12 bundle for browser import
- `server.crt` - DER format for Chrome

## Testing with Chrome

Chrome supports HTTP/3. To test:

```bash
# Start the server
cargo run --example http --features quic

# Launch Chrome with QUIC enabled
./examples/launch_chrome.sh
```

The `launch_chrome.sh` script:
1. Extracts the server's SPKI hash
2. Launches Chrome with `--origin-to-force-quic-on` flag
3. Provides instructions for importing certificates

## Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
jetstream_http = "0.1"
jetstream_quic = "13"
axum = "0.8"
hyper-util = { version = "0.1", features = ["full"] }
tokio-rustls = "0.26"
tower = "0.5"
```

For more details, see the [jetstream_http API documentation](doc/jetstream_http/index.html).
