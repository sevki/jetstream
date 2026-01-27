# JetStream QUIC

[QUIC](https://www.rfc-editor.org/rfc/rfc9000.html) is a modern transport protocol that provides multiplexed connections over UDP with built-in TLS 1.3 encryption. JetStream QUIC provides an HTTP/3 server implementation using [quinn](https://crates.io/crates/quinn) and [h3](https://crates.io/crates/h3).

## Features

- **HTTP/3 Support**: Full HTTP/3 server implementation over QUIC
- **ALPN-based Routing**: Route connections to different handlers based on ALPN protocol negotiation
- **Async Request Handling**: Handle requests concurrently with async/await
- **TLS 1.3**: Built-in secure transport with rustls
- **0-RTT**: Support for zero round-trip connection resumption
- **mTLS Support**: Mutual TLS authentication for client certificate verification
- **Peer Identity**: Extract client certificate information (CN, fingerprint, SANs) in request handlers

## Architecture

The crate is organized around these core components:

- **`Server`**: The main QUIC server that accepts incoming connections
- **`Router`**: Routes connections to protocol handlers based on ALPN
- **`H3Handler`**: HTTP/3 protocol handler
- **`HttpRequestHandler`**: Trait for implementing custom HTTP request handlers
- **`ProtocolHandler`**: Trait for implementing custom protocol handlers

## Example

Here's a complete example of an HTTP/3 server:

```rs
{{#include ../components/jetstream_quic/examples/server.rs}}
```

## Server Setup

To create a QUIC server, you need:

1. TLS certificates (certificate and private key)
2. A router with registered protocol handlers
3. A bind address

```rust
use std::sync::Arc;
use jetstream_quic::{Server, Router, H3Handler, HttpRequestHandler};

// Create your request handler
let h3_handler = Arc::new(H3Handler::new(MyHandler));

// Create and configure the router
let mut router = Router::new();
router.register("h3", h3_handler);

// Create and run the server
let server = Server::new_with_addr(cert, key, "127.0.0.1:4433", router);
server.run().await;
```

## Implementing a Request Handler

Implement the `HttpRequestHandler` trait to handle HTTP requests:

```rust
use async_trait::async_trait;
use bytes::Bytes;
use jetstream_quic::HttpRequestHandler;

struct MyHandler;

#[async_trait]
impl HttpRequestHandler<Bytes, Bytes> for MyHandler {
    async fn handle_request(
        &self,
        req: http::Request<Bytes>,
    ) -> http::Response<Bytes> {
        http::Response::builder()
            .status(200)
            .header("content-type", "text/plain")
            .body(Bytes::from("Hello, QUIC!"))
            .unwrap()
    }
}
```

## Custom Protocol Handlers

For protocols other than HTTP/3, implement the `ProtocolHandler` trait:

```rust
use async_trait::async_trait;
use quinn::Connection;
use jetstream_quic::ProtocolHandler;

struct MyProtocolHandler;

#[async_trait]
impl ProtocolHandler for MyProtocolHandler {
    async fn accept(&self, conn: Connection) {
        // Handle the QUIC connection
    }
}
```

Then register it with a custom ALPN:

```rust
router.register("my-protocol", Arc::new(MyProtocolHandler));
```

## TLS Certificates

For development, you can generate self-signed certificates. The example includes localhost certificates for testing with browsers like Chrome.

For production, use certificates from a trusted CA or Let's Encrypt.

## Mutual TLS (mTLS)

JetStream QUIC supports mutual TLS authentication, where both the server and client present certificates. This is useful for:

- **Client Authentication**: Verify client identity using X.509 certificates
- **Zero-Trust Architecture**: Every connection is authenticated
- **Service-to-Service Communication**: Secure communication between microservices

### Setting up mTLS

Use `Server::new_with_mtls` to create a server that requires client certificates:

```rust
use jetstream_quic::Server;

let server = Server::new_with_mtls(
    server_cert,
    server_key,
    ca_cert,      // CA that signed client certificates
    "0.0.0.0:4433",
    router,
);
server.run().await;
```

### Generating Certificates for mTLS

The example includes a certificate generation script that creates:

1. A Certificate Authority (CA)
2. A server certificate signed by the CA
3. A client certificate signed by the CA

```bash
cd components/jetstream_quic/examples
./generate_localhost_cert.sh
```

This generates:
- `ca.pem` / `ca-key.pem` - Certificate Authority
- `localhost.pem` / `localhost-key.pem` - Server certificate
- `client.pem` / `client-key.pem` - Client certificate

### Accessing Peer Certificate Info

In your request handler, access the peer's certificate information through the `Context`:

```rust
use jetstream_rpc::context::Context;

async fn handle_request(&self, ctx: &Context, req: Request) -> Response {
    if let Some(Peer::Tls(tls_peer)) = ctx.peer() {
        if let Some(leaf) = tls_peer.chain.first() {
            println!("Client CN: {:?}", leaf.common_name);
            println!("Fingerprint: {}", leaf.fingerprint);
            println!("DNS SANs: {:?}", leaf.dns_names);
            println!("IP SANs: {:?}", leaf.ip_addresses);
            println!("URI SANs: {:?}", leaf.uris);
            println!("Email SANs: {:?}", leaf.emails);
        }
    }
    // ...
}
```

### Testing mTLS

Run the example server with mTLS enabled:

```bash
cargo run --example server -- --mtls
```

Test with curl using the client certificate:

```bash
curl --http3 \
  --cacert ca.pem \
  --cert client.pem \
  --key client-key.pem \
  https://localhost:4433/
```

Or use a browser with the client certificate imported (Chrome example included).

## Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
jetstream_quic = "13"
async-trait = "0.1"
bytes = "1"
http = "1"
tokio = { version = "1", features = ["full"] }
```

For more details, see the [jetstream_quic API documentation](doc/jetstream_quic/index.html).
