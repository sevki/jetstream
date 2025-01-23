#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use echo_protocol::EchoChannel;
use futures::{Sink, Stream};
use jetstream::prelude::*;
use jetstream_macros::service;
use okstd::prelude::*;
use server::service::run;
use tokio_stream::wrappers::UnboundedReceiverStream;
pub mod echo_protocol {
    use jetstream::prelude::*;
    use std::io::{self, Read, Write};
    use std::mem;
    use super::Echo;
    const MESSAGE_ID_START: u8 = 101;
    pub const PROTOCOL_VERSION: &str = "dev.branch.jetstream.proto/echo/7.3.0-077f7c69";
    const DIGEST: &str = "077f7c69306c1f61d5aafb9a12bc1a881a131bb22041f24ef58730179036c9cd";
    pub const TPING: u8 = MESSAGE_ID_START + 0u8;
    pub const RPING: u8 = MESSAGE_ID_START + 0u8 + 1;
    #[allow(non_camel_case_types)]
    pub struct Tping {}
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for Tping {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "Tping")
        }
    }
    mod wire_format_tping {
        extern crate std;
        use self::std::io;
        use self::std::result::Result::Ok;
        use super::Tping;
        use jetstream_wireformat::WireFormat;
        impl WireFormat for Tping {
            fn byte_size(&self) -> u32 {
                0
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                Ok(())
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                Ok(Tping {})
            }
        }
    }
    #[allow(non_camel_case_types)]
    pub struct Rping(pub ());
    #[automatically_derived]
    #[allow(non_camel_case_types)]
    impl ::core::fmt::Debug for Rping {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Rping", &&self.0)
        }
    }
    mod wire_format_rping {
        extern crate std;
        use self::std::io;
        use self::std::result::Result::Ok;
        use super::Rping;
        use jetstream_wireformat::WireFormat;
        impl WireFormat for Rping {
            fn byte_size(&self) -> u32 {
                0 + WireFormat::byte_size(&self.0)
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                WireFormat::encode(&self.0, _writer)?;
                Ok(())
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                let __0 = WireFormat::decode(_reader)?;
                Ok(Rping(__0))
            }
        }
    }
    #[repr(u8)]
    pub enum Tmessage {
        Ping(Tping) = TPING,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Tmessage {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Tmessage::Ping(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Ping",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl Framer for Tmessage {
        fn byte_size(&self) -> u32 {
            match &self {
                Tmessage::Ping(msg) => msg.byte_size(),
            }
        }
        fn message_type(&self) -> u8 {
            unsafe { *<*const _>::from(self).cast::<u8>() }
        }
        fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
            match &self {
                Tmessage::Ping(msg) => msg.encode(writer)?,
            }
            Ok(())
        }
        fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Tmessage> {
            match ty {
                TPING => Ok(Tmessage::Ping(WireFormat::decode(reader)?)),
                _ => {
                    Err(
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            ::alloc::__export::must_use({
                                let res = ::alloc::fmt::format(
                                    format_args!("unknown message type: {0}", ty),
                                );
                                res
                            }),
                        ),
                    )
                }
            }
        }
    }
    #[repr(u8)]
    pub enum Rmessage {
        Ping(Rping) = RPING,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Rmessage {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                Rmessage::Ping(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(
                        f,
                        "Ping",
                        &__self_0,
                    )
                }
            }
        }
    }
    impl Framer for Rmessage {
        fn byte_size(&self) -> u32 {
            match &self {
                Rmessage::Ping(msg) => msg.byte_size(),
            }
        }
        fn message_type(&self) -> u8 {
            unsafe { *<*const _>::from(self).cast::<u8>() }
        }
        fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
            match &self {
                Rmessage::Ping(msg) => msg.encode(writer)?,
            }
            Ok(())
        }
        fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Rmessage> {
            match ty {
                RPING => Ok(Rmessage::Ping(WireFormat::decode(reader)?)),
                _ => {
                    Err(
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            ::alloc::__export::must_use({
                                let res = ::alloc::fmt::format(
                                    format_args!("unknown message type: {0}", ty),
                                );
                                res
                            }),
                        ),
                    )
                }
            }
        }
    }
    pub struct EchoService<T: Echo> {
        pub inner: T,
    }
    #[automatically_derived]
    impl<T: ::core::clone::Clone + Echo> ::core::clone::Clone for EchoService<T> {
        #[inline]
        fn clone(&self) -> EchoService<T> {
            EchoService {
                inner: ::core::clone::Clone::clone(&self.inner),
            }
        }
    }
    impl<T> Protocol for EchoService<T>
    where
        T: Echo + Send + Sync + Sized,
    {
        type Request = Tmessage;
        type Response = Rmessage;
        type Error = Error;
        const VERSION: &'static str = PROTOCOL_VERSION;
        fn rpc(
            &mut self,
            frame: Frame<<Self as Protocol>::Request>,
        ) -> impl ::core::future::Future<
            Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
        > + Send + Sync {
            Box::pin(async move {
                let req: <Self as Protocol>::Request = frame.msg;
                let res: Result<<Self as Protocol>::Response, Self::Error> = match req {
                    Tmessage::Ping(msg) => {
                        let msg = Echo::ping(&mut self.inner).await?;
                        let ret = Rping(msg);
                        Ok(Rmessage::Ping(ret))
                    }
                };
                let rframe: Frame<<Self as Protocol>::Response> = Frame::from((
                    frame.tag,
                    res?,
                ));
                Ok(rframe)
            })
        }
    }
    pub struct EchoChannel<'a> {
        pub inner: Box<&'a mut dyn ClientTransport<Self>>,
    }
    #[automatically_derived]
    impl<'a> ::core::clone::Clone for EchoChannel<'a> {
        #[inline]
        fn clone(&self) -> EchoChannel<'a> {
            EchoChannel {
                inner: ::core::clone::Clone::clone(&self.inner),
            }
        }
    }
    impl<'a> Protocol for EchoChannel<'a> {
        type Request = Tmessage;
        type Response = Rmessage;
        type Error = Error;
        const VERSION: &'static str = PROTOCOL_VERSION;
        fn rpc(
            &mut self,
            frame: Frame<<Self as Protocol>::Request>,
        ) -> impl ::core::future::Future<
            Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
        > + Send + Sync {
            use futures::{SinkExt, StreamExt};
            Box::pin(async move {
                self.inner.send(frame).await?;
                let frame = self.inner.next().await.unwrap()?;
                Ok(frame)
            })
        }
    }
    #[allow(missing_copy_implementations)]
    #[allow(non_camel_case_types)]
    #[allow(dead_code)]
    struct ECHO_TAG {
        __private_field: (),
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals)]
    static ECHO_TAG: ECHO_TAG = ECHO_TAG { __private_field: () };
    impl ::lazy_static::__Deref for ECHO_TAG {
        type Target = std::sync::atomic::AtomicU16;
        fn deref(&self) -> &std::sync::atomic::AtomicU16 {
            #[inline(always)]
            fn __static_ref_initialize() -> std::sync::atomic::AtomicU16 {
                std::sync::atomic::AtomicU16::new(0)
            }
            #[inline(always)]
            fn __stability() -> &'static std::sync::atomic::AtomicU16 {
                static LAZY: ::lazy_static::lazy::Lazy<std::sync::atomic::AtomicU16> = ::lazy_static::lazy::Lazy::INIT;
                LAZY.get(__static_ref_initialize)
            }
            __stability()
        }
    }
    impl ::lazy_static::LazyStatic for ECHO_TAG {
        fn initialize(lazy: &Self) {
            let _ = &**lazy;
        }
    }
    impl Echo for EchoChannel<'a> {
        async fn ping(&mut self) -> Result<(), Error> {
            let tag = ECHO_TAG.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let req = Tmessage::Ping(Tping {});
            let tframe = Frame::from((tag, req));
            let rframe = self.rpc(tframe).await?;
            let rmsg = rframe.msg;
            match rmsg {
                Rmessage::Ping(msg) => Ok(msg.0),
            }
        }
    }
}
pub trait Echo: Send + Sync {
    fn ping(
        &mut self,
    ) -> impl ::core::future::Future<Output = Result<(), Error>> + Send + Sync;
}
struct EchoImpl {}
impl Echo for EchoImpl {
    async fn ping(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
use std::{
    assert_eq, assert_ne, io::{self, ErrorKind},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    rc::Rc, time::Duration,
};
use std::future;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{oneshot, Notify},
    time::timeout,
};
use turmoil::{
    lookup, net::{TcpListener, TcpStream},
    Builder, IpVersion,
};
const PORT: u16 = 1738;
fn assert_error_kind<T>(res: io::Result<T>, kind: io::ErrorKind) {
    match (&res.err().map(|e| e.kind()), &Some(kind)) {
        (left_val, right_val) => {
            if !(*left_val == *right_val) {
                let kind = ::core::panicking::AssertKind::Eq;
                ::core::panicking::assert_failed(
                    kind,
                    &*left_val,
                    &*right_val,
                    ::core::option::Option::None,
                );
            }
        }
    };
}
async fn bind_to_v4(port: u16) -> std::result::Result<TcpListener, std::io::Error> {
    TcpListener::bind((IpAddr::from(Ipv4Addr::UNSPECIFIED), port)).await
}
async fn bind_to_v6(port: u16) -> std::result::Result<TcpListener, std::io::Error> {
    TcpListener::bind((IpAddr::from(Ipv6Addr::UNSPECIFIED), port)).await
}
async fn bind() -> std::result::Result<TcpListener, std::io::Error> {
    bind_to_v4(PORT).await
}
fn network_partitions_during_connect() -> turmoil::Result {
    let mut sim = Builder::new().build();
    sim.host(
        "server",
        || async {
            let listener = bind().await?;
            loop {
                let (stream, _) = listener.accept().await?;
                let echo = EchoImpl {};
                let servercodec: jetstream::prelude::server::service::ServerCodec<
                    echo_protocol::EchoService<EchoImpl>,
                > = Default::default();
                let framed = Framed::new(stream, servercodec);
                let mut serv = echo_protocol::EchoService {
                    inner: echo,
                };
                run(&mut serv, framed).await?;
            }
        },
    );
    sim.client(
        "client",
        async {
            let mut stream = TcpStream::connect(("server", PORT)).await?;
            let client_codec: jetstream_client::ClientCodec<EchoChannel> = Default::default();
            let mut framed = Framed::new(stream, client_codec);
            let chan = EchoChannel {
                inner: Box::new(&mut framed),
            };
            Ok(())
        },
    );
    sim.run()
}
async fn old_main() {
    network_partitions_during_connect().unwrap();
}
#[allow(dead_code)]
fn main() {
    let rt = Runtimes::setup_runtimes().unwrap();
    rt.block_on(old_main())
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[])
}
