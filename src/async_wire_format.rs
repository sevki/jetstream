use std::{
    future::Future,
    io::{self},
    pin::Pin,
};

use p9::WireFormat;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub trait AsyncWireFormat: std::marker::Sized {
    fn encode_async<W: AsyncWriteExt + Unpin + Send>(
        self,
        writer: &mut W,
    ) -> impl std::future::Future<Output = io::Result<()>> + Send;
    fn decode_async<R: AsyncReadExt + Unpin + Send>(
        reader: &mut R,
    ) -> impl std::future::Future<Output = io::Result<Self>> + Send;
}

type Task = Pin<Box<dyn std::future::Future<Output = io::Result<()>> + Send>>;

pub trait AsyncWireFormatExt
where
    Self: WireFormat + Send,
{
    fn encode_async<W>(
        self,
        writer: W,
    ) -> impl Future<Output = io::Result<()>> + Send
    where
        Self: Sync,
        W: AsyncWrite + Unpin + Send,
    {
        let mut writer = tokio_util::io::SyncIoBridge::new(writer);
        async { tokio::task::block_in_place(move || self.encode(&mut writer)) }
    }

    fn decode_async<R>(
        reader: R,
    ) -> impl Future<Output = io::Result<Self>> + Send
    where
        Self: Sync,
        R: AsyncRead + Unpin + Send,
    {
        let mut reader = tokio_util::io::SyncIoBridge::new(reader);
        async { tokio::task::block_in_place(move || Self::decode(&mut reader)) }
    }
}

impl<T: WireFormat + Send> AsyncWireFormatExt for T {}

// tests
mod tests {
    use std::{pin::Pin, thread::ThreadId, time::Duration};

    #[allow(unused_imports)]
    use p9::*;
    #[allow(unused_imports)]
    use std::io::Cursor;

    use tokio::time::sleep;

    use super::*;

    struct BlockingIO<T: Sized + Unpin> {
        delay: Duration,
        inner: T,
        read_thread_id: Option<ThreadId>,
    }

    impl BlockingIO<tokio::io::DuplexStream> {
        fn new(delay: Duration, inner: tokio::io::DuplexStream) -> Self {
            Self {
                delay,
                inner: inner,
                read_thread_id: None,
            }
        }
    }

    impl<T> AsyncRead for BlockingIO<T>
    where
        T: AsyncRead + Unpin,
    {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            let delay = self.delay;

            // If there's a delay, we schedule a sleep before proceeding with the read.
            if delay > Duration::ZERO {
                // This future will complete after the specified delay.
                tokio::spawn(async move {
                    sleep(delay).await;
                });
            }
            let this = self.get_mut();
            // Poll the inner AsyncRead.
            Pin::new(&mut this.inner).poll_read(cx, buf)
        }
    }

    impl<T> AsyncWrite for BlockingIO<T>
    where
        T: AsyncWrite + Unpin,
    {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<io::Result<usize>> {
            let delay = self.delay;

            // If there's a delay, we schedule a sleep before proceeding with the write.
            if delay > Duration::ZERO {
                // This future will complete after the specified delay.
                tokio::spawn(async move {
                    sleep(delay).await;
                });
            }
            let this = self.get_mut();
            // Poll the inner AsyncWrite.
            Pin::new(&mut this.inner).poll_write(cx, buf)
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            let this = self.get_mut();
            Pin::new(&mut this.inner).poll_flush(cx)
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            let this = self.get_mut();
            Pin::new(&mut this.inner).poll_shutdown(cx)
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_async_wire_format() {
        let test = Tframe {
            tag: 0,
            msg: Ok(Tmessage::Version(Tversion {
                msize: 8192,
                version: "9P2000.L".to_string(),
            })),
        };

        let mut buf = Vec::new();
        test.encode_async(&mut buf).await.unwrap();
        let mut cursor = Cursor::new(buf);
        let decoded = Tframe::decode_async(&mut cursor).await.unwrap();
        assert_eq!(decoded.tag, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_async_wire_format_delayed() {
        let test = Tframe {
            tag: 0,
            msg: Ok(Tmessage::Version(Tversion {
                msize: 8192,
                version: "9P2000.L".to_string(),
            })),
        };

        let (upstream, downstream) = tokio::io::duplex(1024);
        let writer = BlockingIO::new(Duration::from_millis(1), upstream);
        let reader = BlockingIO::new(Duration::from_millis(1), downstream);

        test.encode_async(writer).await.unwrap();
        let decoded = Tframe::decode_async(reader).await.unwrap();
        assert_eq!(decoded.tag, 0);
    }
}
