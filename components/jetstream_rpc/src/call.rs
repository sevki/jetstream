use std::future::Future;

use futures::FutureExt;

use crate::{Frame, Protocol};

pub struct RpcCall<P: Protocol> {
    pub tag: u16,
    pub future: RpcFuture<P>,
}

/// Either a call that reached the lane, or one that could not.
///
/// r[impl jetstream.subscription.dispatch.issue-order]
/// Queuing the request is what fixes issue order, and queuing can fail
/// when the lane is closed. That has to resolve the caller's future as
/// an error rather than panic in a detached task, which is what the
/// spawned send used to do.
pub enum RpcFuture<P: Protocol> {
    Waiting(
        tokio::sync::oneshot::Receiver<
            jetstream_error::Result<Frame<P::Response>>,
        >,
    ),
    Failed(Option<jetstream_error::Error>),
}

impl<P: Protocol> Future for RpcCall<P> {
    type Output = jetstream_error::Result<Frame<P::Response>>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        let rx = match &mut this.future {
            RpcFuture::Waiting(rx) => rx,
            RpcFuture::Failed(err) => {
                let err = err.take().unwrap_or_else(|| {
                    jetstream_error::Error::new("the lane is closed")
                });
                return std::task::Poll::Ready(Err(err));
            }
        };
        match rx.poll_unpin(cx) {
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

/// The caller's end of a subscription: many responses under one tag.
///
/// r[impl jetstream.subscription.surface]
/// A `Stream`, because that is what Rust callers already know how to
/// consume, and `r[jetstream.subscription.surface]` requires the
/// language's idiomatic asynchronous sequence rather than a JetStream
/// shape they must learn.
///
/// r[impl jetstream.subscription.surface.cancellation]
/// Dropping it abandons the subscription. Sending the cancellation frame
/// that tells the *producer* is not done here yet — see the note on
/// `Drop` below — so this is not yet the whole of what `cancel`
/// requires.
pub struct RpcStream<P: Protocol> {
    pub tag: u16,
    pub(crate) items: tokio::sync::mpsc::Receiver<
        jetstream_error::Result<Frame<P::Response>>,
    >,
    pub(crate) in_flight: crate::mux::InFlight<P>,
}

impl<P: Protocol> futures::Stream for RpcStream<P> {
    type Item = jetstream_error::Result<Frame<P::Response>>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.items.poll_recv(cx)
    }
}

impl<P: Protocol> Drop for RpcStream<P> {
    fn drop(&mut self) {
        // r[impl jetstream.subscription.identity]
        // Mark the tag abandoned rather than freeing it. The producer may
        // still be emitting under it, and an in-lane item carries no
        // binding identifier, so a tag rebound now would collect the old
        // subscription's frames. The terminator frees it.
        //
        // `Drop` is synchronous, so this cannot await the map. A
        // `try_lock` that fails leaves the waiter in place, which is the
        // safe direction: frames are delivered to a receiver nobody
        // reads, and the terminator still frees the tag.
        if let Ok(mut map) = self.in_flight.try_lock() {
            if let Some(slot) = map.get_mut(&self.tag) {
                *slot = crate::mux::Waiter::Abandoned;
            }
        }
    }
}
