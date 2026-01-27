#[cfg(tokio_unix)]
use std::ops::{Deref, DerefMut};
#[cfg(tokio_unix)]
use std::path::PathBuf;
use std::{collections::BTreeSet, fmt::Display, net::IpAddr};

use jetstream_wireformat::JetStreamWireFormat;
#[cfg(tokio_unix)]
use tokio::net::{unix::UCred, UnixStream};
#[cfg(any(feature = "turmoil", tokio_unix))]
use tokio_util::codec::Framed;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Context {
    remote: Option<RemoteAddr>,
    peer: Option<Peer>,
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.peer {
            Some(Peer::NodeId(id)) => write!(f, "{}", id.0),
            #[cfg(tokio_unix)]
            Some(Peer::Unix(cred)) => write!(
                f,
                "{}",
                cred.process_path()
                    .expect("Failed to get process path")
                    .to_string_lossy()
            ),
            #[cfg(feature = "x509")]
            Some(Peer::Tls(tls_peer)) => {
                if let Some(leaf) = tls_peer.leaf() {
                    if let Some(cn) = &leaf.common_name {
                        write!(f, "{}", cn)
                    } else {
                        write!(f, "{}", leaf.fingerprint)
                    }
                } else {
                    write!(f, "TLS(empty chain)")
                }
            }
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
    #[cfg(feature = "x509")]
    Tls(TlsPeer),
}

/// Parsed TLS certificate with extracted identity information
#[cfg(feature = "x509")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, JetStreamWireFormat)]
pub struct TlsCert {
    /// SHA-256 fingerprint of the certificate (hex encoded)
    pub fingerprint: String,
    /// Common Name from the certificate subject (if present)
    pub common_name: Option<String>,
    /// DNS names from SAN (e.g., "example.com")
    pub dns_names: Vec<String>,
    /// IP addresses from SAN (for P2P certs)
    pub ip_addresses: Vec<IpAddr>,
    /// Email addresses from SAN (RFC 822 names)
    pub emails: Vec<String>,
    /// URIs from SAN (e.g., "spiffe://cluster.local/ns/default/sa/myapp")
    pub uris: Vec<Url>,
}

#[cfg(feature = "x509")]
impl TlsCert {
    /// Parse a TLS certificate from DER-encoded bytes
    pub fn from_der(
        der: &[u8],
    ) -> Result<Self, x509_certificate::X509CertificateError> {
        use x509_certificate::X509Certificate;

        let cert = X509Certificate::from_der(der)?;

        let fingerprint = cert
            .sha256_fingerprint()
            .map(|d| hex::encode(d.as_ref()))
            .unwrap_or_default();

        let common_name = cert.subject_common_name();

        let mut dns_names = Vec::new();
        let mut ip_addresses = Vec::new();
        let mut emails = Vec::new();
        let mut uris = Vec::new();

        // OID for Subject Alternative Name: 2.5.29.17
        let oid_san = bcder::Oid(bytes::Bytes::from_static(&[85, 29, 17]));

        for ext in cert.iter_extensions() {
            if ext.id == oid_san {
                // Parse SANs manually since x509-certificate expects constructed
                // tags but RFC 5280 uses IMPLICIT (primitive) tags for most types
                let value_bytes = ext.value.clone().into_bytes();
                parse_subject_alt_names(
                    &value_bytes,
                    &mut dns_names,
                    &mut ip_addresses,
                    &mut emails,
                    &mut uris,
                );
            }
        }

        Ok(TlsCert {
            fingerprint,
            common_name,
            dns_names,
            ip_addresses,
            emails,
            uris,
        })
    }
}

/// Parse Subject Alternative Names from DER-encoded extension value.
/// RFC 5280 uses IMPLICIT tagging for GeneralName, so we parse manually.
#[cfg(feature = "x509")]
fn parse_subject_alt_names(
    data: &[u8],
    dns_names: &mut Vec<String>,
    ip_addresses: &mut Vec<IpAddr>,
    emails: &mut Vec<String>,
    uris: &mut Vec<Url>,
) {
    // SAN extension value is a SEQUENCE of GeneralName
    // Skip the outer SEQUENCE tag (0x30) and length
    if data.len() < 2 || data[0] != 0x30 {
        return;
    }

    let (seq_len, offset) = parse_der_length(&data[1..]);
    if offset == 0 {
        return;
    }
    let mut i = 1 + offset;
    let end = std::cmp::min(i + seq_len, data.len());

    // GeneralName IMPLICIT tags (context-specific, primitive):
    // [1] rfc822Name       - 0x81
    // [2] dNSName          - 0x82
    // [6] uniformResourceIdentifier - 0x86
    // [7] iPAddress        - 0x87
    while i + 2 <= end {
        let tag = data[i];
        let (content_len, len_offset) = parse_der_length(&data[i + 1..]);
        if len_offset == 0 {
            break;
        }
        let content_start = i + 1 + len_offset;
        let content_end = content_start + content_len;
        if content_end > end {
            break;
        }
        let content = &data[content_start..content_end];

        match tag {
            0x81 => {
                // rfc822Name (email)
                if let Ok(email) = std::str::from_utf8(content) {
                    emails.push(email.to_string());
                }
            }
            0x82 => {
                // dNSName
                if let Ok(dns) = std::str::from_utf8(content) {
                    dns_names.push(dns.to_string());
                }
            }
            0x86 => {
                // uniformResourceIdentifier
                if let Ok(uri_str) = std::str::from_utf8(content) {
                    if let Ok(url) = Url::parse(uri_str) {
                        uris.push(url);
                    }
                }
            }
            0x87 => {
                // iPAddress
                match content.len() {
                    4 => {
                        if let Ok(arr) = <[u8; 4]>::try_from(content) {
                            ip_addresses.push(IpAddr::V4(arr.into()));
                        }
                    }
                    16 => {
                        if let Ok(arr) = <[u8; 16]>::try_from(content) {
                            ip_addresses.push(IpAddr::V6(arr.into()));
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                // Skip other GeneralName types (otherName, x400Address, etc.)
            }
        }

        i = content_end;
    }
}

/// Parse DER length encoding. Returns (length, bytes_consumed).
#[cfg(feature = "x509")]
fn parse_der_length(data: &[u8]) -> (usize, usize) {
    if data.is_empty() {
        return (0, 0);
    }
    let first = data[0];
    if first < 0x80 {
        // Short form: length is the byte itself
        (first as usize, 1)
    } else if first == 0x80 {
        // Indefinite length - not valid in DER
        (0, 0)
    } else {
        // Long form: first byte indicates number of length bytes
        let num_bytes = (first & 0x7f) as usize;
        if num_bytes > data.len() - 1 || num_bytes > 4 {
            return (0, 0);
        }
        let mut len = 0usize;
        for &b in &data[1..1 + num_bytes] {
            len = (len << 8) | (b as usize);
        }
        (len, 1 + num_bytes)
    }
}

/// TLS peer identity containing the certificate chain
#[cfg(feature = "x509")]
#[derive(Debug, Clone, PartialEq, Eq, Hash, JetStreamWireFormat)]
pub struct TlsPeer {
    /// Certificate chain (leaf cert first, then intermediates)
    pub chain: Vec<TlsCert>,
}

#[cfg(feature = "x509")]
impl TlsPeer {
    /// Parse a certificate chain from DER-encoded bytes
    pub fn from_der_chain<T: AsRef<[u8]>>(
        certs: &[T],
    ) -> Result<Self, x509_certificate::X509CertificateError> {
        let chain = certs
            .iter()
            .map(|c| TlsCert::from_der(c.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(TlsPeer { chain })
    }

    /// Get the leaf certificate (first in chain)
    pub fn leaf(&self) -> Option<&TlsCert> {
        self.chain.first()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, JetStreamWireFormat)]
pub struct NodeId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, JetStreamWireFormat)]
pub struct NodeAddr {
    id: NodeId,
    relay_url: Option<Url>,
    direct_addresses: BTreeSet<std::net::SocketAddr>,
}

#[cfg(feature = "iroh")]
impl From<iroh::PublicKey> for NodeId {
    fn from(value: iroh::PublicKey) -> Self {
        NodeId(value.to_string())
    }
}

#[cfg(feature = "iroh")]
impl From<NodeAddr> for iroh::NodeAddr {
    fn from(value: NodeAddr) -> Self {
        use std::str::FromStr;

        use iroh::PublicKey;

        iroh::NodeAddr {
            node_id: PublicKey::from_str(&value.id.0)
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
            id: NodeId(value.node_id.to_string()),
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

    /// Get the remote address
    pub fn remote(&self) -> Option<&RemoteAddr> {
        self.remote.as_ref()
    }

    /// Get the peer identity
    pub fn peer(&self) -> Option<&Peer> {
        self.peer.as_ref()
    }
}
