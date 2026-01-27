//! Alt-Svc layer for advertising HTTP/3 availability to HTTP/2 clients.
//!
//! This layer adds the `Alt-Svc` header to responses, telling clients that
//! HTTP/3 is available on the same host.

use http::{header::HeaderName, HeaderValue, Request, Response};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower_layer::Layer;
use tower_service::Service;

static ALT_SVC: HeaderName = HeaderName::from_static("alt-svc");

/// Layer that adds `Alt-Svc` header to responses.
#[derive(Clone)]
pub struct AltSvcLayer {
    header_value: HeaderValue,
}

impl AltSvcLayer {
    /// Create a new `AltSvcLayer` that advertises HTTP/3 on the given port.
    ///
    /// # Example
    /// ```ignore
    /// let layer = AltSvcLayer::new(4433);
    /// // Adds header: alt-svc: h3=":4433"; ma=86400
    /// ```
    pub fn new(port: u16) -> Self {
        let value = format!("h3=\":{}\"; ma=86400", port);
        Self {
            header_value: HeaderValue::from_str(&value)
                .expect("valid header value"),
        }
    }

    /// Create a new `AltSvcLayer` with a custom `Alt-Svc` header value.
    ///
    /// # Example
    /// ```ignore
    /// let layer = AltSvcLayer::with_value("h3=\":443\"; ma=3600, h3-29=\":443\"; ma=3600");
    /// ```
    pub fn with_value(value: &str) -> Self {
        Self {
            header_value: HeaderValue::from_str(value)
                .expect("valid header value"),
        }
    }
}

impl<S> Layer<S> for AltSvcLayer {
    type Service = AltSvcService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AltSvcService {
            inner,
            header_value: self.header_value.clone(),
        }
    }
}

/// Service that adds `Alt-Svc` header to responses.
#[derive(Clone)]
pub struct AltSvcService<S> {
    inner: S,
    header_value: HeaderValue,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for AltSvcService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = AltSvcFuture<S::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        AltSvcFuture {
            future: self.inner.call(req),
            header_value: self.header_value.clone(),
        }
    }
}

#[pin_project::pin_project]
pub struct AltSvcFuture<F> {
    #[pin]
    future: F,
    header_value: HeaderValue,
}

impl<F, ResBody, E> Future for AltSvcFuture<F>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
{
    type Output = Result<Response<ResBody>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            Poll::Ready(Ok(mut response)) => {
                response
                    .headers_mut()
                    .insert(ALT_SVC.clone(), this.header_value.clone());
                Poll::Ready(Ok(response))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}
