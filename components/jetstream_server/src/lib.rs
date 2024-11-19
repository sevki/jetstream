#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Server
//! ## Feature Flags
//! - `proxy` - Enables the proxy server
//! - `quic` - Enables the QUIC server
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
// Copyright (c) 2024, Sevki <s@sevki.io>
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
#[cfg(feature = "proxy")]
pub mod proxy;
#[cfg(feature = "quic")]
pub mod quic;

use std::fmt::Debug;
use tokio::io::{AsyncRead, AsyncWrite};

#[cfg(feature = "vsock")]
use tokio_vsock::{VsockAddr, VsockListener};

#[async_trait::async_trait]
pub trait ListenerStream: Send + Sync + Debug + 'static {
    type Stream: AsyncRead + AsyncWrite + Unpin + Send + Sync;
    type Addr: std::fmt::Debug;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)>;
}

#[async_trait::async_trait]
impl ListenerStream for tokio::net::UnixListener {
    type Stream = tokio::net::UnixStream;
    type Addr = tokio::net::unix::SocketAddr;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)> {
        tokio::net::UnixListener::accept(self).await
    }
}

#[cfg(feature = "vsock")]
#[async_trait::async_trait]
impl ListenerStream for VsockListener {
    type Stream = tokio_vsock::VsockStream;
    type Addr = VsockAddr;
    async fn accept(&mut self) -> std::io::Result<(Self::Stream, Self::Addr)> {
        VsockListener::accept(self).await
    }
}
