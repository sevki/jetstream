//! Sessions and lanes.
//!
//! r[impl jetstream.session.overview]
//! A **session** is the association with a peer: it carries identity and
//! capabilities and can open and accept lanes. A **lane** is one
//! ordered, reliable, flow-controlled sequence of frames — exactly what
//! [`ClientTransport`] describes from one end and [`ServiceTransport`]
//! from the other. A session orders nothing; ordering is a property of a
//! lane and only of a lane.
//!
//! r[impl jetstream.session.overview.additive]
//! Nothing here changes the [`ClientTransport`] or [`ServiceTransport`]
//! bounds. A lane *is* a value satisfying them; what a session adds is a
//! way to obtain one without naming the concrete transport. Code that
//! holds a single transport and multiplexes by tag keeps working.

mod capabilities;
mod error;
pub mod lifetime;
pub mod local;
mod order;
mod single;

#[cfg(test)]
mod tests;

use bytes::Bytes;
use jetstream_wireformat::WireFormat;

pub use self::{
    capabilities::{Capabilities, Capability, IdentityKind, LaneSupport},
    error::SessionError,
    lifetime::LaneGuard,
    local::{LocalSession, LocalSessionPair},
    order::{LaneOrder, OrderTicket},
    single::{NoClientLane, NoServiceLane, SingleLaneSession},
};
use crate::{
    client::ClientTransport, context::Context, server::ServiceTransport, Error,
    Frame, Framer, Protocol,
};

/// An association with a peer, from which lanes are obtained.
///
/// r[impl jetstream.session.trait]
/// Opening and accepting are fallible and asynchronous, and both take
/// `&self`: a session is usable from several tasks at once, and opening
/// a lane never requires exclusive access to the session.
///
/// r[impl jetstream.session.symmetric]
/// Where the transport is symmetric either peer may open a lane, and the
/// opener is the RPC caller on it. That is why the two directions have
/// different lane types: [`Session::open_lane`] yields the caller's end,
/// [`Session::accept_lane`] the callee's.
#[async_trait::async_trait]
pub trait Session<P: Protocol>: Send + Sync {
    /// The end of a lane this peer opened, driven as the RPC caller.
    type ClientLane: ClientTransport<P>;
    /// The end of a lane the peer opened, served as the RPC callee.
    type ServiceLane: ServiceTransport<P>;

    /// What this session can do.
    ///
    /// r[impl jetstream.session.capabilities]
    fn capabilities(&self) -> Capabilities;

    /// The peer's identity, in the form the transport established it.
    ///
    /// r[impl jetstream.session.identity]
    fn context(&self) -> Context {
        Context::default()
    }

    /// Open a new lane and take the caller's end of it.
    async fn open_lane(&self) -> Result<Self::ClientLane, SessionError>;

    /// Wait for the peer to open a lane and take the callee's end of it.
    async fn accept_lane(&self) -> Result<Self::ServiceLane, SessionError>;

    /// Close the session.
    ///
    /// r[impl jetstream.session.lifetime]
    /// Every lane opened on the session terminates, and calls in flight
    /// on those lanes fail rather than hang. Closing a lane does not
    /// close its session.
    async fn close(&self);
}

/// The optional unordered channel belonging to a session.
///
/// r[impl jetstream.session.datagrams]
/// Datagrams may be dropped, may arrive in any order, and are never
/// retransmitted by JetStream.
///
/// r[impl jetstream.session.datagrams.not-a-lane]
/// This is deliberately not a [`ClientTransport`]: a datagram channel is
/// not a lane, and none of the lane ordering requirements apply to it.
///
/// A transport implements the three raw methods; the four framed ones
/// are provided. That is deliberate. Every framed method has to check
/// the path limit before sending and reject a datagram carrying
/// anything after its frame, and when each transport wrote those out
/// itself the two implementations drifted — iroh rejected trailing
/// bytes and QUIC accepted them. Encoding and decoding belong to the
/// model, not to the transport.
///
/// r[impl jetstream.session.symmetric]
/// The channel belongs to the connection, so both directions exist at
/// both ends and the methods name the direction rather than assuming
/// one. A session is not fixed to a role — the peer that opens a lane
/// is the caller *on that lane* — so a datagram API that only sent
/// requests and only received responses was wrong at whichever end was
/// serving: a callee decoded its peer's requests as responses, which
/// for any protocol whose two message types differ means rejecting or
/// misreading every datagram it receives.
#[async_trait::async_trait]
pub trait Datagrams<P: Protocol>: Send + Sync
where
    // The framed methods are provided rather than required, so their
    // bodies own a frame across an await and need the message types to
    // outlive it. Every real protocol's messages are owned values, so
    // this costs nothing but has to be said.
    P::Request: 'static,
    P::Response: 'static,
{
    /// The largest frame the path will carry, if the transport knows it.
    fn max_datagram_size(&self) -> Option<u32>;

    /// Send one already-encoded datagram, or fail. Never fragmented,
    /// never retried.
    async fn send_datagram_bytes(
        &self,
        bytes: Bytes,
    ) -> Result<(), SessionError>;

    /// Receive the bytes of the next datagram that arrived intact.
    async fn recv_datagram_bytes(&self) -> Result<Bytes, SessionError>;

    /// Send a request, as the caller.
    async fn send_request_datagram(
        &self,
        frame: Frame<P::Request>,
    ) -> Result<(), SessionError> {
        self.send_datagram_bytes(encode_datagram(
            &frame,
            self.max_datagram_size(),
        )?)
        .await
    }

    /// Receive the next request, as the callee.
    async fn recv_request_datagram(
        &self,
    ) -> Result<Frame<P::Request>, SessionError> {
        decode_datagram(self.recv_datagram_bytes().await?)
    }

    /// Send a response, as the callee.
    async fn send_response_datagram(
        &self,
        frame: Frame<P::Response>,
    ) -> Result<(), SessionError> {
        self.send_datagram_bytes(encode_datagram(
            &frame,
            self.max_datagram_size(),
        )?)
        .await
    }

    /// Receive the next response, as the caller.
    async fn recv_response_datagram(
        &self,
    ) -> Result<Frame<P::Response>, SessionError> {
        decode_datagram(self.recv_datagram_bytes().await?)
    }
}

/// r[impl jetstream.session.datagrams]
/// Reject an oversized frame at the send site, which is the only place
/// that can report it: a frame that exceeds the path limit is not
/// fragmented, so there is nothing further down to notice.
pub fn check_datagram_size<T: Framer>(
    frame: &Frame<T>,
    limit: Option<u32>,
) -> Result<(), SessionError> {
    let Some(limit) = limit else {
        return Ok(());
    };
    let size = WireFormat::byte_size(frame);
    if size > limit {
        return Err(SessionError::DatagramTooLarge { size, limit });
    }
    Ok(())
}

/// r[impl jetstream.session.datagrams]
/// Encode a frame for the datagram channel, refusing one the path
/// cannot carry.
pub fn encode_datagram<T: Framer>(
    frame: &Frame<T>,
    limit: Option<u32>,
) -> Result<Bytes, SessionError> {
    check_datagram_size(frame, limit)?;

    let mut buf = Vec::with_capacity(WireFormat::byte_size(frame) as usize);
    WireFormat::encode(frame, &mut buf)
        .map_err(|err| SessionError::Transport(Error::from(err)))?;
    Ok(Bytes::from(buf))
}

/// r[impl jetstream.session.datagrams]
/// Decode one datagram, which carries a complete frame or is discarded.
/// There is nothing to reassemble it from, so bytes after the frame mean
/// it is not a complete frame either: decoding would succeed on the
/// prefix and hand back something the peer never sent.
pub fn decode_datagram<T: Framer>(
    datagram: Bytes,
) -> Result<Frame<T>, SessionError> {
    let mut reader = std::io::Cursor::new(datagram.as_ref());
    let frame = <Frame<T> as WireFormat>::decode(&mut reader)
        .map_err(|err| SessionError::Transport(Error::from(err)))?;

    let consumed = reader.position() as usize;
    if consumed != datagram.len() {
        return Err(SessionError::Transport(Error::new(format!(
            "datagram has {} trailing bytes after the frame",
            datagram.len() - consumed
        ))));
    }

    Ok(frame)
}
