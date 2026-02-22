use async_trait::async_trait;
use axum::{routing::IntoMakeService, BoxError, Router};
use bytes::{Buf, Bytes};
use futures::{Stream, StreamExt};
use h3::quic;
use h3_webtransport::server::{AcceptedBi, WebTransportSession};
use http::{Method, Request};
use jetstream_quic::QuicHandler;
use jetstream_rpc::Router as RpcRouter;
use quinn::Connection;
use rustls::pki_types::CertificateDer;
use rustls::server::danger::ClientCertVerifier;
use std::sync::Arc;
use tracing::{error, info};

// r[impl jetstream.webtransport.h3-handler]
// r[impl jetstream.webtransport.registration]
// r[impl jetstream.version.routing.protocol-router]
pub struct H3Service {
    handler: Arc<IntoMakeService<Router>>,
    rpc_router: Arc<RpcRouter>,
    /// Optional client certificate verifier for WebTransport `?cert=` authentication.
    /// When present, certificates from query params are verified against this before
    /// being accepted as `Peer::Tls`. When `None`, cert query param authentication
    /// is disabled and connections will have no peer identity.
    cert_verifier: Option<Arc<dyn ClientCertVerifier>>,
}

impl H3Service {
    pub fn new(router: Router, rpc_router: Arc<jetstream_rpc::Router>) -> Self {
        Self {
            handler: Arc::new(router.into_make_service()),
            rpc_router,
            cert_verifier: None,
        }
    }

    /// Create a new H3Service with a client certificate verifier.
    ///
    /// The verifier is used to validate certificates provided via `?cert=<base64_der>`
    /// query parameters on WebTransport CONNECT requests. This ensures the certificate
    /// chains to a trusted root and passes any custom verification logic (e.g. revocation).
    ///
    /// **Note:** WebTransport does not perform a TLS client handshake, so proof-of-possession
    /// of the private key is NOT verified. The verifier only checks chain trust. Callers
    /// should use short-lived certificates to limit the replay window.
    pub fn new_with_cert_verifier(
        router: Router,
        rpc_router: Arc<jetstream_rpc::Router>,
        cert_verifier: Arc<dyn ClientCertVerifier>,
    ) -> Self {
        Self {
            handler: Arc::new(router.into_make_service()),
            rpc_router,
            cert_verifier: Some(cert_verifier),
        }
    }
}

pub struct RequestStream<S, B>(h3::server::RequestStream<S, B>);

impl<S, B> Stream for RequestStream<S, B>
where
    S: quic::RecvStream,
    B: Buf,
{
    type Item = Result<Bytes, BoxError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match std::pin::Pin::new(&mut this.0).poll_recv_data(cx) {
            std::task::Poll::Ready(Ok(Some(mut data))) => {
                let bytes = data.copy_to_bytes(data.remaining());
                std::task::Poll::Ready(Some(Ok(bytes)))
            }
            std::task::Poll::Ready(Ok(None)) => std::task::Poll::Ready(None),
            std::task::Poll::Ready(Err(err)) => {
                std::task::Poll::Ready(Some(Err(err.into())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl H3Service {
    #[allow(clippy::extra_unused_type_parameters)]
    async fn handle_http_request<S, R>(
        req: http::Request<()>,
        stream: h3::server::RequestStream<S, Bytes>,
        ctx: jetstream_rpc::context::Context,
        http_handler: Arc<IntoMakeService<Router>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        S: h3::quic::BidiStream<Bytes> + Send + 'static,
        R: h3::quic::RecvStream + Send + 'static,
        <S as h3::quic::BidiStream<Bytes>>::RecvStream: Send + 'static,
    {
        info!(?req, "Received request");

        let (mut send_stream, recv_stream) = stream.split();
        let request_stream = RequestStream(recv_stream);
        let body = axum::body::Body::from_stream(request_stream);
        let (parts, _) = req.into_parts();
        let mut req = Request::from_parts(parts, body);
        req.extensions_mut().insert(ctx);
        use tower_service::Service;
        let mut make_svc = http_handler.as_ref().clone();
        let mut svc = make_svc.call(()).await.unwrap();
        let resp = svc.call(req).await.unwrap();

        let (parts, body) = resp.into_parts();
        let resp_header = http::Response::from_parts(parts, ());

        if let Err(err) = send_stream.send_response(resp_header).await {
            error!("unable to send response to connection peer: {:?}", err);
            return Err(err.into());
        }

        let mut body_stream = body.into_data_stream();
        while let Some(chunk) = body_stream.next().await {
            match chunk {
                Ok(bytes) => {
                    if let Err(err) = send_stream.send_data(bytes).await {
                        error!(
                            "unable to send response body to connection peer: {:?}",
                            err
                        );
                        return Err(err.into());
                    }
                }
                Err(err) => {
                    error!(
                        "unable to read response body for connection peer: {:?}",
                        err
                    );
                    return Err(err.into());
                }
            }
        }

        send_stream.finish().await?;
        info!("successfully respond to connection");

        Ok(())
    }
}

/// Extract and verify a TLS peer identity from a `?cert=<base64_der>` query parameter.
///
/// The cert value is URL-decoded (via `url::form_urlencoded`), then base64-decoded
/// to obtain the DER bytes. If a `verifier` is provided, the certificate is validated
/// against it (chain trust, revocation, etc.) before being accepted. If verification
/// fails, returns `None`.
///
/// **Security note:** WebTransport does not perform a TLS client handshake, so
/// proof-of-possession of the private key is NOT verified by this function. Use
/// short-lived certificates to limit the replay window.
///
/// Returns `None` if:
/// - The URI has no `cert` param
/// - Base64 or DER decoding fails
/// - The verifier rejects the certificate
pub fn peer_from_cert_query(
    uri: &http::Uri,
    verifier: Option<&dyn ClientCertVerifier>,
) -> Option<jetstream_rpc::context::Peer> {
    use base64::Engine;
    use rustls::pki_types::UnixTime;

    let query = uri.query()?;
    let b64 = url::form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == "cert")
        .map(|(_, v)| v.into_owned())?;

    let der = base64::engine::general_purpose::STANDARD
        .decode(&b64)
        .map_err(|e| {
            error!("WebTransport CONNECT: failed to base64-decode cert: {}", e);
            e
        })
        .ok()?;

    // If a verifier is configured, validate the certificate chain before trusting it.
    if let Some(verifier) = verifier {
        let cert = CertificateDer::from(der.clone());
        if let Err(e) = verifier.verify_client_cert(&cert, &[], UnixTime::now())
        {
            error!(
                "WebTransport CONNECT: certificate verification failed: {}",
                e
            );
            return None;
        }
    } else {
        // No verifier configured — cert query auth is effectively disabled.
        // Log and return None to avoid accepting unverified certificates.
        info!("WebTransport CONNECT: no cert verifier configured, ignoring cert query param");
        return None;
    }

    let tls_peer = jetstream_rpc::context::TlsPeer::from_der_chain(&[&der])
        .map_err(|e| {
            error!("WebTransport CONNECT: failed to parse cert: {}", e);
            e
        })
        .ok()?;

    Some(jetstream_rpc::context::Peer::Tls(tls_peer))
}

static H3: &[u8] = b"h3";

#[async_trait]
impl QuicHandler for H3Service {
    fn alpns(&self) -> Vec<String> {
        vec![String::from_utf8_lossy(H3).to_string()]
    }
    // r[impl jetstream.webtransport.handler.context]
    async fn accept(
        &self,
        ctx: jetstream_rpc::context::Context,
        conn: Connection,
    ) {
        let remote = conn.remote_address();
        let mut h3_conn = h3::server::builder()
            .enable_webtransport(true)
            .enable_extended_connect(true)
            .enable_datagram(true)
            .max_webtransport_sessions(1024)
            .send_grease(true)
            .build(h3_quinn::Connection::new(conn))
            .await
            .unwrap();

        loop {
            match h3_conn.accept().await {
                Ok(Some(resolver)) => {
                    let (req, stream) = match resolver.resolve_request().await {
                        Ok(resolved) => resolved,
                        Err(err) => {
                            error!("error resolving request: {:?}", err);
                            continue;
                        }
                    };

                    // r[impl jetstream.webtransport.h3-handler.dispatch]
                    // r[impl jetstream.webtransport.session.protocol-version]
                    // r[impl jetstream.version.routing.identifiers]
                    let ext = req.extensions();
                    if req.method() == Method::CONNECT
                        && ext.get::<h3::ext::Protocol>()
                            == Some(&h3::ext::Protocol::WEB_TRANSPORT)
                    {
                        info!("Peer wants to initiate a webtransport session");
                        info!("Handing over connection to WebTransport");

                        // Resolve the handler: prefer stream_router (per-stream
                        // version routing) over legacy path-based routing.

                        // Try to extract peer identity from the CONNECT request.
                        // 1. Cookie header (if browser sends it)
                        // 2. ?cert=<base64_der> query param (client cert for WebTransport auth)
                        let peer = match peer_from_cert_query(
                            req.uri(),
                            self.cert_verifier.as_deref(),
                        ) {
                            Some(peer) => {
                                info!("WebTransport CONNECT: using cert query param for auth");
                                Some(peer)
                            }
                            None => {
                                info!(
                                    "WebTransport CONNECT: no auth credentials"
                                );
                                None
                            }
                        };
                        let remote =
                            Some(jetstream_rpc::context::RemoteAddr::IpAddr(
                                remote.ip(),
                            ));
                        let ctx =
                            jetstream_rpc::context::Context::new(remote, peer);
                        info!("WebTransport context peer: {:?}", ctx.peer());

                        // r[impl jetstream.webtransport.session]
                        // r[impl jetstream.webtransport.lifecycle.h3-fallback]
                        match WebTransportSession::accept(req, stream, h3_conn)
                            .await
                        {
                            Ok(session) => {
                                info!("Established webtransport session");

                                if let Err(e) = handle_session(
                                    self.rpc_router.clone(),
                                    session,
                                    ctx,
                                )
                                .await
                                {
                                    error!(
                                        "webtransport handler failed: {}",
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    "failed to accept webtransport session: {}",
                                    e
                                );
                            }
                        }
                        // WebTransport consumes the connection, exit the loop
                        return;
                    }

                    // Regular HTTP/3 request - spawn a task to handle it
                    let handler = Arc::clone(&self.handler);
                    let ctx = ctx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_http_request::<
                            h3_quinn::BidiStream<Bytes>,
                            h3_quinn::RecvStream,
                        >(
                            req, stream, ctx, handler
                        )
                        .await
                        {
                            error!("handling request failed: {}", e);
                        }
                    });
                }
                // indicating that the remote sent a go-away frame
                // all requests have been processed
                Ok(None) => {
                    break;
                }
                Err(err) => {
                    error!("error on accept {}", err);
                    break;
                }
            }
        }
    }
}

async fn handle_session(
    rpc_router: Arc<RpcRouter>,
    session: WebTransportSession<h3_quinn::Connection, Bytes>,
    ctx: jetstream_rpc::context::Context,
) -> jetstream_error::Result<()> {
    let handler = rpc_router.clone();
    tokio::spawn(async move {
        loop {
            match session.accept_bi().await {
                Ok(Some(AcceptedBi::BidiStream(_, stream))) => {
                    let (send, recv) = quic::BidiStream::split(stream);
                    let handler = handler.clone();
                    let ctx = ctx.clone();

                    tokio::spawn(async move {
                        handler
                            .accept(ctx, Box::new(recv), Box::new(send))
                            .await
                    });
                }
                Ok(Some(_)) => continue,
                Ok(None) => break,
                Err(err) => {
                    eprintln!("Error accepting bidi stream: {}", err);
                    break;
                }
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use rustls::server::WebPkiClientVerifier;
    use rustls::RootCertStore;

    /// Generate a self-signed certificate DER for testing.
    fn test_cert_der() -> (Vec<u8>, String) {
        let mut params = rcgen::CertificateParams::default();
        let mut dn = rcgen::DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, "test@example.com");
        params.distinguished_name = dn;
        params.subject_alt_names = vec![rcgen::SanType::Rfc822Name(
            "test@example.com".try_into().unwrap(),
        )];

        let key_pair = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key_pair).unwrap();
        let der = cert.der().to_vec();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&der);
        (der, b64)
    }

    /// Generate a CA certificate and a client certificate signed by it.
    /// Returns (ca_der, client_der, client_b64).
    fn ca_signed_cert() -> (Vec<u8>, Vec<u8>, String) {
        use rcgen::{
            BasicConstraints, CertificateParams, DistinguishedName, DnType,
            IsCa, KeyPair, KeyUsagePurpose, SanType,
        };

        // Generate CA
        let ca_key = KeyPair::generate().unwrap();
        let mut ca_params = CertificateParams::default();
        let mut ca_dn = DistinguishedName::new();
        ca_dn.push(DnType::CommonName, "Test CA");
        ca_params.distinguished_name = ca_dn;
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages =
            vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let ca_cert = ca_params.self_signed(&ca_key).unwrap();
        let ca_der = ca_cert.der().to_vec();

        // Generate client cert signed by CA
        let client_key = KeyPair::generate().unwrap();
        let mut client_params = CertificateParams::default();
        let mut client_dn = DistinguishedName::new();
        client_dn.push(DnType::CommonName, "test@example.com");
        client_params.distinguished_name = client_dn;
        client_params.subject_alt_names =
            vec![SanType::Rfc822Name("test@example.com".try_into().unwrap())];
        let client_cert = client_params
            .signed_by(&client_key, &ca_cert, &ca_key)
            .unwrap();
        let client_der = client_cert.der().to_vec();
        let client_b64 =
            base64::engine::general_purpose::STANDARD.encode(&client_der);

        (ca_der, client_der, client_b64)
    }

    /// Build a WebPkiClientVerifier that trusts the given CA DER.
    fn verifier_for_ca(ca_der: &[u8]) -> Arc<dyn ClientCertVerifier> {
        // Ensure the ring crypto provider is installed (idempotent).
        let _ = rustls::crypto::ring::default_provider().install_default();

        let mut root_store = RootCertStore::empty();
        root_store
            .add(CertificateDer::from(ca_der.to_vec()))
            .unwrap();
        WebPkiClientVerifier::builder(Arc::new(root_store))
            .allow_unauthenticated()
            .build()
            .unwrap()
    }

    // -- Tests without verifier: all should return None (secure default) --

    #[test]
    fn no_verifier_rejects_valid_cert() {
        let (_der, b64) = test_cert_der();
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("cert", &b64)
            .finish();
        let uri: http::Uri = format!("/pki?{}", query).parse().unwrap();
        assert!(
            peer_from_cert_query(&uri, None).is_none(),
            "should reject cert when no verifier is configured"
        );
    }

    #[test]
    fn no_verifier_none_without_param() {
        let uri: http::Uri = "/pki".parse().unwrap();
        assert!(peer_from_cert_query(&uri, None).is_none());
    }

    #[test]
    fn no_verifier_none_with_other_params() {
        let uri: http::Uri = "/pki?foo=bar&baz=1".parse().unwrap();
        assert!(peer_from_cert_query(&uri, None).is_none());
    }

    // -- Tests with verifier: valid CA-signed certs accepted, others rejected --

    #[test]
    fn verifier_accepts_ca_signed_cert() {
        let (ca_der, _client_der, client_b64) = ca_signed_cert();
        let verifier = verifier_for_ca(&ca_der);
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("cert", &client_b64)
            .finish();
        let uri: http::Uri = format!("/pki?{}", query).parse().unwrap();

        let peer = peer_from_cert_query(&uri, Some(verifier.as_ref()));
        assert!(
            peer.is_some(),
            "should accept a CA-signed cert with matching verifier"
        );
        match peer.unwrap() {
            jetstream_rpc::context::Peer::Tls(tls_peer) => {
                let leaf = tls_peer.leaf().expect("should have leaf cert");
                assert!(
                    leaf.emails.contains(&"test@example.com".to_string()),
                    "leaf cert should contain the email SAN"
                );
            }
            _ => panic!("expected Peer::Tls"),
        }
    }

    #[test]
    fn verifier_rejects_self_signed_cert() {
        let (ca_der, _client_der, _client_b64) = ca_signed_cert();
        let verifier = verifier_for_ca(&ca_der);

        // Use a self-signed cert (not signed by the CA)
        let (_der, b64) = test_cert_der();
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("cert", &b64)
            .finish();
        let uri: http::Uri = format!("/pki?{}", query).parse().unwrap();

        assert!(
            peer_from_cert_query(&uri, Some(verifier.as_ref())).is_none(),
            "should reject a self-signed cert not chaining to the trusted CA"
        );
    }

    #[test]
    fn verifier_rejects_cert_from_different_ca() {
        // Create two separate CAs
        let (ca_der_a, _client_der_a, _) = ca_signed_cert();
        let (_ca_der_b, _client_der_b, client_b64_b) = ca_signed_cert();

        // Verifier trusts CA A, but we present a cert from CA B
        let verifier = verifier_for_ca(&ca_der_a);
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("cert", &client_b64_b)
            .finish();
        let uri: http::Uri = format!("/pki?{}", query).parse().unwrap();

        assert!(
            peer_from_cert_query(&uri, Some(verifier.as_ref())).is_none(),
            "should reject a cert signed by a different CA"
        );
    }

    #[test]
    fn verifier_none_for_invalid_base64() {
        let (ca_der, _, _) = ca_signed_cert();
        let verifier = verifier_for_ca(&ca_der);
        let uri: http::Uri = "/pki?cert=not-valid-base64!!!".parse().unwrap();
        assert!(peer_from_cert_query(&uri, Some(verifier.as_ref())).is_none());
    }

    #[test]
    fn verifier_none_for_invalid_der() {
        let (ca_der, _, _) = ca_signed_cert();
        let verifier = verifier_for_ca(&ca_der);
        let b64 =
            base64::engine::general_purpose::STANDARD.encode(b"not a cert");
        let uri: http::Uri = format!("/pki?cert={}", b64).parse().unwrap();
        assert!(peer_from_cert_query(&uri, Some(verifier.as_ref())).is_none());
    }

    // -- axum-test integration tests using CA-signed certs --

    /// Load the vendor CA cert as DER bytes.
    fn vendor_ca_cert_der() -> Vec<u8> {
        let pem = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../certs/ca.pem"
        ))
        .expect("failed to read vendor ca.pem");
        let certs: Vec<_> = rustls_pemfile::certs(&mut &pem[..])
            .filter_map(|r| r.ok())
            .collect();
        assert!(!certs.is_empty(), "ca.pem should contain at least one cert");
        certs[0].to_vec()
    }

    /// Load the vendor client.pem cert as DER bytes.
    fn vendor_client_cert_der() -> Vec<u8> {
        let pem = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../certs/client.pem"
        ))
        .expect("failed to read vendor client.pem");
        let certs: Vec<_> = rustls_pemfile::certs(&mut &pem[..])
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            !certs.is_empty(),
            "client.pem should contain at least one cert"
        );
        certs[0].to_vec()
    }

    /// Axum handler that extracts the peer from the cert query param
    /// using the vendor CA verifier and returns the email from the leaf certificate.
    async fn extract_cert_handler(
        axum::extract::State(verifier): axum::extract::State<
            Arc<dyn ClientCertVerifier>,
        >,
        uri: http::Uri,
    ) -> axum::response::Response {
        use axum::response::IntoResponse;
        let peer = match peer_from_cert_query(&uri, Some(verifier.as_ref())) {
            Some(p) => p,
            None => {
                return (
                    http::StatusCode::UNAUTHORIZED,
                    "no valid cert in query",
                )
                    .into_response()
            }
        };
        match peer {
            jetstream_rpc::context::Peer::Tls(tls_peer) => {
                let email = tls_peer
                    .leaf()
                    .and_then(|c| c.emails.first().cloned())
                    .unwrap_or_default();
                email.into_response()
            }
            _ => (
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "unexpected peer type",
            )
                .into_response(),
        }
    }

    fn test_router_with_verifier(
        verifier: Arc<dyn ClientCertVerifier>,
    ) -> axum::Router {
        axum::Router::new()
            .route("/pki", axum::routing::get(extract_cert_handler))
            .with_state(verifier)
    }

    #[tokio::test]
    async fn axum_test_vendor_client_cert_accepted() {
        let ca_der = vendor_ca_cert_der();
        let verifier = verifier_for_ca(&ca_der);
        let server =
            axum_test::TestServer::new(test_router_with_verifier(verifier))
                .unwrap();
        let der = vendor_client_cert_der();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&der);

        let response = server.get("/pki").add_query_param("cert", &b64).await;

        response.assert_status_ok();
        response.assert_text("test@example.com");
    }

    #[tokio::test]
    async fn axum_test_self_signed_cert_rejected() {
        let ca_der = vendor_ca_cert_der();
        let verifier = verifier_for_ca(&ca_der);
        let server =
            axum_test::TestServer::new(test_router_with_verifier(verifier))
                .unwrap();
        let (_der, b64) = test_cert_der();

        let response = server.get("/pki").add_query_param("cert", &b64).await;

        response.assert_status(http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn axum_test_no_cert_returns_error() {
        let ca_der = vendor_ca_cert_der();
        let verifier = verifier_for_ca(&ca_der);
        let server =
            axum_test::TestServer::new(test_router_with_verifier(verifier))
                .unwrap();

        let response = server.get("/pki").await;

        response.assert_status(http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn axum_test_invalid_cert_returns_error() {
        let ca_der = vendor_ca_cert_der();
        let verifier = verifier_for_ca(&ca_der);
        let server =
            axum_test::TestServer::new(test_router_with_verifier(verifier))
                .unwrap();

        let response =
            server.get("/pki").add_query_param("cert", "garbage").await;

        response.assert_status(http::StatusCode::UNAUTHORIZED);
    }
}
