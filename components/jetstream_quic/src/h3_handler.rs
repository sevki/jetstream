use std::sync::Arc;

use async_trait::async_trait;
use bytes::{Buf, Bytes};
use h3::server::RequestResolver;
use http::Request;
use quinn::Connection;
use tracing::{error, info};

use crate::quic_handler::ProtocolHandler;

#[async_trait]
pub trait HttpRequestHandler<ReqBody, RespBody>: Send + Sync + 'static {
    async fn handle_request(
        &self,
        ctx: jetstream_rpc::context::Context,
        req: http::Request<ReqBody>,
    ) -> http::Response<RespBody>;
}

pub struct H3Handler {
    handler: Arc<dyn HttpRequestHandler<Bytes, Bytes> + Send + Sync>,
}

impl H3Handler {
    pub fn new<H>(handler: H) -> Self
    where
        H: HttpRequestHandler<Bytes, Bytes> + 'static,
    {
        Self {
            handler: Arc::new(handler),
        }
    }
}

impl H3Handler {
    async fn handle_request_with_handler<C>(
        ctx: jetstream_rpc::context::Context,
        handler: &Arc<dyn HttpRequestHandler<Bytes, Bytes> + Send + Sync>,
        resolver: RequestResolver<C, Bytes>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        C: h3::quic::Connection<Bytes>,
    {
        let (req, mut stream) = resolver.resolve_request().await?;
        let request_size = req
            .headers()
            .get(http::header::CONTENT_LENGTH)
            .and_then(|val| val.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let data = stream.recv_data().await?;

        let body = if let Some(mut body) = data {
            body.copy_to_bytes(request_size)
        } else {
            Bytes::new()
        };
        let resp = handler
            .handle_request(ctx, Request::from_parts(req.into_parts().0, body))
            .await;

        let (parts, body) = resp.into_parts();
        let resp_header = http::Response::from_parts(parts, ());

        match stream.send_response(resp_header).await {
            Ok(_) => {
                stream.send_data(body).await?;
                info!("successfully respond to connection");
            }
            Err(err) => {
                error!("unable to send response to connection peer: {:?}", err);
            }
        }

        Ok(stream.finish().await?)
    }
}

static H3: &[u8] = b"h3";

#[async_trait]
impl ProtocolHandler for H3Handler {
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
