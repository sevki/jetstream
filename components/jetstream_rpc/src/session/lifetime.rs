//! Session-scoped lane lifetime.
//!
//! r[impl jetstream.session.lifetime]
//! A lane must not outlive its session. Each lane holds the receiving
//! half of a token its session owns; when the session closes it drops
//! the sending halves, and every lane observes that on its next poll —
//! its stream reports the closure and then ends, and its sink refuses
//! further writes, so a call in flight fails rather than hanging.

use std::{
    pin::Pin,
    task::{Context as TaskContext, Poll},
};

use futures::{channel::oneshot, Future, Sink, Stream};

use crate::{
    context::{Context, Contextual},
    session::error::SessionError,
    Error,
};

/// One lane's share of its session's lifetime.
pub(crate) struct LaneLifetime {
    token: Option<oneshot::Receiver<()>>,
    /// Whether the closure has already been reported to the reader, so
    /// that a closed lane reports once and then ends.
    pub(crate) reported: bool,
}

impl LaneLifetime {
    pub(crate) fn new(token: oneshot::Receiver<()>) -> Self {
        Self {
            token: Some(token),
            reported: false,
        }
    }

    /// Whether the session has closed, registering `cx` if it has not.
    pub(crate) fn poll_closed(&mut self, cx: &mut TaskContext<'_>) -> bool {
        let Some(token) = self.token.as_mut() else {
            return true;
        };
        match Pin::new(token).poll(cx) {
            Poll::Ready(_) => {
                self.token = None;
                true
            }
            Poll::Pending => false,
        }
    }
}

/// A lane that terminates when its session closes.
///
/// r[impl jetstream.session.lifetime]
/// Wraps a lane the session handed to a caller. Once the caller owns the
/// lane the session can no longer reach inside it, so the lane observes
/// the session's closure itself rather than the session reaching back.
///
/// The `Sink` and `Stream` impls are generic over what the wrapped lane
/// carries, so one guard serves both directions: the caller's end
/// (requests out, responses in) and the callee's (responses out,
/// requests in).
pub struct LaneGuard<L> {
    lane: L,
    lifetime: LaneLifetime,
}

impl<L> LaneGuard<L> {
    pub(crate) fn new(lane: L, token: oneshot::Receiver<()>) -> Self {
        Self {
            lane,
            lifetime: LaneLifetime::new(token),
        }
    }

    /// The wrapped lane.
    pub fn get_ref(&self) -> &L {
        &self.lane
    }
}

impl<L: std::fmt::Debug> std::fmt::Debug for LaneGuard<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaneGuard")
            .field("lane", &self.lane)
            .finish_non_exhaustive()
    }
}

impl<L, Item> Sink<Item> for LaneGuard<L>
where
    L: Sink<Item, Error = Error> + Unpin,
{
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        if this.lifetime.poll_closed(cx) {
            return Poll::Ready(Err(SessionError::Closed.into()));
        }
        Pin::new(&mut this.lane).poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().lane).start_send(item)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        if this.lifetime.poll_closed(cx) {
            return Poll::Ready(Err(SessionError::Closed.into()));
        }
        Pin::new(&mut this.lane).poll_flush(cx)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().lane).poll_close(cx)
    }
}

impl<L, T> Stream for LaneGuard<L>
where
    L: Stream<Item = Result<T, Error>> + Unpin,
{
    type Item = Result<T, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.lifetime.poll_closed(cx) {
            if this.lifetime.reported {
                return Poll::Ready(None);
            }
            this.lifetime.reported = true;
            return Poll::Ready(Some(Err(SessionError::Closed.into())));
        }
        Pin::new(&mut this.lane).poll_next(cx)
    }
}

impl<L: Contextual> Contextual for LaneGuard<L> {
    fn context(&self) -> Context {
        self.lane.context()
    }
}
