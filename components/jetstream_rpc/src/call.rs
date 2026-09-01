use std::future::Future;

use futures::FutureExt;

use crate::{Frame, Framer, Protocol};

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
    /// Where a dropped subscription's tag goes to be cancelled.
    ///
    /// A `u16` rather than a built request, because `Drop` is
    /// synchronous: building the cancellation needs nothing async, but
    /// *sending* it needs a tag of its own, and acquiring one may wait.
    /// The `Mux`'s cancellation task does the waiting.
    pub(crate) cancels: tokio::sync::mpsc::UnboundedSender<u16>,
    /// Whether the terminator has already arrived. A subscription that
    /// ended has nothing to cancel, and cancelling it anyway costs a tag
    /// and a round trip on every completed subscription.
    pub(crate) finished: bool,
}

impl<P: Protocol> futures::Stream for RpcStream<P> {
    type Item = jetstream_error::Result<Frame<P::Response>>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let polled = self.items.poll_recv(cx);
        // r[impl jetstream.subscription.termination]
        if let std::task::Poll::Ready(end) = &polled {
            self.finished = match end {
                Some(Ok(frame)) => {
                    frame.msg.message_type() == crate::subscription::RDONE
                }
                // An error item, or the channel closed under us: either
                // way no terminator is coming and there is nothing left
                // to cancel.
                _ => true,
            };
        }
        polled
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

        // r[impl jetstream.subscription.surface.cancellation]
        // r[impl jetstream.subscription.cancel]
        // Marking the tag abandoned only stops *delivery*. The producer
        // is still working, and cancellation is required to reach the
        // work — so the drop a Rust caller uses to cancel has to send the
        // cancellation, not merely stop reading. Without this the rule
        // was unimplemented end to end: the service never heard.
        if self.finished {
            return;
        }
        if self.cancels.send(self.tag).is_err() {
            // The client is shutting down, or more subscriptions were
            // dropped at once than the queue holds. The producer learns
            // when the lane closes, which is the same conclusion by a
            // slower route.
            tracing::debug!(tag = self.tag, "cancellation not queued");
        }
    }
}
