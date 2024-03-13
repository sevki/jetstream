use std::{
    cell::{Cell, RefCell},
    collections::{btree_map, BTreeMap},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use p9::{server::Server, Rframe, Rmessage, Tframe, Tmessage};
use tokio::{sync::oneshot::*, task::JoinHandle};

use crate::{NinePService, Message, JetStreamService};

pub struct Handle {
    tframe: Tframe,
    reply_to: tokio::sync::oneshot::Sender<Rframe>,
}

pub struct Ufs {
    sender: tokio::sync::mpsc::UnboundedSender<Handle>,
    processor: tokio::sync::mpsc::UnboundedReceiver<Handle>,
    server: Server,
}

impl Ufs {
    pub fn new(path: PathBuf) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Handle>();
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
    pub fn get_handler(&self) -> Handler {
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
            let rframe = self
                .server
                .handle(&tframe)
                .await
                .unwrap();
            reply_to.send(rframe).unwrap();
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Handler {
    tx: tokio::sync::mpsc::UnboundedSender<Handle>,
}

impl Message for Tframe {}
impl Message for Rframe {}

impl JetStreamService<Tframe, Rframe> for Handler {
    fn call(
        &mut self,
        req: p9::Tframe,
    ) -> std::pin::Pin<
        Box<
            dyn futures::prelude::Future<
                    Output = Result<
                        p9::Rframe,
                        Box<dyn std::error::Error + Send + Sync>,
                    >,
                > + Send,
        >,
    > {
        let (reply, result) = tokio::sync::oneshot::channel::<Rframe>();
        self.tx
            .send(Handle {
                tframe: req,
                reply_to: reply,
            })
            .unwrap();

        Box::pin(async { result.await.map_err(|e| e.into()) })
    }
}
