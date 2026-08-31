//! Sessions and lanes, without a network.
//!
//! A **session** is the association with a peer: it carries identity,
//! reports capabilities, and opens and accepts lanes. A **lane** is one
//! ordered, reliable sequence of frames — exactly what a
//! `ClientTransport` already is from one end and a `ServiceTransport`
//! from the other. A session orders nothing; ordering belongs to a lane
//! and only to a lane.
//!
//! This example uses `LocalSession`, the in-process binding, so it needs
//! no sockets and no certificates. The model is the same one the QUIC,
//! iroh and WebTransport bindings implement — see `quic_session.rs` for
//! the same shape over a real connection.
//!
//! ```console
//! cargo run --example session_lanes
//! ```

use std::time::Duration;

use jetstream::prelude::*;
use jetstream_macros::service;
use jetstream_rpc::session::{
    Capabilities, Capability, LaneSupport, LocalSession, Session, SessionError,
};
use tokio::time::sleep;

use crate::sleepy_protocol::{SleepyChannel, SleepyService};

#[service]
pub trait Sleepy {
    /// Nap for `ms` milliseconds, then say so.
    async fn nap(&self, ctx: Context, ms: u32) -> Result<String>;
}

#[derive(Debug, Clone)]
struct SleepyServer;

impl Sleepy for SleepyServer {
    async fn nap(&self, _ctx: Context, ms: u32) -> Result<String> {
        sleep(Duration::from_millis(ms as u64)).await;
        Ok(format!("slept {ms}ms"))
    }
}

#[tokio::main]
async fn main() {
    let pair = LocalSession::<SleepyChannel>::pair();

    // A session says what it can do rather than leaving callers to
    // infer it from the concrete transport type. Every binding reports
    // its own row: this one has many lanes but no datagrams, no
    // transport-level identity, and no migration.
    let caps = Session::<SleepyChannel>::capabilities(&pair.client);
    assert_eq!(caps, Capabilities::in_process());
    println!(
        "in-process session: lanes={} datagrams={} identity={}",
        caps.lanes, caps.datagrams, caps.identity
    );

    // Asking for something the session does not have fails loudly. The
    // alternative — emulating a datagram channel over a lane — would be
    // silently wrong about ordering and delivery, so it is not offered.
    match caps.require(Capability::Datagrams) {
        Err(SessionError::Unsupported(what)) => {
            println!("this session has no {what}, and says so")
        }
        other => panic!("expected the capability to be refused: {other:?}"),
    }

    // Serve every lane the peer opens. One task per lane: a lane is a
    // `ServiceTransport`, so it goes straight into the RPC loop with no
    // adapter in between.
    let server = pair.server.clone();
    let serving = tokio::spawn(async move {
        while let Ok(lane) =
            Session::<SleepyChannel>::accept_lane(&server).await
        {
            tokio::spawn(async move {
                let mut service = SleepyService {
                    inner: SleepyServer,
                };
                let _ = jetstream_rpc::server::run(&mut service, lane).await;
            });
        }
    });

    // Three lanes on the one session. `LaneSupport::Many` is what makes
    // this legal; a byte-stream session would refuse the second open
    // with `SessionError::LaneLimitReached` rather than queue behind the
    // first.
    assert_eq!(caps.lanes, LaneSupport::Many);
    let mut calls = Vec::new();
    for (lane_no, nap_ms) in [200u32, 20, 20].into_iter().enumerate() {
        let lane = Session::<SleepyChannel>::open_lane(&pair.client)
            .await
            .expect("the session is open");
        let channel = SleepyChannel::new(4, Box::new(lane));
        calls.push(tokio::spawn(async move {
            let started = std::time::Instant::now();
            let answer = channel.nap(Context::default(), nap_ms).await.unwrap();
            (lane_no, answer, started.elapsed())
        }));
    }

    // The lanes are independent sequences, so the long nap on the first
    // one does not hold up the other two.
    for call in calls {
        let (lane_no, answer, took) = call.await.unwrap();
        println!("lane {lane_no}: {answer} (round trip {took:?})");
    }

    // Closing the session ends every lane on it. Work in flight fails
    // rather than hanging, and a later open is refused outright.
    Session::<SleepyChannel>::close(&pair.client).await;
    match Session::<SleepyChannel>::open_lane(&pair.client).await {
        Err(SessionError::Closed) => {
            println!("the closed session opens no more lanes")
        }
        other => panic!("expected the session to be closed: {other:?}"),
    }

    serving.abort();
}
