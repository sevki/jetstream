//! Randomised scenarios over a simulated network.
//!
//! `turmoil_subscriptions.rs` pins named rules with hand-written stories.
//! Those prove the promises hold on the paths someone thought of, which
//! is not the same as their holding. This searches instead: a seeded
//! sequence of actions — subscribe, read, drop, call, partition, repair,
//! wait — run against invariants that must survive *any* ordering.
//!
//! The invariants are the point. A scenario runner with no oracle finds
//! panics and hangs and nothing else, which is a very expensive way to
//! discover that the code does not crash.
//!
//! Everything here is deterministic: turmoil's network is seeded, its
//! clock is simulated, and `jetstream_rpc` reads neither a wall clock nor
//! a random number generator. A failing seed reproduces exactly.

// Mounted by three targets — the seed walk, the fuzz target, and the
// regression replay — and each uses a different part of it: only the
// walk builds scenarios from seeds, only the fuzzer builds them from
// bytes. Warning about the unused half in each is noise about a shared
// module doing its job.
#![allow(dead_code)]

use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};

use futures::StreamExt;
use jetstream::prelude::*;
use jetstream_rpc::session::{Session, SingleLaneSession};
use turmoil::{
    net::{TcpListener, TcpStream},
    Builder,
};

use self::churn_protocol::{ChurnChannel, ChurnService};

#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Tick(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, JetStreamWireFormat)]
pub struct Closed(pub u64);

#[service(uses(super::{Tick, Closed}))]
pub trait Churn {
    async fn bump(&self, ctx: Context) -> Result<u64>;

    #[subscription]
    fn feed(&self, ctx: Context) -> Subscription<Tick, Closed>;
}

#[derive(Clone, Default)]
struct Feeder {
    sent: Arc<AtomicU64>,
    /// Producers still running. The scenario's strongest oracle: after
    /// every subscription is gone and the network is whole, this must
    /// reach zero, whatever order things happened in.
    producers: Arc<AtomicUsize>,
}

impl Churn for Feeder {
    async fn bump(&self, _ctx: Context) -> Result<u64> {
        Ok(self.sent.fetch_add(1, SeqCst))
    }

    fn feed(&self, _ctx: Context) -> Subscription<Tick, Closed> {
        let producers = self.producers.clone();
        Subscription::producing(4, move |producer| async move {
            producers.fetch_add(1, SeqCst);
            let mut n = 0u64;
            while producer.send(Tick(n)).await.is_ok() {
                n += 1;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            producers.fetch_sub(1, SeqCst);
        })
    }
}

/// One step a scenario can take.
///
/// Deliberately small: every variant is something a caller or a network
/// can actually do, so any sequence of them is a run the implementation
/// has to survive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Open another subscription on the shared lane.
    Subscribe,
    /// Read one item from the oldest live subscription.
    Read,
    /// Drop the oldest live subscription — the caller's way to cancel.
    Drop,
    /// A unary call, which must keep working throughout.
    Bump,
    /// Cut the network.
    Partition,
    /// Restore it.
    Repair,
    /// Let time pass, so producers run and frames move.
    Sleep { millis: u8 },
}

impl Action {
    /// One action from two numbers: which one, and its parameter.
    ///
    /// Both generators go through here, so the seed walk and the fuzzer
    /// explore the same space. If they diverged, a seed filed by one
    /// would not reproduce under the other, and the committed seed
    /// corpus would quietly stop meaning anything.
    ///
    /// The weighting is deliberate. `Read` is the commonest because
    /// delivery is what the invariants are about; `Partition` and
    /// `Repair` are rarer because a scenario that spends its length
    /// cutting the network tests turmoil rather than this code.
    fn from_pair(choice: u64, param: u64) -> Self {
        match choice % 10 {
            0..=1 => Action::Subscribe,
            2..=4 => Action::Read,
            5 => Action::Drop,
            6 => Action::Bump,
            7 => Action::Partition,
            8 => Action::Repair,
            _ => Action::Sleep {
                millis: (param % 120) as u8,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scenario {
    pub seed: u64,
    pub actions: Vec<Action>,
}

impl Scenario {
    /// Build one from a seed. The same seed always yields the same
    /// scenario *and* the same network schedule, so a failure reported
    /// as a seed is a failure anyone can re-run.
    pub fn from_seed(seed: u64) -> Self {
        let mut rng = Lcg::new(seed);
        let len = 6 + (rng.next() % 24) as usize;
        let actions = (0..len)
            .map(|_| Action::from_pair(rng.next(), rng.next()))
            .collect();
        Scenario { seed, actions }
    }

    /// Build one from an arbitrary byte string.
    ///
    /// This is what makes the scenarios fuzzable. `from_seed` derives
    /// both the action sequence and the network schedule from a single
    /// number, which is what lets a finding be reported as one — but it
    /// also means the only way to explore is to count upwards, blind to
    /// whether the next seed reaches anything the last one did not.
    /// Bytes let a coverage-guided fuzzer steer: it keeps the inputs
    /// that reached new code and mutates those.
    ///
    /// The first eight bytes are turmoil's network seed. The rest are
    /// the actions, two bytes each — which action, and its parameter.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        // Too short to describe anything. Rejected rather than padded:
        // padding would make every short input describe the same empty
        // scenario, and libfuzzer would fill the corpus with them.
        let (head, tail) = data.split_at_checked(8)?;
        let seed = u64::from_le_bytes(head.try_into().ok()?);

        let actions: Vec<Action> = tail
            .chunks_exact(2)
            .take(MAX_ACTIONS)
            .map(|pair| Action::from_pair(pair[0] as u64, pair[1] as u64))
            .collect();

        // An empty sequence runs no actions and asserts nothing, so it
        // is not a scenario. Saying so here keeps the runner's
        // preconditions in one place.
        if actions.is_empty() {
            return None;
        }
        Some(Scenario { seed, actions })
    }
}

/// How long a fuzzed scenario may get.
///
/// Length alone is never interesting, and nothing penalises it: given
/// the chance libfuzzer will grow an input forever, and every extra
/// action costs simulated time in every later execution. The seed walk
/// tops out at 29 actions; this leaves generous room above that while
/// keeping one execution bounded.
const MAX_ACTIONS: usize = 64;

/// A tiny deterministic generator, so the scenario shape needs no
/// dependency and no global state. Numerical Recipes' constants.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        // Avoid the fixed point at zero.
        Lcg(seed.wrapping_mul(6364136223846793005).wrapping_add(1))
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

const PORT: u16 = 1740;

/// What a subscription is allowed to do next, from the caller's side.
///
/// r[impl jetstream.subscription.surface.termination]
/// An ending is final. Tracking it per subscription is what turns "an
/// item arrived" into an assertion: after `Done` or a failure, nothing
/// more may come under that subscription, ever.
#[derive(Debug, PartialEq, Eq)]
enum Live {
    Running,
    Ended,
}

/// Run one scenario, asserting the invariants that must hold whatever
/// the ordering was. Returns the number of items observed, so a caller
/// can tell a scenario that exercised something from one that did not.
pub fn run_scenario(scenario: Scenario) -> turmoil::Result<usize> {
    let feeder = Feeder::default();
    let producers = feeder.producers.clone();
    let observed = Arc::new(AtomicUsize::new(0));
    let seen = observed.clone();

    let mut sim = Builder::new()
        .rng_seed(scenario.seed)
        .simulation_duration(Duration::from_secs(120))
        .build();

    sim.host("server", move || {
        let feeder = feeder.clone();
        async move {
            let listener =
                TcpListener::bind((IpAddr::from(Ipv4Addr::UNSPECIFIED), PORT))
                    .await?;
            loop {
                let (stream, _) = listener.accept().await?;
                let feeder = feeder.clone();
                tokio::spawn(async move {
                    let session =
                        SingleLaneSession::<ChurnChannel, _, _>::service_io(
                            stream,
                        );
                    let Ok(lane) =
                        Session::<ChurnChannel>::accept_lane(&session).await
                    else {
                        return;
                    };
                    let mut service = ChurnService { inner: feeder };
                    let _ =
                        jetstream_rpc::server::run(&mut service, lane).await;
                });
            }
        }
    });

    let actions = scenario.actions.clone();
    let producers_at_end = producers.clone();

    sim.client("client", async move {
        let stream = TcpStream::connect(("server", PORT)).await?;
        let session =
            SingleLaneSession::<ChurnChannel, _, _>::client_io(stream);
        let lane = Session::<ChurnChannel>::open_lane(&session)
            .await
            .expect("the one lane is available");
        let channel = ChurnChannel::new(64, Box::new(lane));

        // Oldest first, so `Read` and `Drop` are well defined without
        // needing the scenario to name an index.
        let mut live: VecDeque<(
            subscription::Subscription<Tick, Closed>,
            Live,
        )> = VecDeque::new();
        let mut partitioned = false;
        let mut ever_partitioned = false;

        for action in actions {
            match action {
                Action::Subscribe => {
                    // Cap it: a scenario that opens hundreds tests the
                    // machine, not the code.
                    if live.len() < 4 {
                        let mut feed = channel.feed(Context::default());
                        // Establishing under a partition would block
                        // forever, which is correct and untestable.
                        if !partitioned {
                            let _ = tokio::time::timeout(
                                Duration::from_secs(2),
                                feed.establish(),
                            )
                            .await;
                        }
                        live.push_back((feed, Live::Running));
                    }
                }
                Action::Read => {
                    if let Some((feed, state)) = live.front_mut() {
                        let read = tokio::time::timeout(
                            Duration::from_millis(400),
                            feed.next(),
                        )
                        .await;
                        match read {
                            // A partition, or simply nothing ready.
                            Err(_) => {}
                            Ok(item) => {
                                // r[verify jetstream.subscription.surface.termination]
                                // Nothing may follow an ending.
                                assert_eq!(
                                    *state,
                                    Live::Running,
                                    "a subscription that had already ended \
                                     produced {item:?}",
                                );
                                match item {
                                    Some(Ok(Item::Next(_))) => {
                                        seen.fetch_add(1, SeqCst);
                                    }
                                    Some(Ok(Item::Done(_)))
                                    | Some(Err(_))
                                    | None => *state = Live::Ended,
                                }
                            }
                        }
                    }
                }
                Action::Drop => {
                    live.pop_front();
                }
                Action::Bump => {
                    // Under a partition this cannot answer; that is the
                    // network's doing, not the lane's, so a timeout here
                    // is not a failure.
                    let _ = tokio::time::timeout(
                        Duration::from_millis(400),
                        channel.bump(Context::default()),
                    )
                    .await;
                }
                Action::Partition => {
                    if !partitioned {
                        turmoil::partition("client", "server");
                        partitioned = true;
                        ever_partitioned = true;
                    }
                }
                Action::Repair => {
                    if partitioned {
                        turmoil::repair("client", "server");
                        partitioned = false;
                    }
                }
                Action::Sleep { millis } => {
                    tokio::time::sleep(Duration::from_millis(
                        millis as u64 + 1,
                    ))
                    .await;
                }
            }
        }

        // Whatever happened, end in a known state: every subscription
        // released.
        live.clear();

        // r[verify jetstream.subscription.cancel]
        // r[verify jetstream.subscription.surface.cancellation]
        // Every producer must stop once its subscriber has gone —
        // *provided the lane is still there to carry the cancellation*.
        //
        // A scenario that partitioned has no such lane. Turmoil's
        // partition severs an established connection permanently:
        // `repair` restores routing but not this socket, verified
        // directly — a call ten simulated seconds after the repair still
        // never answers. With no keepalive, neither peer can tell that
        // from a quiet moment, so the server goes on producing for a
        // subscriber that will never read again.
        //
        // That is a real gap, and it is filed rather than asserted here:
        // asserting it would demand of the implementation something it
        // has no mechanism to do, and asserting its *opposite* would
        // enshrine the gap as intended. So the invariant is claimed only
        // where it is honest, which is every scenario whose lane
        // survived.
        if !ever_partitioned {
            let deadline = Duration::from_secs(30);
            let started = tokio::time::Instant::now();
            while producers_at_end.load(SeqCst) != 0 {
                assert!(
                    started.elapsed() < deadline,
                    "every producer must stop once its subscriber has \
                     gone: {} still running",
                    producers_at_end.load(SeqCst),
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        Ok(())
    });

    sim.run()?;

    Ok(observed.load(SeqCst))
}
