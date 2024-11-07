// Copyright (c) 2024, Sevki <s@sevki.io>
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
use jetstream_client::{self, DialQuic};

use okstd::prelude::*;

use jetstream_wireformat::wire_format_extensions::AsyncWireFormatExt;

use crate::ListenerStream;

pub struct Proxy<L>
where
    L: ListenerStream,
{
    dial: DialQuic,
    listener: L,
}

impl<L> Proxy<L>
where
    L: ListenerStream,
{
    pub fn new(dial: DialQuic, listener: L) -> Self {
        Self { dial, listener }
    }

    pub async fn run<P>(&mut self)
    where
        P: jetstream_rpc::Protocol,
    {
        debug!("Listening on {:?}", self.listener);
        while let std::result::Result::Ok((down_stream, _)) =
            self.listener.accept().await
        {
            debug!("Accepted connection from");
            let down_stream = down_stream;
            let dial = self.dial.clone();
            tokio::spawn(async move {
                debug!("Dialing {:?}", dial);
                let mut dial = dial.clone().dial().await.unwrap();
                debug!("Connected to {:?}", dial.remote_addr());
                let up_stream = dial.open_bidirectional_stream().await.unwrap();
                let (rx, mut tx) = up_stream.split();
                let (read, mut write) = tokio::io::split(down_stream);
                let mut upstream_reader = tokio::io::BufReader::new(rx);
                let mut downstream_reader = tokio::io::BufReader::new(read);
                loop {
                    // read and send to up_stream
                    {
                        debug!("Reading from down_stream");
                        let tframe =
                            P::Request::decode_async(&mut downstream_reader)
                                .await;
                        if let Err(e) = tframe {
                            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                break;
                            } else {
                                error!(
                                    "Error decoding from down_stream: {:?}",
                                    e
                                );
                                break;
                            }
                        } else if let std::io::Result::Ok(tframe) = tframe {
                            tframe.encode_async(&mut tx).await.unwrap();
                        }
                    }
                    // write and send to down_stream
                    {
                        debug!("Reading from up_stream");
                        let rframe =
                            P::Response::decode_async(&mut upstream_reader)
                                .await
                                .unwrap();
                        debug!("Sending to down_stream");
                        rframe.encode_async(&mut write).await.unwrap();
                    }
                }
            });
        }
    }
}
