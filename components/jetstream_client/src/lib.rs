// Copyright (c) 2024, Sevki <s@sevki.io>
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
#![doc(html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use {
    jetstream_rpc::{Frame, Protocol},
    jetstream_wireformat::WireFormat,
    tokio_util::{
        bytes::{self, Buf, BufMut},
        codec::{Decoder, Encoder},
    },
};

pub struct ClientCodec<P>
where
    P: Protocol,
{
    _p: std::marker::PhantomData<P>,
}
impl<P: jetstream_rpc::Protocol> Encoder<Frame<P::Request>> for ClientCodec<P> {
    type Error = std::io::Error;

    fn encode(
        &mut self,
        item: Frame<P::Request>,
        dst: &mut bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        WireFormat::encode(&item, &mut dst.writer())
    }
}

impl<P: jetstream_rpc::Protocol> Decoder for ClientCodec<P> {
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(Some(Frame::<P::Response>::decode(&mut src.reader())?))
    }

    type Item = Frame<P::Response>;
}

impl<P> Default for ClientCodec<P>
where
    P: jetstream_rpc::Protocol,
{
    fn default() -> Self {
        Self {
            _p: std::marker::PhantomData,
        }
    }
}
