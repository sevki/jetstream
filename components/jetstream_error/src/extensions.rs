#[cfg(feature = "quinn")]
impl From<quinn::ConnectionError> for crate::Error {
    fn from(value: quinn::ConnectionError) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::quinn::connection_error"),
        )
    }
}

#[cfg(feature = "quinn")]
impl From<quinn::WriteError> for crate::Error {
    fn from(value: quinn::WriteError) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::quinn::write_error"),
        )
    }
}

#[cfg(feature = "quinn")]
impl From<quinn::ReadError> for crate::Error {
    fn from(value: quinn::ReadError) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::quinn::read_error"),
        )
    }
}

#[cfg(feature = "quinn")]
impl From<quinn::ClosedStream> for crate::Error {
    fn from(value: quinn::ClosedStream) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::quinn::closed_stream"),
        )
    }
}

#[cfg(feature = "iroh")]
impl From<iroh::endpoint::ConnectionError> for crate::Error {
    fn from(value: iroh::endpoint::ConnectionError) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::iroh::connection_error"),
        )
    }
}

#[cfg(feature = "worker")]
impl From<worker::Error> for crate::Error {
    fn from(value: worker::Error) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::worker::error"),
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
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::addr_parse_error"),
        )
    }
}
