use std::future::Future;

use futures::FutureExt;

use crate::Frame;

use crate::Protocol;

pub struct RpcCall<P: Protocol> {
    pub tag: u16,
    pub future: tokio::sync::oneshot::Receiver<
        jetstream_error::Result<Frame<P::Response>>,
    >,
}

impl<P: Protocol> Future for RpcCall<P> {
    type Output = jetstream_error::Result<Frame<P::Response>>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.get_mut().future.poll_unpin(cx) {
            std::task::Poll::Ready(Ok(result)) => {
                std::task::Poll::Ready(result)
            }
            std::task::Poll::Ready(Err(err)) => {
                std::task::Poll::Ready(Err(jetstream_error::Error::with_code(
                    err.to_string(),
                    "jetstream::mux::error",
                )))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
