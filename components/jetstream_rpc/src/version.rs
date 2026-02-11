use std::{fmt::Display, str::FromStr};

use jetstream_wireformat::{JetStreamWireFormat, WireFormat};

use crate::Framer;

pub const TVERSION: u8 = 100;
pub const RVERSION: u8 = TVERSION + 1;

/// version -- negotiate protocol version
///
/// ```text
/// size[4] Tversion tag[2] msize[4] version[s]
/// ```
///
/// version establishes the msize, which is the maximum message size inclusive of the size value that can be handled by both client and server.
///
/// It also establishes the protocol version. For 9P2000.L version must be the string 9P2000.L.
///
/// See the Plan 9 manual page for [version(5)](http://9p.io/magic/man2html/5/version).
#[derive(Debug, JetStreamWireFormat)]
pub struct Tversion {
    pub msize: u32,
    pub version: String,
}

/// version -- negotiate protocol version
///
/// ```text
/// size[4] Rversion tag[2] msize[4] version[s]
/// ```
///
/// version establishes the msize, which is the maximum message size inclusive of the size value that can be handled by both client and server.
///
/// It also establishes the protocol version. For 9P2000.L version must be the string 9P2000.L.
///
/// See the Plan 9 manual page for [version(5)](http://9p.io/magic/man2html/5/version).
#[derive(Debug, JetStreamWireFormat)]
pub struct Rversion {
    pub msize: u32,
    pub version: String,
}

pub enum VersionFrame {
    Tversion(Tversion),
    Rversion(Rversion),
}

impl Framer for VersionFrame {
    fn message_type(&self) -> u8 {
        match self {
            VersionFrame::Tversion(_) => TVERSION,
            VersionFrame::Rversion(_) => RVERSION,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            VersionFrame::Tversion(tversion) => tversion.byte_size(),
            VersionFrame::Rversion(rversion) => rversion.byte_size(),
        }
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            VersionFrame::Tversion(tversion) => tversion.encode(writer),
            VersionFrame::Rversion(rversion) => rversion.encode(writer),
        }
    }

    fn decode<R: std::io::Read>(
        reader: &mut R,
        ty: u8,
    ) -> std::io::Result<Self> {
        match ty {
            TVERSION => {
                let tversion = Tversion::decode(reader)?;
                Ok(VersionFrame::Tversion(tversion))
            }
            RVERSION => {
                let rversion = Rversion::decode(reader)?;
                Ok(VersionFrame::Rversion(rversion))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid version frame type: {}", ty),
            )),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Version {
    V9P2000L,
    V9P2000,
    /// example rs.jetstream.proto/echohttp/15.0.0+bfd7d20e
    JetStream {
        name: String,
        version: semver::Version,
    },
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::V9P2000L => write!(f, "9P2000.L"),
            Version::V9P2000 => write!(f, "9P2000"),
            Version::JetStream { name, version } => {
                write!(f, "rs.jetstream.proto/{}/{}", name, version)
            }
        }
    }
}

impl TryFrom<VersionFrame> for Version {
    type Error = String;

    fn try_from(value: VersionFrame) -> Result<Self, Self::Error> {
        match value {
            VersionFrame::Tversion(tversion) => {
                Version::from_str(&tversion.version)
            }
            VersionFrame::Rversion(rversion) => {
                Version::from_str(&rversion.version)
            }
        }
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "9P2000.L" => Ok(Version::V9P2000L),
            "9P2000" => Ok(Version::V9P2000),
            //example rs.jetstream.proto/echohttp/15.0.0+bfd7d20e
            s if s.starts_with("rs.jetstream.proto") => {
                let parts: Vec<&str> = s.split('/').collect();
                if parts.len() != 3 {
                    return Err(format!(
                        "Invalid JetStream version format: {}",
                        s
                    ));
                }
                let name = parts[1].to_string();
                let version_str = parts[2];
                let version = semver::Version::parse(version_str).map_err(|e| {
                    format!("Invalid semver version in JetStream version: {}: {}", version_str, e)
                })?;
                Ok(Version::JetStream { name, version })
            }
            _ => Err(format!("Unknown version string: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_from_str() {
        assert_eq!(Version::from_str("9P2000.L").unwrap(), Version::V9P2000L);
        assert_eq!(Version::from_str("9P2000").unwrap(), Version::V9P2000);
        assert_eq!(
            Version::from_str("rs.jetstream.proto/echohttp/15.0.0+bfd7d20e")
                .unwrap(),
            Version::JetStream {
                name: "echohttp".to_string(),
                version: semver::Version::parse("15.0.0+bfd7d20e").unwrap(),
            }
        );

        assert!(Version::from_str("invalid").is_err());
    }
}
