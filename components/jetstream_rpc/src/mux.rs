use std::{collections::BTreeMap, pin::Pin, sync::Arc};

use futures::{Sink, Stream, StreamExt};
use jetstream_error::{Error, Result};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::{
    client::ClientTransport, context::Context, subscription::RDONE, Frame,
    Framer, Protocol, RpcCall, RpcFuture, RpcStream, TagPool,
};

pub type RxStream<P> = Pin<
    Box<
        dyn Stream<Item = Result<Frame<<P as Protocol>::Response>>>
            + Send
            + Sync,
    >,
>;

pub type TxSink<P> = Pin<
    Box<dyn Sink<Frame<<P as Protocol>::Request>, Error = Error> + Send + Sync>,
>;

pub type InFlight<P> = Arc<Mutex<BTreeMap<u16, Waiter<P>>>>;

/// What is waiting on a tag.
///
/// r[impl jetstream.subscription.definition]
/// A unary call's tag is freed by its one response. A subscription's is
/// held for as long as the producer may still emit, and freed by the
/// terminator — which is why the two cannot share a representation.
pub enum Waiter<P: Protocol> {
    /// One response, then the tag is free.
    Unary(oneshot::Sender<Result<Frame<P::Response>>>),
    /// Many responses under one tag, until the terminator.
    ///
    /// r[impl jetstream.subscription.identity]
    /// The `binding` is which subscription this slot belongs to, and it
    /// is what makes abandoning one safe: a tag is reused once its
    /// terminator frees it, so a stream dropped after that would
    /// otherwise find a *different* subscription's waiter under its old
    /// number and silence it. Identifiers start at one and are never
    /// reused, so a stale holder simply fails to match.
    Streaming {
        binding: u64,
        tx: mpsc::Sender<Result<Frame<P::Response>>>,
    },
    /// The caller dropped its stream. Frames are discarded, and the tag
    /// stays held until the terminator arrives.
    ///
    /// r[impl jetstream.subscription.identity]
    /// Releasing it at drop instead would let the tag be rebound while
    /// the producer is still emitting under it, and an in-lane item
    /// carries no binding identifier to tell the two apart.
    Abandoned,
}

/// Client Mux
pub struct Mux<P: Protocol> {
    send_queue: tokio::sync::mpsc::Sender<Frame<P::Request>>,
    in_flight: InFlight<P>,
    tag_pool: Arc<TagPool>,
    /// r[impl jetstream.subscription.cancel]
    /// Tags of subscriptions whose caller has gone, awaiting a
    /// cancellation on the wire.
    ///
    /// Unbounded, and that is deliberate. A bounded queue here is filled
    /// by dropping enough unfinished subscriptions without yielding —
    /// trivial on a single-threaded runtime — and the drop path cannot
    /// wait, so the overflow is *discarded*: those subscriptions stay
    /// `Abandoned`, their producers keep working, and their tags are
    /// never released. Teardown has to be lossless, and this is bounded
    /// in practice by the number of live subscriptions anyway.
    cancels: mpsc::UnboundedSender<u16>,
    /// r[impl jetstream.subscription.identity]
    /// Hands out the binding identifiers above. Unique, never reused,
    /// and never zero — the counter starts at one, so `fetch_add`
    /// returning the pre-increment value would hand out zero first.
    bindings: std::sync::atomic::AtomicU64,
}

impl<P: Protocol> Mux<P>
where
    P: 'static,
{
    async fn demux(
        mut rx: RxStream<P>,
        in_flight: InFlight<P>,
        tag_pool: Arc<TagPool>,
        cancels: mpsc::UnboundedSender<u16>,
    ) -> Result<()> {
        use futures::StreamExt;
        // Why the lane stopped, so the drain below can tell every
        // waiter something true rather than just dropping them.
        let reason: Option<Error> = loop {
            let frame = match rx.next().await {
                Some(Ok(frame)) => frame,
                Some(Err(e)) => break Some(e),
                None => break None,
            };
            let frame: Frame<P::Response> = frame;
            let tag = frame.tag;
            // r[impl jetstream.subscription.termination]
            let ends_here = frame.msg.message_type() == RDONE;

            // Take what we need under the lock and release it before any
            // send: a bounded stream channel can block, and holding the
            // map across that would stall every other tag.
            let waiter = {
                let mut map = in_flight.lock().await;
                match map.get(&tag) {
                    None => {
                        // r[impl jetstream.rcp.multiplexing]
                        // A frame for a tag nobody holds means this end
                        // and the peer disagree about what is in flight.
                        // This used to `unwrap` — a panic on any stray
                        // frame — and then, briefly, to log and carry
                        // on. Carrying on is the worse of the two: the
                        // tag stays eligible for reuse, so the *next*
                        // stray frame bearing it is delivered to
                        // whichever call has since been bound to it, and
                        // a detected desynchronisation turns into a
                        // response misbinding that nothing detects.
                        //
                        // The lane is the unit that is out of step, so
                        // the lane is what fails. Everything on it is
                        // resolved by the drain below.
                        tracing::error!(
                            tag,
                            "response for an unknown tag; failing the lane"
                        );
                        break Some(Error::new(format!(
                            "peer sent a response for tag {tag}, which is \
                             not in flight on this lane"
                        )));
                    }
                    Some(Waiter::Unary(_)) => map.remove(&tag),
                    Some(Waiter::Streaming { binding, tx }) => {
                        let (binding, tx) = (*binding, tx.clone());
                        if ends_here {
                            map.remove(&tag);
                        }
                        Some(Waiter::Streaming { binding, tx })
                    }
                    Some(Waiter::Abandoned) => {
                        if ends_here {
                            map.remove(&tag);
                        }
                        Some(Waiter::Abandoned)
                    }
                }
            };

            match waiter {
                Some(Waiter::Unary(tx)) => {
                    if tx.send(Ok(frame)).is_err() {
                        tracing::debug!(tag, "caller gone before its response");
                    }
                    tag_pool.release_tag(tag).await;
                }
                Some(Waiter::Streaming { binding, tx }) => {
                    // r[impl jetstream.subscription.backpressure]
                    // r[impl jetstream.subscription.backpressure.reporting]
                    // Never *await* the subscriber here. This is the
                    // lane's only reader, so a full channel would stop
                    // every other subscription and every unary call on
                    // the lane until this one caught up — the transport
                    // receive window consumed by whichever subscriber
                    // stopped polling. The specification offers exactly
                    // two conforming responses to a subscriber that
                    // cannot keep up, and blocking its neighbours is
                    // neither: apply backpressure, or terminate the
                    // subscription and report it.
                    //
                    // Backpressure belongs at the producer, which is
                    // where the token goes; here the honest move is to
                    // end this one subscription and say why.
                    match tx.try_send(Ok(frame)) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            tracing::debug!(tag, "subscriber gone mid-stream");
                        }
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            tracing::warn!(
                                tag,
                                "subscriber fell behind; terminating it \
                                 rather than stalling the lane"
                            );
                            // Abandoned rather than removed: the tag is
                            // still in flight at the producer, and is
                            // released when its terminator arrives.
                            //
                            // Unless this frame *was* the terminator.
                            // `ends_here` has already taken the waiter
                            // out, and nothing further will arrive under
                            // this tag — so putting it back would strand
                            // the entry and the tag for the life of the
                            // lane, which is what a burst that fills the
                            // queue exactly at `RDONE` would do.
                            if !ends_here {
                                let mut map = in_flight.lock().await;
                                // Only if this subscription still owns
                                // the slot: the caller may have dropped
                                // its stream while the lock was open.
                                if matches!(
                                    map.get(&tag),
                                    Some(Waiter::Streaming { binding: b, .. })
                                        if *b == binding
                                ) {
                                    map.insert(tag, Waiter::Abandoned);
                                }
                            }
                            // Delivering the reason may block, so it
                            // does not happen here. The sender moves to
                            // a task; the lane keeps reading.
                            let overflowed = Error::new(format!(
                                "subscription {tag} fell behind and was \
                                 terminated"
                            ));
                            tokio::spawn(async move {
                                let _ = tx.send(Err(overflowed)).await;
                            });
                            if ends_here {
                                tag_pool.release_tag(tag).await;
                            }
                            // r[impl jetstream.subscription.cancel]
                            // And tell the producer, so the work stops
                            // rather than only the delivery.
                            let _ = cancels.send(tag);
                            continue;
                        }
                    }
                    if ends_here {
                        tag_pool.release_tag(tag).await;
                    }
                }
                Some(Waiter::Abandoned) => {
                    if ends_here {
                        tag_pool.release_tag(tag).await;
                    }
                }
                None => unreachable!("checked under the lock"),
            }
        };

        // r[impl jetstream.subscription.surface.termination]
        // The lane is finished, and every waiter still on it has to be
        // told. A streaming waiter left in the map keeps its receiver
        // open — `RpcStream` holds the same map, so dropping the loop's
        // handle closes nothing — and its caller waits forever for an
        // item that cannot come, holding a tag that can never be
        // released. "The connection dropped" is one of the three
        // outcomes the surface has to distinguish; silence is not one of
        // them.
        let dropped = |tag: u16| {
            Error::new(match &reason {
                Some(e) => {
                    format!("lane failed before tag {tag} finished: {e}")
                }
                None => format!("lane closed before tag {tag} finished"),
            })
        };
        let orphans: Vec<(u16, Waiter<P>)> = {
            let mut map = in_flight.lock().await;
            std::mem::take(&mut *map).into_iter().collect()
        };
        for (tag, waiter) in orphans {
            match waiter {
                Waiter::Unary(tx) => {
                    let _ = tx.send(Err(dropped(tag)));
                }
                // The same rule as the delivery loop above, for the same
                // reason: never *await* a subscriber. These are drained
                // in sequence, so one subscriber whose queue is full
                // would hold up every waiter behind it — none of them
                // told the lane had closed, none of their tags released.
                // The waiter that cannot take its error now gets it from
                // a task instead; the drain moves on.
                Waiter::Streaming { tx, .. } => {
                    let closed = dropped(tag);
                    if let Err(mpsc::error::TrySendError::Full(item)) =
                        tx.try_send(Err(closed))
                    {
                        tokio::spawn(async move {
                            let _ = tx.send(item).await;
                        });
                    }
                }
                Waiter::Abandoned => {}
            }
            tag_pool.release_tag(tag).await;
        }

        match reason {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    async fn mux(
        mut send_queue: tokio::sync::mpsc::Receiver<Frame<P::Request>>,
        mut tx_sink: TxSink<P>,
    ) -> Result<()> {
        while let Some(frame) = send_queue.recv().await {
            use futures::SinkExt;
            tx_sink.send(frame).await?;
        }
        Ok(())
    }

    pub async fn rpc(&self, _ctx: Context, request: P::Request) -> RpcCall<P> {
        let tag = self.tag_pool.acquire_tag().await;
        let (tx, rx) = oneshot::channel();
        self.in_flight.lock().await.insert(tag, Waiter::Unary(tx));

        // r[impl jetstream.subscription.dispatch.issue-order]
        // Queued here, not from a spawned task. A lane is one *ordered*
        // sequence, and handing each request to the scheduler makes the
        // order in which they reach it the scheduler's to decide.
        // Between independent calls that is merely surprising; between a
        // subscription and the cancellation naming it, the cancellation
        // arrives first and names a subscription that does not exist.
        let (tag, future) =
            match self.send_queue.send(Frame { tag, msg: request }).await {
                Ok(()) => (tag, RpcFuture::Waiting(rx)),
                Err(_) => {
                    self.in_flight.lock().await.remove(&tag);
                    self.tag_pool.release_tag(tag).await;
                    (
                        tag,
                        RpcFuture::Failed(Some(Error::new(
                            "the lane is closed",
                        ))),
                    )
                }
            };
        RpcCall { tag, future }
    }

    /// r[impl jetstream.subscription.cancel]
    /// Turns a dropped subscription's tag into a cancellation on the
    /// wire.
    ///
    /// This is a task rather than something `Drop` does itself because
    /// the cancellation needs a *fresh* tag — the specification forbids
    /// reusing the subscription's own — and acquiring one may wait,
    /// which a synchronous `Drop` cannot.
    async fn cancel_dropped(
        mut cancels: mpsc::UnboundedReceiver<u16>,
        in_flight: InFlight<P>,
        tag_pool: Arc<TagPool>,
        send_queue: mpsc::Sender<Frame<P::Request>>,
    ) {
        while let Some(oldtag) = cancels.recv().await {
            // Zero: the cancellation travels the subscription's own lane,
            // where `oldtag` is unambiguous.
            let Some(msg) = P::tcancel(oldtag, 0) else {
                continue;
            };
            // r[impl jetstream.subscription.cancel]
            // Control capacity, not the ordinary pool. Taking an ordinary
            // tag here deadlocks precisely when cancellation matters:
            // with the pool saturated by live subscriptions, this waits
            // for a tag that is only released by the terminator this
            // cancellation would produce.
            let tag = tag_pool.acquire_control_tag().await;
            // A unary waiter whose receiver is dropped: nobody is left to
            // read the acknowledgement, but the tag must stay held until
            // it arrives, and that is what the unary path already does.
            let (tx, _rx) = oneshot::channel();
            in_flight.lock().await.insert(tag, Waiter::Unary(tx));
            if send_queue.send(Frame { tag, msg }).await.is_err() {
                break;
            }
        }
    }

    pub fn new(
        max_concurrent_requests: u16,
        transport: Box<dyn ClientTransport<P>>,
    ) -> Self {
        let tag_pool = Arc::new(TagPool::new(max_concurrent_requests));
        let (send_queue, send_queue_rx) = tokio::sync::mpsc::channel(1024);
        let (tx, rx) = StreamExt::split(transport);
        let (tx, rx) = (Box::pin(tx), Box::pin(rx));
        let in_flight = Arc::new(Mutex::new(BTreeMap::new()));
        let pending = in_flight.clone();
        let tags = tag_pool.clone();
        let (cancels, cancels_rx) = mpsc::unbounded_channel();
        let lagged = cancels.clone();
        tokio::spawn(
            async move { Self::demux(rx, pending, tags, lagged).await },
        );
        tokio::spawn(async move { Self::mux(send_queue_rx, tx).await });

        let map = in_flight.clone();
        let tags = tag_pool.clone();
        let queue = send_queue.clone();
        tokio::spawn(Self::cancel_dropped(cancels_rx, map, tags, queue));

        Self {
            in_flight,
            send_queue,
            tag_pool,
            cancels,
            bindings: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Issue a streaming call: one request, many responses under its tag.
    ///
    /// r[impl jetstream.subscription.overview]
    /// r[impl jetstream.subscription.definition]
    /// The tag stays in flight until the terminator, which is what makes
    /// it the subscription's identity for as long as it lives.
    pub async fn rpc_stream(
        &self,
        _ctx: Context,
        request: P::Request,
        capacity: usize,
    ) -> RpcStream<P> {
        // r[impl jetstream.subscription.cancel]
        // From the streaming region, so no number of live subscriptions
        // can make an ordinary call impossible.
        let tag = self.tag_pool.acquire_streaming_tag().await;
        let binding = self
            .bindings
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (tx, items) = mpsc::channel(capacity);
        self.in_flight
            .lock()
            .await
            .insert(tag, Waiter::Streaming { binding, tx });

        // r[impl jetstream.subscription.dispatch.issue-order]
        // Queued here rather than from a spawned task, for the ordering
        // reason spelled out in `rpc`.
        // r[impl jetstream.subscription.surface.termination]
        // A request that never reached the lane must not look like a
        // subscription that ended normally. Dropping the sender here
        // closes `items`, so the first poll would yield `None` — the
        // same thing a clean, empty subscription yields, and the caller
        // has no way to tell the difference. The unary path resolves
        // this as an error; so does this one.
        let failed = if self
            .send_queue
            .send(Frame { tag, msg: request })
            .await
            .is_err()
        {
            self.in_flight.lock().await.remove(&tag);
            self.tag_pool.release_tag(tag).await;
            Some(Error::new(
                "the lane is closed; the subscription was never issued",
            ))
        } else {
            None
        };
        RpcStream {
            tag,
            binding,
            items,
            in_flight: self.in_flight.clone(),
            cancels: self.cancels.clone(),
            finished: false,
            failed,
        }
    }
}

#[cfg(test)]
mod tests;
