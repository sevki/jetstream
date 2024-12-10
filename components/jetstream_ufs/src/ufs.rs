// Copyright (c) 2024, Sevki <s@sevki.io>
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
use std::{collections::btree_map, path::PathBuf};

use jetstream_9p::{Rframe, Tframe};
use jetstream_rpc::{Protocol, Service};

pub use crate::Server;

pub struct Handle<P: Protocol> {
    tframe: P::Request,
    reply_to: tokio::sync::oneshot::Sender<P::Response>,
}

pub struct Ufs {
    sender: tokio::sync::mpsc::UnboundedSender<Handle<UfsProtocol>>,
    processor: tokio::sync::mpsc::UnboundedReceiver<Handle<UfsProtocol>>,
    server: Server,
}

#[derive(Clone)]
pub struct UfsProtocol;

impl Protocol for UfsProtocol {
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
pub struct Handler<P: Protocol> {
    tx: tokio::sync::mpsc::UnboundedSender<Handle<P>>,
}

impl Default for Handler<UfsProtocol> {
    fn default() -> Self {
        let (tx, _) =
            tokio::sync::mpsc::unbounded_channel::<Handle<UfsProtocol>>();
        Self { tx }
    }
}

impl<P: Protocol> Protocol for Handler<P> {
    type Request = P::Request;

    type Response = P::Response;
}

impl<P> Service<P> for Handler<P>
where
    P: Protocol,
{
    async fn rpc(
        self,
        req: P::Request,
    ) -> Result<P::Response, jetstream_rpc::Error> {
        let (reply, result) = tokio::sync::oneshot::channel::<P::Response>();
        self.tx
            .send(Handle {
                tframe: req,
                reply_to: reply,
            })
            .unwrap();

        result
            .await
            .map_err(|e| jetstream_rpc::Error::Anyhow(e.into()))
    }
}
