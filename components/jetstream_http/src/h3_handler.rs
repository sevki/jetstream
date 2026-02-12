use async_trait::async_trait;
use axum::{routing::IntoMakeService, BoxError, Router};
use bytes::{Buf, Bytes};
use futures::{Stream, StreamExt};
use h3::quic;
use h3_webtransport::server::WebTransportSession;
use http::{Method, Request};
use jetstream_quic::ProtocolHandler;
use quinn::Connection;
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info};

use crate::webtransport_handler::WebTransportHandler;

// r[impl jetstream.webtransport.h3-handler]
// r[impl jetstream.webtransport.registration]
// r[impl jetstream.version.routing.protocol-router]
pub struct H3Service {
    handler: Arc<IntoMakeService<Router>>,
    rpc_handlers: Arc<HashMap<String, Box<dyn WebTransportHandler>>>,
}

impl H3Service {
    pub fn new(router: Router) -> Self {
        Self {
            handler: Arc::new(router.into_make_service()),
            rpc_handlers: Arc::new(HashMap::new()),
        }
    }

    // r[impl jetstream.version.routing]
    /// Register a WebTransport RPC handler for the given protocol name.
    /// The name is used for routing only — version negotiation happens
    /// via Tversion/Rversion on the stream after the connection is established.
    /// Must be called before the H3Service is shared (i.e., before wrapping in Arc).
    pub fn with_handler(
        mut self,
        protocol_name: &str,
        handler: impl WebTransportHandler + 'static,
    ) -> Self {
        let handlers = Arc::get_mut(&mut self.rpc_handlers)
            .expect("register handlers before sharing H3Service");
        handlers.insert(protocol_name.to_string(), Box::new(handler));
        self
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

static H3: &[u8] = b"h3";

#[async_trait]
impl ProtocolHandler for H3Service {
    fn alpns(&self) -> Vec<String> {
        vec![String::from_utf8_lossy(H3).to_string()]
    }
    // r[impl jetstream.webtransport.handler.context]
    async fn accept(
        &self,
        ctx: jetstream_rpc::context::Context,
        conn: Connection,
    ) {
        let mut h3_conn = h3::server::builder()
            .enable_webtransport(true)
            .enable_extended_connect(true)
            .enable_datagram(true)
            .max_webtransport_sessions(1)
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
                        // Extract protocol name from URI path (strip leading '/')
                        let path = req.uri().path();
                        let protocol_name =
                            path.strip_prefix('/').unwrap_or(path);

                        let handler = if let Some(handler) =
                            self.rpc_handlers.get(protocol_name)
                        {
                            handler
                        } else {
                            error!(
                                "no handler registered for protocol: {}",
                                protocol_name
                            );
                            continue;
                        };
                        // r[impl jetstream.webtransport.session]
                        // r[impl jetstream.webtransport.lifecycle.h3-fallback]
                        match WebTransportSession::accept(req, stream, h3_conn)
                            .await
                        {
                            Ok(session) => {
                                info!("Established webtransport session");
                                if let Err(e) =
                                    handler.handle_session(session, ctx).await
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
