#[cfg(feature = "quinn")]
impl From<quinn::ConnectionError> for crate::Error {
    fn from(value: quinn::ConnectionError) -> Self {
        crate::Error::with_code(
            value.to_string(),
            "jetstream::quinn::connection_error",
        )
    }
}

#[cfg(feature = "quinn")]
impl From<quinn::WriteError> for crate::Error {
    fn from(value: quinn::WriteError) -> Self {
        crate::Error::with_code(
            value.to_string(),
            "jetstream::quinn::write_error",
        )
    }
}

#[cfg(feature = "quinn")]
impl From<quinn::ReadError> for crate::Error {
    fn from(value: quinn::ReadError) -> Self {
        crate::Error::with_code(
            value.to_string(),
            "jetstream::quinn::read_error",
        )
    }
}

#[cfg(feature = "quinn")]
impl From<quinn::ClosedStream> for crate::Error {
    fn from(value: quinn::ClosedStream) -> Self {
        crate::Error::with_code(
            value.to_string(),
            "jetstream::quinn::closed_stream",
        )
    }
}

#[cfg(feature = "iroh")]
impl From<iroh::endpoint::ConnectionError> for crate::Error {
    fn from(value: iroh::endpoint::ConnectionError) -> Self {
        crate::Error::with_code(
            value.to_string(),
            "jetstream::iroh::connection_error",
        )
    }
}

impl From<std::convert::Infallible> for crate::Error {
    fn from(value: std::convert::Infallible) -> Self {
        match value {}
    }
}

impl From<std::net::AddrParseError> for crate::Error {
    fn from(value: std::net::AddrParseError) -> Self {
        crate::Error::with_code(
            value.to_string(),
            "jetstream::addr_parse_error",
        )
    }
}
