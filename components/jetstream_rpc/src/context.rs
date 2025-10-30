#[cfg(tokio_unix)]
use std::ops::{Deref, DerefMut};
#[cfg(tokio_unix)]
use std::path::PathBuf;
use std::{collections::BTreeSet, fmt::Display, net::IpAddr};

use jetstream_wireformat::JetStreamWireFormat;
#[cfg(feature = "s2n-quic")]
use s2n_quic::stream::BidirectionalStream;
#[cfg(tokio_unix)]
use tokio::net::{unix::UCred, UnixStream};
#[cfg(any(feature = "s2n-quic", feature = "turmoil", tokio_unix))]
use tokio_util::codec::Framed;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Context {
    remote: Option<RemoteAddr>,
    peer: Option<Peer>,
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.peer {
            Some(Peer::NodeId(ref id)) => write!(f, "{}", id.0),
            #[cfg(tokio_unix)]
            Some(Peer::Unix(ref cred)) => write!(
                f,
                "{}",
                cred.process_path()
                    .expect("Failed to get process path")
                    .to_string_lossy()
            ),
            None => write!(f, "None"),
        }
    }
}

impl From<NodeId> for Context {
    fn from(value: NodeId) -> Self {
        Context {
            remote: None,
            peer: Some(Peer::NodeId(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RemoteAddr {
    #[cfg(tokio_unix)]
    UnixAddr(PathBuf),
    NodeAddr(NodeAddr),
    IpAddr(IpAddr),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Peer {
    #[cfg(tokio_unix)]
    Unix(Unix),
    NodeId(NodeId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, JetStreamWireFormat)]
pub struct NodeId(okid::OkId);

#[derive(Debug, Clone, PartialEq, Eq, Hash, JetStreamWireFormat)]
pub struct NodeAddr {
    id: NodeId,
    relay_url: Option<Url>,
    direct_addresses: BTreeSet<std::net::SocketAddr>,
}

#[cfg(feature = "iroh")]
impl From<iroh::PublicKey> for NodeId {
    fn from(value: iroh::PublicKey) -> Self {
        NodeId(value.into())
    }
}

#[cfg(feature = "iroh")]
impl From<NodeAddr> for iroh::NodeAddr {
    fn from(value: NodeAddr) -> Self {
        iroh::NodeAddr {
            node_id: value
                .id
                .0
                .try_into()
                .expect("Failed to convert NodeId to iroh::NodeId"),
            relay_url: if value.relay_url.is_some() {
                use iroh::RelayUrl;
                Some(RelayUrl::from(value.relay_url.unwrap()))
            } else {
                None
            },
            direct_addresses: value.direct_addresses.clone(),
        }
    }
}

#[cfg(feature = "iroh")]
impl From<iroh::NodeAddr> for NodeAddr {
    fn from(value: iroh::NodeAddr) -> Self {
        NodeAddr {
            id: NodeId(value.node_id.into()),
            relay_url: value.relay_url.map(|url| url.into()),
            direct_addresses: value.direct_addresses,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg(tokio_unix)]
pub struct Unix(UCred);

#[cfg(tokio_unix)]
impl std::hash::Hash for Unix {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(pid) = self.0.pid() {
            H::write_i32(state, pid);
        }
        H::write_u32(state, self.0.uid());
        H::write_u32(state, self.0.gid());
    }
}

#[cfg(tokio_unix)]
impl PartialEq for Unix {
    fn eq(&self, other: &Self) -> bool {
        self.0.pid() == other.0.pid()
            && self.0.uid() == other.0.uid()
            && self.0.gid() == other.0.gid()
    }
}

#[cfg(tokio_unix)]
impl Eq for Unix {}

#[cfg(tokio_unix)]
impl Deref for Unix {
    type Target = UCred;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(tokio_unix)]
impl DerefMut for Unix {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(tokio_unix)]
impl Unix {
    /// returns the process' path
    pub fn process_path(&self) -> Result<PathBuf, std::io::Error> {
        use std::fs::read_link;
        if let Some(pid) = self.pid() {
            read_link(format!("/proc/{}/exe", pid))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "PID not found",
            ))
        }
    }
}

pub trait Contextual {
    fn context(&self) -> Context;
}

#[cfg(tokio_unix)]
impl<U> Contextual for Framed<UnixStream, U> {
    fn context(&self) -> Context {
        let addr = self.get_ref().peer_addr().unwrap();
        let ucred = self.get_ref().peer_cred().unwrap();
        Context {
            remote: Some(RemoteAddr::UnixAddr(
                addr.as_pathname()
                    .expect("Failed to get path")
                    .to_path_buf(),
            )),
            peer: Some(Peer::Unix(Unix(ucred))),
        }
    }
}

#[cfg(feature = "s2n-quic")]
impl<U> Contextual for Framed<BidirectionalStream, U> {
    fn context(&self) -> Context {
        let addr = self
            .get_ref()
            .connection()
            .remote_addr()
            .expect("Failed to get remote address");
        Context {
            remote: Some(RemoteAddr::IpAddr(addr.ip())),
            peer: None,
        }
    }
}

#[cfg(feature = "turmoil")]
impl<U> Contextual for Framed<turmoil::net::TcpStream, U> {
    fn context(&self) -> Context {
        let addr = self.get_ref().peer_addr().unwrap();
        Context {
            remote: Some(RemoteAddr::IpAddr(addr.ip())),
            peer: None,
        }
    }
}

#[cfg(cloudflare)]
impl Contextual for worker::Request {
    fn context(&self) -> Context {
        Context {
            remote: None,
            peer: None,
        }
    }
}

impl Context {
    pub fn new(remote: Option<RemoteAddr>, peer: Option<Peer>) -> Self {
        Context { remote, peer }
    }
}
