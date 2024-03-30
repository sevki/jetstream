use std::{
    error::Error,
    pin::Pin,
};

use crate::coding::{Rframe, Tframe, WireFormat};
use futures::prelude::*;

/// Message trait for JetStream messages, which need to implement the `WireFormat` trait.
pub trait Message: WireFormat + Send + Sync {}

/// A trait for implementing a 9P service.
/// This trait is implemented for types that can handle 9P requests.
pub trait JetStreamService<Req: Message, Resp: Message>:
    Send + Sync + Sized
{
    #[allow(clippy::type_complexity)]
    fn call(
        &mut self,
        req: Req,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Resp, Box<dyn Error + Send + Sync>>>
                + Send,
        >,
    >;
}

/// A trait for implementing a 9P service.
/// This trait is implemented for types that can handle 9P requests.
pub trait NinePService:
    JetStreamService<Tframe, Rframe> + Send + Sync + Clone + Clone
{
}

/// A service that implements the 9P protocol.
#[derive(Debug, Clone, Copy)]
pub struct NinePServiceImpl<S: NinePService> {
    inner: S,
}

impl<S: NinePService> NinePServiceImpl<S> {
    pub fn new(inner: S) -> Self {
        NinePServiceImpl { inner }
    }
}

impl<S: NinePService> JetStreamService<Tframe, Rframe> for NinePServiceImpl<S> {
    fn call(
        &mut self,
        req: Tframe,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Rframe, Box<dyn Error + Send + Sync>>>
                + Send,
        >,
    > {
        self.inner.call(req)
    }
}
