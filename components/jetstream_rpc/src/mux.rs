use std::{collections::BTreeMap, pin::Pin, sync::Arc};

use futures::{Sink, Stream, StreamExt};
use tokio::sync::{oneshot, Mutex};

use jetstream_error::{Error, Result};

use crate::{
    client::ClientTransport, context::Context, Frame, Protocol, RpcCall,
    TagPool,
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

pub type InFlight<P> = Arc<
    Mutex<
        BTreeMap<
            u16,
            tokio::sync::oneshot::Sender<
                Result<Frame<<P as Protocol>::Response>>,
            >,
        >,
    >,
>;

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
            let result = in_flight
                .lock()
                .await
                .remove(&frame.tag)
                .unwrap()
                .send(Ok(frame));
            match result {
                Ok(_) => {}
                Err(_) => {
                    tracing::error!("couldn't send response frame");
                }
            };
            tag_pool.release_tag(tag).await;
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
            in_flight.lock().await.insert(tag, tx);
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
}
