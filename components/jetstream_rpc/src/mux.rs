use std::{collections::BTreeMap, pin::Pin, sync::Arc};

use futures::{Sink, Stream, StreamExt};
use tokio::sync::{oneshot, Mutex};

use jetstream_error::{Error, Result};

use crate::{
    client::ClientTransport, context::Context, Frame, Protocol, RpcCall,
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
    tag_pool: Arc<Mutex<Vec<u16>>>,
}

impl<P: Protocol> Mux<P>
where
    P: 'static,
{
    async fn rx_loop(
        mut rx: RxStream<P>,
        in_flight: InFlight<P>,
        tag_pool: Arc<Mutex<Vec<u16>>>,
    ) -> Result<()> {
        use futures::StreamExt;
        while let Some(Ok(frame)) = rx.next().await {
            let frame: Frame<P::Response> = frame;
            let tag = frame.tag;
            in_flight
                .lock()
                .await
                .remove(&frame.tag)
                .unwrap()
                .send(Ok(frame))
                .map_err(|_| {
                    jetstream_error::Error::new(
                        "Failed to send response".to_string(),
                    )
                })?;
            tag_pool.lock().await.push(tag);
        }

        Ok(())
    }

    async fn tx_loop(
        mut send_queue: tokio::sync::mpsc::Receiver<Frame<P::Request>>,
        mut tx_sink: TxSink<P>,
    ) -> Result<()> {
        while let Some(frame) = send_queue.recv().await {
            use futures::SinkExt;
            tx_sink.send(frame).await?;
        }
        Ok(())
    }

    pub async fn rpc(&self, ctx: Context, request: P::Request) -> RpcCall<P> {
        let (tx, rx) = oneshot::channel();
        let in_flight = self.in_flight.clone();
        let send_queue = self.send_queue.clone();
        let tag_pool = self.tag_pool.clone();
        let tag = tag_pool.lock().await.pop().unwrap();
        tokio::spawn(async move {
            in_flight.lock().await.insert(tag, tx);
            send_queue.send(Frame { tag, msg: request }).await.unwrap();
        });
        RpcCall { tag, future: rx }
    }

    pub fn new(
        tag_pool_size: u16,
        transport: Box<dyn ClientTransport<P>>,
    ) -> Self {
        let tag_pool = Arc::new(Mutex::new((1_u16..tag_pool_size).collect()));
        let (send_queue, send_queue_rx) = tokio::sync::mpsc::channel(1024);
        let (tx, rx) = StreamExt::split(transport);
        let (tx, rx) = (Box::pin(tx), Box::pin(rx));
        let in_flight = Arc::new(Mutex::new(BTreeMap::new()));
        let pending = in_flight.clone();
        let tags = tag_pool.clone();
        tokio::spawn(async move { Self::rx_loop(rx, pending, tags).await });
        tokio::spawn(async move { Self::tx_loop(send_queue_rx, tx).await });
        Self {
            in_flight,
            send_queue,
            tag_pool,
        }
    }
}
