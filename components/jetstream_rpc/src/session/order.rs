//! Delivery order taken at the call site.
//!
//! r[impl jetstream.session.local.order-handoff]
//! A lane whose delivery is admitted asynchronously cannot take its
//! order from the order in which admission completes: two tasks that
//! were handed frames in a known order will finish acquiring the send
//! half in an arbitrary one. [`LaneOrder`] hands out a ticket
//! synchronously, at the point where the order is decided, and delivery
//! waits for its ticket's turn.
//!
//! A ticket that is dropped without delivering passes its place to the
//! next frame on the lane rather than releasing it, so a frame that is
//! abandoned mid-flight cannot let a later frame overtake an earlier one
//! that is still in flight.

use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use tokio::sync::Notify;

#[derive(Debug, Default)]
struct OrderState {
    /// The ticket whose turn it is.
    turn: u64,
    /// Tickets dropped before their turn arrived. Skipped when reached.
    abandoned: BTreeSet<u64>,
}

#[derive(Debug, Default)]
struct OrderInner {
    next: AtomicU64,
    state: Mutex<OrderState>,
    notify: Notify,
}

impl OrderInner {
    /// Move past `seq`, skipping any ticket abandoned in the meantime.
    fn advance_locked(state: &mut OrderState, seq: u64) {
        let mut turn = seq + 1;
        while state.abandoned.remove(&turn) {
            turn += 1;
        }
        state.turn = turn;
    }

    fn release(&self, seq: u64) {
        let mut state = self.state.lock().expect("lane order poisoned");
        if state.turn == seq {
            Self::advance_locked(&mut state, seq);
            drop(state);
            self.notify.notify_waiters();
        } else {
            // Not our turn yet: mark the slot so that whoever reaches it
            // skips straight over us.
            state.abandoned.insert(seq);
        }
    }
}

/// Hands out delivery order for one lane.
///
/// Cheap to clone; every clone shares one order.
#[derive(Debug, Clone, Default)]
pub struct LaneOrder {
    inner: Arc<OrderInner>,
}

impl LaneOrder {
    /// A fresh order, starting at ticket zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Take the next place in line.
    ///
    /// This is deliberately synchronous: it must run where the order is
    /// decided, not inside the task that later performs the delivery.
    pub fn ticket(&self) -> OrderTicket {
        let seq = self.inner.next.fetch_add(1, Ordering::SeqCst);
        OrderTicket {
            seq,
            inner: self.inner.clone(),
            done: false,
        }
    }

    /// The ticket whose turn it currently is.
    pub fn turn(&self) -> u64 {
        self.inner.state.lock().expect("lane order poisoned").turn
    }
}

/// One frame's place in a lane's delivery order.
///
/// Dropping a ticket without calling [`OrderTicket::complete`] hands its
/// place to the next frame on the lane.
#[derive(Debug)]
pub struct OrderTicket {
    seq: u64,
    inner: Arc<OrderInner>,
    done: bool,
}

impl OrderTicket {
    /// This ticket's position in the order.
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// Wait until it is this ticket's turn to deliver.
    pub async fn wait(&self) {
        loop {
            let notified = self.inner.notify.notified();
            tokio::pin!(notified);
            // Register before checking, so a release between the check
            // and the await cannot be missed.
            notified.as_mut().enable();

            {
                let state =
                    self.inner.state.lock().expect("lane order poisoned");
                if state.turn == self.seq {
                    return;
                }
            }

            notified.await;
        }
    }

    /// Record that this ticket's frame has been delivered and let the
    /// next one go.
    pub fn complete(mut self) {
        self.done = true;
        self.inner.release(self.seq);
    }
}

impl Drop for OrderTicket {
    fn drop(&mut self) {
        if !self.done {
            self.inner.release(self.seq);
        }
    }
}
