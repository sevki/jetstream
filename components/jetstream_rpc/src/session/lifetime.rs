//! Session-scoped lane lifetime.
//!
//! r[impl jetstream.session.lifetime]
//! A lane must not outlive its session. Each session owns a
//! [`CancellationToken`] and hands every lane a child of it; closing the
//! session cancels the parent, and every lane and every handle derived
//! from one observes that. A lane's stream reports the closure and then
//! ends, and its sink refuses further writes, so a call in flight fails
//! rather than hanging.
//!
//! A cancellation token rather than a one-shot channel per lane, for
//! three reasons: it is cloneable, so handles other than the lane itself
//! — an [`crate::session::local::OrderedSender`], say — can observe the
//! same closure; a child taken from an already-cancelled parent is born
//! cancelled, which closes the race where a lane is handed out
//! concurrently with `close`; and a dropped child deregisters itself, so
//! a session that opens many short-lived lanes does not accumulate one
//! entry per lane it has ever opened.

use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context as TaskContext, Poll},
};

use futures::{Future, Sink, Stream};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use crate::{
    context::{Context, Contextual},
    session::error::SessionError,
    Error,
};

/// One lane's share of its session's lifetime.
pub(crate) struct LaneLifetime {
    token: CancellationToken,
    /// Built on first poll, so that a lane which is never polled costs
    /// nothing.
    waiter: Option<Pin<Box<WaitForCancellationFutureOwned>>>,
    /// Whether the closure has already been reported to the reader, so
    /// that a closed lane reports once and then ends.
    pub(crate) reported: bool,
    live: Arc<AtomicUsize>,
}

impl LaneLifetime {
    pub(crate) fn new(
        token: CancellationToken,
        live: Arc<AtomicUsize>,
    ) -> Self {
        live.fetch_add(1, Ordering::SeqCst);
        Self {
            token,
            waiter: None,
            reported: false,
            live,
        }
    }

    /// Whether the session has closed, without needing a waker.
    ///
    /// r[impl jetstream.session.lifetime]
    /// `start_send` commits a write and has no context to register, so
    /// it needs a synchronous answer.
    pub(crate) fn is_closed(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Whether the session has closed, registering `cx` if it has not.
    pub(crate) fn poll_closed(&mut self, cx: &mut TaskContext<'_>) -> bool {
        if self.token.is_cancelled() {
            return true;
        }
        if self.waiter.is_none() {
            self.waiter = Some(Box::pin(self.token.clone().cancelled_owned()));
        }
        let waiter = self.waiter.as_mut().expect("waiter was just constructed");
        matches!(waiter.as_mut().poll(cx), Poll::Ready(()))
    }
}

impl Drop for LaneLifetime {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::SeqCst);
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
    /// r[impl jetstream.session.identity]
    /// The identity the *session* established, where that is more than
    /// the lane itself knows — a TLS adapter authenticates the peer
    /// during a handshake the framed byte stream underneath never sees.
    /// `None` means the lane's own context is the whole story.
    context: Option<Context>,
}

impl<L> LaneGuard<L> {
    pub(crate) fn new(lane: L, lifetime: LaneLifetime) -> Self {
        Self {
            lane,
            lifetime,
            context: None,
        }
    }

    /// A guard that reports the session's identity rather than the
    /// wrapped lane's.
    pub(crate) fn with_context(
        lane: L,
        lifetime: LaneLifetime,
        context: Option<Context>,
    ) -> Self {
        Self {
            lane,
            lifetime,
            context,
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
        match &self.context {
            Some(context) => context.clone(),
            None => self.lane.context(),
        }
    }
}
