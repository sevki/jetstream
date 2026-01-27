use async_trait::async_trait;
use axum::{routing::IntoMakeService, BoxError, Router};
use bytes::{Buf, Bytes};
use futures::{Stream, StreamExt};
use h3::{quic, server::RequestResolver};
use http::Request;
use jetstream_quic::ProtocolHandler;
use quinn::Connection;
use std::sync::Arc;
use tracing::{error, info};

pub struct H3Service {
    handler: Arc<IntoMakeService<Router>>,
}

impl H3Service {
    pub fn new(router: Router) -> Self {
        Self {
            handler: Arc::new(router.into_make_service()),
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
    async fn handle_request_with_handler<C>(
        ctx: jetstream_rpc::context::Context,
        handler: &Arc<IntoMakeService<Router>>,
        resolver: RequestResolver<C, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        C: h3::quic::Connection<Bytes>,
        <C as h3::quic::OpenStreams<Bytes>>::BidiStream:
            h3::quic::BidiStream<Bytes> + Send,
        <<C as h3::quic::OpenStreams<Bytes>>::BidiStream as h3::quic::BidiStream<
            Bytes,
        >>::RecvStream: Send + 'static,
    {
        let (req, stream) = resolver.resolve_request().await?;
        let (mut send_stream, recv_stream) = stream.split();
        let request_stream = RequestStream(recv_stream);
        let body = axum::body::Body::from_stream(request_stream);
        let (parts, _) = req.into_parts();
        let mut req = Request::from_parts(parts, body);
        req.extensions_mut().insert(ctx);
        use tower_service::Service;
        let mut make_svc = handler.as_ref().clone();
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
    fn alpn(&self) -> String {
        String::from_utf8_lossy(H3).to_string()
    }
    async fn accept(
        &self,
        ctx: jetstream_rpc::context::Context,
        conn: Connection,
    ) {
        let mut h3_conn =
            h3::server::Connection::new(h3_quinn::Connection::new(conn))
                .await
                .unwrap();

        loop {
            match h3_conn.accept().await {
                Ok(Some(resolver)) => {
                    let handler = Arc::clone(&self.handler);
                    let ctx = ctx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_request_with_handler(
                            ctx, &handler, resolver,
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
