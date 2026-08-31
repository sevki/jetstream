use std::{collections::BTreeMap, pin::Pin, sync::Arc};

use futures::{Sink, Stream, StreamExt};
use jetstream_error::{Error, Result};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::{
    client::ClientTransport, context::Context, subscription::RDONE, Frame,
    Framer, Protocol, RpcCall, RpcStream, TagPool,
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
    Streaming(mpsc::Sender<Result<Frame<P::Response>>>),
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
}

impl<P: Protocol> Mux<P>
where
    P: 'static,
{
    async fn demux(
        mut rx: RxStream<P>,
        in_flight: InFlight<P>,
        tag_pool: Arc<TagPool>,
    ) -> Result<()> {
        use futures::StreamExt;
        while let Some(Ok(frame)) = rx.next().await {
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
                        // A frame for a tag nobody is waiting on. This
                        // used to `unwrap` and take the whole client
                        // down with it, which made any unsolicited or
                        // duplicate frame a panic.
                        tracing::warn!(tag, "response for an unknown tag");
                        continue;
                    }
                    Some(Waiter::Unary(_)) => map.remove(&tag),
                    Some(Waiter::Streaming(tx)) => {
                        let tx = tx.clone();
                        if ends_here {
                            map.remove(&tag);
                        }
                        Some(Waiter::Streaming(tx))
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
                Some(Waiter::Streaming(tx)) => {
                    if tx.send(Ok(frame)).await.is_err() {
                        tracing::debug!(tag, "subscriber gone mid-stream");
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
        }

        Ok(())
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
        let in_flight = self.in_flight.clone();
        let send_queue = self.send_queue.clone();

        tokio::spawn(async move {
            in_flight.lock().await.insert(tag, Waiter::Unary(tx));
            send_queue.send(Frame { tag, msg: request }).await.unwrap();
        });
        RpcCall { tag, future: rx }
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
        tokio::spawn(async move { Self::demux(rx, pending, tags).await });
        tokio::spawn(async move { Self::mux(send_queue_rx, tx).await });
        Self {
            in_flight,
            send_queue,
            tag_pool,
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
        let tag = self.tag_pool.acquire_tag().await;
        let (tx, items) = mpsc::channel(capacity);
        self.in_flight
            .lock()
            .await
            .insert(tag, Waiter::Streaming(tx));
        let send_queue = self.send_queue.clone();
        tokio::spawn(async move {
            let _ = send_queue.send(Frame { tag, msg: request }).await;
        });
        RpcStream {
            tag,
            items,
            in_flight: self.in_flight.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
