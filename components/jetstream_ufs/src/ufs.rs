use std::error::Error;
use std::{collections::btree_map, path::PathBuf};

use jetstream_9p::{Rframe, Tframe};
use jetstream_rpc::{JetStreamAsyncService, JetStreamProtocol};

pub use crate::Server;

pub struct Handle<Protocol: JetStreamProtocol> {
    tframe: Protocol::Request,
    reply_to: tokio::sync::oneshot::Sender<Protocol::Response>,
}

pub struct Ufs {
    sender: tokio::sync::mpsc::UnboundedSender<Handle<UfsProtocol>>,
    processor: tokio::sync::mpsc::UnboundedReceiver<Handle<UfsProtocol>>,
    server: Server,
}

struct UfsProtocol;

impl JetStreamProtocol for UfsProtocol {
    type Request = Tframe;
    type Response = Rframe;
}

impl Ufs {
    pub fn new(path: PathBuf) -> Self {
        let (tx, rx) =
            tokio::sync::mpsc::unbounded_channel::<Handle<UfsProtocol>>();
        Self {
            sender: tx,
            processor: rx,
            server: Server::new(
                path,
                btree_map::BTreeMap::new(),
                btree_map::BTreeMap::new(),
            )
            .unwrap(),
        }
    }

    pub fn get_handler(&self) -> Handler<UfsProtocol> {
        Handler {
            tx: self.sender.clone(),
        }
    }
}

impl Ufs {
    pub async fn run(&mut self) -> anyhow::Result<()> {
        while let Some(handle) = self.processor.recv().await {
            let tframe = handle.tframe;
            let reply_to = handle.reply_to;
            let rframe = self.server.handle(&tframe).await.unwrap();
            reply_to.send(rframe).unwrap();
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Handler<Protocol: JetStreamProtocol> {
    tx: tokio::sync::mpsc::UnboundedSender<Handle<Protocol>>,
}

impl<Protocol: JetStreamProtocol> JetStreamProtocol for Handler<Protocol> {
    type Request = Protocol::Request;

    type Response = Protocol::Response;
}

impl<Protocol> JetStreamAsyncService for Handler<Protocol>
where
    Protocol: JetStreamProtocol,
{
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn rpc<'life0, 'async_trait>(
        &'life0 mut self,
        req: Protocol::Request,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<
                    Output = Result<
                        Protocol::Response,
                        Box<dyn Error + Send + Sync>,
                    >,
                > + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let (reply, result) =
            tokio::sync::oneshot::channel::<Protocol::Response>();
        self.tx
            .send(Handle {
                tframe: req,
                reply_to: reply,
            })
            .unwrap();

        Box::pin(async { result.await.map_err(|e| e.into()) })
    }
}
