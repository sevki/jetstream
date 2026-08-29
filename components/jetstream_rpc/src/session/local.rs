//! Sessions between two peers in one process.
//!
//! r[impl jetstream.session.local]
//! Two peers in one process hold a session without a transport. It
//! reports many lanes, no datagrams and no identity, and its lanes give
//! the same ordering guarantees a transport-backed lane does.
//!
//! r[impl jetstream.session.local.no-serialisation]
//! Frames move between the two ends as values. Nothing is encoded to
//! bytes to obtain ordering: ordering is what the lane provides, and
//! in-process it is provided by the lane itself.

use std::{
    fmt,
    marker::PhantomData,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering as AtomicOrdering},
        Arc, Mutex as StdMutex,
    },
    task::{Context as TaskContext, Poll},
};

use futures::{
    channel::{mpsc, oneshot},
    future::{select, Either},
    pin_mut, Future, Sink, SinkExt, Stream, StreamExt,
};
use tokio::sync::{Mutex, Notify};

use crate::{
    context::{Context, Contextual},
    session::{
        capabilities::Capabilities, error::SessionError, order::LaneOrder,
        OrderTicket, Session,
    },
    Error, Frame, Protocol,
};

/// Frames a lane will hold before the writer is told to wait.
///
/// r[impl jetstream.lane.backpressure]
/// The lane's `Sink` reports pending once this many frames are
/// undelivered, rather than buffering without bound.
pub const DEFAULT_LANE_CAPACITY: usize = 64;

/// What one peer hands the other when it opens a lane: the callee's two
/// halves of the new lane.
type LaneOffer<P> = (
    mpsc::Sender<Frame<<P as Protocol>::Response>>,
    mpsc::Receiver<Frame<<P as Protocol>::Request>>,
);

struct SessionInner<P: Protocol> {
    /// Lanes we open are offered to the peer down this channel.
    peer_offers: mpsc::UnboundedSender<LaneOffer<P>>,
    /// Lanes the peer opened, waiting to be accepted.
    offers: Mutex<mpsc::UnboundedReceiver<LaneOffer<P>>>,
    /// One token per live lane. Dropping a token terminates its lane.
    lanes: StdMutex<Vec<oneshot::Sender<()>>>,
    /// Wakes anything parked in `accept_lane` when the session closes.
    closing: Notify,
    closed: AtomicBool,
    capacity: usize,
}

/// One end of an in-process session.
///
/// Cheap to clone; clones address the same session, which is what makes
/// a session usable from several tasks at once.
pub struct LocalSession<P: Protocol> {
    inner: Arc<SessionInner<P>>,
}

impl<P: Protocol> Clone for LocalSession<P> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// The two ends of an in-process session.
///
/// r[impl jetstream.session.symmetric]
/// The names are conventional. Either end may open a lane, and the end
/// that opens it is the caller on it.
pub struct LocalSessionPair<P: Protocol> {
    /// The end that opens lanes in the usual arrangement.
    pub client: LocalSession<P>,
    /// The end that accepts them.
    pub server: LocalSession<P>,
}

impl<P: Protocol> LocalSession<P> {
    /// A connected pair of in-process sessions.
    pub fn pair() -> LocalSessionPair<P> {
        Self::pair_with_capacity(DEFAULT_LANE_CAPACITY)
    }

    /// A connected pair whose lanes hold `capacity` frames before
    /// applying backpressure.
    pub fn pair_with_capacity(capacity: usize) -> LocalSessionPair<P> {
        let (client_offers, from_client) = mpsc::unbounded();
        let (server_offers, from_server) = mpsc::unbounded();
        LocalSessionPair {
            client: LocalSession {
                inner: Arc::new(SessionInner::new(
                    client_offers,
                    from_server,
                    capacity,
                )),
            },
            server: LocalSession {
                inner: Arc::new(SessionInner::new(
                    server_offers,
                    from_client,
                    capacity,
                )),
            },
        }
    }

    fn is_closed(&self) -> bool {
        self.inner.closed.load(AtomicOrdering::SeqCst)
    }

    /// r[impl jetstream.session.lifetime]
    /// Every lane holds a token owned by its session, so a lane cannot
    /// outlive the session that opened it.
    fn register_lane(&self) -> Result<LaneLifetime, SessionError> {
        let (tx, rx) = oneshot::channel();
        let mut lanes =
            self.inner.lanes.lock().expect("session lanes poisoned");
        if self.is_closed() {
            return Err(SessionError::Closed);
        }
        lanes.push(tx);
        Ok(LaneLifetime::new(rx))
    }
}

impl<P: Protocol> SessionInner<P> {
    fn new(
        peer_offers: mpsc::UnboundedSender<LaneOffer<P>>,
        offers: mpsc::UnboundedReceiver<LaneOffer<P>>,
        capacity: usize,
    ) -> Self {
        Self {
            peer_offers,
            offers: Mutex::new(offers),
            lanes: StdMutex::new(Vec::new()),
            closing: Notify::new(),
            closed: AtomicBool::new(false),
            capacity,
        }
    }
}

impl<P: Protocol> fmt::Debug for LocalSession<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalSession")
            .field("closed", &self.is_closed())
            .field("capacity", &self.inner.capacity)
            .finish_non_exhaustive()
    }
}

impl<P: Protocol> fmt::Debug for LocalClientLane<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalClientLane").finish_non_exhaustive()
    }
}

impl<P: Protocol> fmt::Debug for LocalServiceLane<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalServiceLane").finish_non_exhaustive()
    }
}

impl<P: Protocol> fmt::Debug for OrderedSender<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderedSender")
            .field("turn", &self.order.turn())
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl<P> Session<P> for LocalSession<P>
where
    P: Protocol<Error = Error> + 'static,
{
    type ClientLane = LocalClientLane<P>;
    type ServiceLane = LocalServiceLane<P>;

    /// r[impl jetstream.session.local]
    fn capabilities(&self) -> Capabilities {
        Capabilities::in_process()
    }

    /// r[impl jetstream.lane.independence]
    /// Each lane is its own pair of channels, so a stalled lane holds up
    /// nothing but itself.
    async fn open_lane(&self) -> Result<Self::ClientLane, SessionError> {
        if self.is_closed() {
            return Err(SessionError::Closed);
        }
        let capacity = self.inner.capacity;
        let (requests_tx, requests_rx) = mpsc::channel(capacity);
        let (responses_tx, responses_rx) = mpsc::channel(capacity);
        let lifetime = self.register_lane()?;

        self.inner
            .peer_offers
            .unbounded_send((responses_tx, requests_rx))
            .map_err(|_| SessionError::Closed)?;

        Ok(LocalClientLane {
            tx: requests_tx,
            rx: responses_rx,
            order: LaneOrder::new(),
            lifetime,
            _p: PhantomData,
        })
    }

    async fn accept_lane(&self) -> Result<Self::ServiceLane, SessionError> {
        if self.is_closed() {
            return Err(SessionError::Closed);
        }
        let offer = {
            let mut offers = self.inner.offers.lock().await;
            let next = offers.next();
            let closing = self.inner.closing.notified();
            pin_mut!(next);
            pin_mut!(closing);
            match select(next, closing).await {
                Either::Left((offer, _)) => offer,
                // r[impl jetstream.session.lifetime]
                Either::Right(_) => return Err(SessionError::Closed),
            }
        };
        let (tx, rx) = offer.ok_or(SessionError::Closed)?;
        let lifetime = self.register_lane()?;
        Ok(LocalServiceLane {
            tx,
            rx,
            lifetime,
            _p: PhantomData,
        })
    }

    /// r[impl jetstream.session.lifetime]
    /// Dropping the lane tokens terminates every lane this session
    /// opened or accepted: their streams end and their sinks fail, so a
    /// call in flight fails rather than hanging.
    async fn close(&self) {
        self.inner.closed.store(true, AtomicOrdering::SeqCst);
        self.inner
            .lanes
            .lock()
            .expect("session lanes poisoned")
            .clear();
        self.inner.closing.notify_waiters();
    }
}

/// A lane's share of its session's lifetime.
struct LaneLifetime {
    token: Option<oneshot::Receiver<()>>,
    reported: bool,
}

impl LaneLifetime {
    fn new(token: oneshot::Receiver<()>) -> Self {
        Self {
            token: Some(token),
            reported: false,
        }
    }

    /// Whether the session has closed, registering `cx` if it has not.
    fn poll_closed(&mut self, cx: &mut TaskContext<'_>) -> bool {
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

/// The caller's end of an in-process lane.
pub struct LocalClientLane<P: Protocol> {
    tx: mpsc::Sender<Frame<P::Request>>,
    rx: mpsc::Receiver<Frame<P::Response>>,
    order: LaneOrder,
    lifetime: LaneLifetime,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol> LocalClientLane<P> {
    /// A handle for writing to this lane from several tasks at once.
    ///
    /// r[impl jetstream.session.local.order-handoff]
    /// The lane's own `Sink` is sequential and needs no help. Concurrent
    /// producers do: use one or the other on a given lane, not both.
    pub fn ordered_sender(&self) -> OrderedSender<P> {
        OrderedSender {
            tx: Arc::new(Mutex::new(self.tx.clone())),
            order: self.order.clone(),
        }
    }
}

/// Writes frames to one lane in the order their tickets were taken.
///
/// r[impl jetstream.session.local.order-handoff]
/// Take the ticket with [`OrderedSender::admit`] where the order is
/// decided — before spawning, not inside the spawned task — then hand it
/// to [`OrderedSender::deliver`]. A ticket dropped without delivering
/// passes its place on rather than releasing it.
pub struct OrderedSender<P: Protocol> {
    tx: Arc<Mutex<mpsc::Sender<Frame<P::Request>>>>,
    order: LaneOrder,
}

impl<P: Protocol> Clone for OrderedSender<P> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            order: self.order.clone(),
        }
    }
}

impl<P: Protocol> OrderedSender<P> {
    /// Take this frame's place in the lane's delivery order.
    pub fn admit(&self) -> OrderTicket {
        self.order.ticket()
    }

    /// Deliver `frame` when its ticket's turn comes.
    pub async fn deliver(
        &self,
        ticket: OrderTicket,
        frame: Frame<P::Request>,
    ) -> Result<(), SessionError> {
        ticket.wait().await;
        let result = {
            let mut tx = self.tx.lock().await;
            tx.send(frame).await
        };
        ticket.complete();
        result.map_err(|_| SessionError::LaneClosed)
    }

    /// Admit and deliver in one step, for a caller that is not handing
    /// the frame to another task.
    pub async fn send(
        &self,
        frame: Frame<P::Request>,
    ) -> Result<(), SessionError> {
        let ticket = self.admit();
        self.deliver(ticket, frame).await
    }
}

impl<P: Protocol> Sink<Frame<P::Request>> for LocalClientLane<P> {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        if this.lifetime.poll_closed(cx) {
            return Poll::Ready(Err(SessionError::Closed.into()));
        }
        Pin::new(&mut this.tx)
            .poll_ready(cx)
            .map_err(|_| SessionError::LaneClosed.into())
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: Frame<P::Request>,
    ) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().tx)
            .start_send(item)
            .map_err(|_| SessionError::LaneClosed.into())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().tx)
            .poll_flush(cx)
            .map_err(|_| SessionError::LaneClosed.into())
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().tx)
            .poll_close(cx)
            .map_err(|_| SessionError::LaneClosed.into())
    }
}

impl<P: Protocol> Stream for LocalClientLane<P> {
    type Item = Result<Frame<P::Response>, Error>;

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
        Pin::new(&mut this.rx).poll_next(cx).map(|f| f.map(Ok))
    }
}

/// The callee's end of an in-process lane.
pub struct LocalServiceLane<P: Protocol> {
    tx: mpsc::Sender<Frame<P::Response>>,
    rx: mpsc::Receiver<Frame<P::Request>>,
    lifetime: LaneLifetime,
    _p: PhantomData<fn() -> P>,
}

impl<P: Protocol> Sink<Frame<P::Response>> for LocalServiceLane<P> {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        if this.lifetime.poll_closed(cx) {
            return Poll::Ready(Err(SessionError::Closed.into()));
        }
        Pin::new(&mut this.tx)
            .poll_ready(cx)
            .map_err(|_| SessionError::LaneClosed.into())
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: Frame<P::Response>,
    ) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().tx)
            .start_send(item)
            .map_err(|_| SessionError::LaneClosed.into())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().tx)
            .poll_flush(cx)
            .map_err(|_| SessionError::LaneClosed.into())
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().tx)
            .poll_close(cx)
            .map_err(|_| SessionError::LaneClosed.into())
    }
}

impl<P: Protocol> Stream for LocalServiceLane<P> {
    type Item = Result<Frame<P::Request>, Error>;

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
        Pin::new(&mut this.rx).poll_next(cx).map(|f| f.map(Ok))
    }
}

/// r[impl jetstream.session.local]
/// An in-process session authenticates nothing, so the callee sees an
/// empty context rather than a fabricated identity.
impl<P: Protocol> Contextual for LocalServiceLane<P> {
    fn context(&self) -> Context {
        Context::default()
    }
}
