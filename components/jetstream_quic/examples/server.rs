use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use askama::Template;
use async_trait::async_trait;
use bytes::Bytes;
use http::header;
use jetstream_quic::{H3Handler, HttpRequestHandler, Router};
use jetstream_rpc::context::{Context, Peer, RemoteAddr};

#[derive(Template)]
#[template(path = "template.html")]
struct JetStreamTemplate<'a> {
    body: &'a str,
    version: &'static str,
    year: u16,
}

impl Default for JetStreamTemplate<'_> {
    fn default() -> Self {
        Self {
            body: env!("CARGO_PKG_DESCRIPTION"),
            version: env!("CARGO_PKG_VERSION"),
            year: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| 1970 + (d.as_secs() / 31_536_000) as u16)
                .unwrap_or(2026),
        }
    }
}

#[derive(Template)]
#[template(path = "context.html")]
struct ContextTemplate<'a> {
    remote_addr: &'a str,
    has_peer_cert: bool,
    common_name: Option<&'a str>,
    fingerprint: &'a str,
    dns_names: Vec<&'a str>,
    ip_addresses: Vec<String>,
    emails: Vec<&'a str>,
    uris: Vec<String>,
    method: &'a str,
    path: &'a str,
}

struct TemplateHandler;

#[async_trait]
impl HttpRequestHandler<Bytes, Bytes> for TemplateHandler {
    async fn handle_request(
        &self,
        ctx: Context,
        req: http::Request<Bytes>,
    ) -> http::Response<Bytes> {
        // Log the peer context
        println!("Request from peer: {}", ctx);

        // Get remote address
        let remote_addr = match ctx.remote() {
            Some(RemoteAddr::IpAddr(ip)) => ip.to_string(),
            _ => "unknown".to_string(),
        };

        // Extract peer cert info
        let (
            has_peer_cert,
            common_name,
            fingerprint,
            dns_names,
            ip_addresses,
            emails,
            uris,
        ) = match ctx.peer() {
            Some(Peer::Tls(tls_peer)) => {
                if let Some(leaf) = tls_peer.leaf() {
                    (
                        true,
                        leaf.common_name.clone(),
                        leaf.fingerprint.clone(),
                        leaf.dns_names.clone(),
                        leaf.ip_addresses
                            .iter()
                            .map(|ip| ip.to_string())
                            .collect(),
                        leaf.emails.clone(),
                        leaf.uris.iter().map(|u| u.to_string()).collect(),
                    )
                } else {
                    (false, None, String::new(), vec![], vec![], vec![], vec![])
                }
            }
            _ => (false, None, String::new(), vec![], vec![], vec![], vec![]),
        };

        let method = req.method().as_str();
        let path = req.uri().path();

        let context_html = ContextTemplate {
            remote_addr: &remote_addr,
            has_peer_cert,
            common_name: common_name.as_deref(),
            fingerprint: &fingerprint,
            dns_names: dns_names.iter().map(|s| s.as_str()).collect(),
            ip_addresses,
            emails: emails.iter().map(|s| s.as_str()).collect(),
            uris,
            method,
            path,
        }
        .render()
        .unwrap_or_else(|_| "Error rendering context".to_string());

        let template = JetStreamTemplate {
            body: &context_html,
            ..Default::default()
        };
        let body = template.render().unwrap_or_else(|_| "Error".to_string());

        http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CONTENT_LENGTH, body.len().to_string())
            .body(Bytes::from(body))
            .unwrap()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for --mtls flag
    let mtls_enabled = std::env::args().any(|arg| arg == "--mtls");

    let cert_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/localhost.pem");
    let key_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/localhost.key");
    let ca_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/ca.pem");

    // Load server certificate
    let cert_chain = fs::read(Path::new(cert_path))?;
    let cert = rustls_pemfile::certs(&mut &*cert_chain)
        .collect::<Result<Vec<_>, _>>()?
        .pop()
        .ok_or("No certificate found")?;

    // Load server private key
    let key_data = fs::read(Path::new(key_path))?;
    let key = rustls_pemfile::private_key(&mut &*key_data)?
        .ok_or("No private key found")?;

    // Create the H3 handler with our template handler
    let h3_handler = Arc::new(H3Handler::new(TemplateHandler));

    // Create router and register the H3 handler
    let mut router = Router::new();
    router.register(h3_handler);

    let addr: SocketAddr = "127.0.0.1:4433".parse()?;

    // Create server with or without mTLS
    let server = if mtls_enabled {
        // Load CA certificate for client verification
        let ca_data = fs::read(Path::new(ca_path))?;
        let ca_cert = rustls_pemfile::certs(&mut &*ca_data)
            .collect::<Result<Vec<_>, _>>()?
            .pop()
            .ok_or("No CA certificate found")?;

        println!("QUIC server with mTLS listening on {}", addr);
        println!(
            "Client certificates will be required and verified against ca.pem"
        );
        jetstream_quic::Server::new_with_mtls(cert, key, ca_cert, addr, router)
    } else {
        println!("QUIC server listening on {}", addr);
        println!("No client auth (use --mtls to enable)");
        jetstream_quic::Server::new_with_addr(cert, key, addr, router)
    };

    println!("You can now run ./launch_chrome.sh to connect with Chrome");
    server.run().await;

    Ok(())
}
