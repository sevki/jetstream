use crate::templates;
use askama::Template;
use axum::{body::Body, response::Response};
use axum::{routing::get, Router};
use http::header::CONTENT_LENGTH;
use jetstream_rpc::{
    context::Context, server::Server, ErrorFrame, Frame, Framer, IntoError,
};
use jetstream_wireformat::WireFormat;
use std::{convert::Infallible, io::Cursor};
use tower_service::Service;

/// Wrap a `Server` implementation into a `tower_service::Service`
#[derive(Clone)]
pub struct ProtocolService<S: Server + Clone>(S);

impl<S: Server + Clone> From<S> for ProtocolService<S> {
    fn from(server: S) -> Self {
        ProtocolService(server)
    }
}

impl<S: Server + Clone> ProtocolService<S> {
    pub fn new(server: S) -> Self {
        ProtocolService(server)
    }
}

impl<S: Server + Clone + Send + 'static> Service<axum::http::Request<Body>>
    for ProtocolService<S>
where
    S::Request: Send + Sync + 'static,
    S::Response: Send + Sync + 'static,
    S::Error: jetstream_error::IntoError + Send + Sync + 'static,
{
    type Response = axum::http::Response<Body>;

    type Error = Infallible;

    type Future = std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Self::Response, Self::Error>,
                > + Send,
        >,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::http::Request<Body>) -> Self::Future {
        let mut service = self.0.clone();
        let (parts, body) = req.into_parts();
        let request_size = parts
            .headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        Box::pin(async move {
            let bytes = match axum::body::to_bytes(body, request_size).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    return Ok(error_to_response(
                        jetstream_error::Error::with_code(
                            err.to_string(),
                            "jetstream_http::E0001",
                        ),
                    ));
                }
            };
            let frame =
                match Frame::<S::Request>::decode(&mut Cursor::new(bytes)) {
                    Ok(frame) => frame,
                    Err(err) => {
                        return Ok(error_to_response(
                            jetstream_error::Error::with_code(
                                err.to_string(),
                                "jetstream_http::E0002",
                            ),
                        ));
                    }
                };
            let resp = service.rpc(Context::default(), frame).await;
            match resp {
                Ok(frame) => Ok(frame_to_response(frame)),
                Err(err) => Ok(error_to_response(err.into_error())),
            }
        })
    }
}

fn error_to_response(err: jetstream_error::Error) -> Response<Body> {
    let error_frame: Frame<ErrorFrame> =
        Frame::from((0u16, ErrorFrame::from(err)));
    frame_to_response(error_frame)
}

fn frame_to_response<F: Framer>(f: Frame<F>) -> Response<Body> {
    let mut buf = vec![];
    let mut writer = Cursor::new(&mut buf);
    f.encode(&mut writer).unwrap();
    let body = Body::from(buf);
    Response::new(body)
}

pub fn new_jetsream_router() -> Router {
    Router::new().fallback(get(|| async {
        templates::JetStreamTemplate::default().render().unwrap()
    }))
}
