#[cfg(all(feature = "s2n-quic", not(target_os = "windows")))]
impl From<s2n_quic::provider::tls::default::error::Error> for crate::Error {
    fn from(value: s2n_quic::provider::tls::default::error::Error) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::s2n_quic::tls_error"),
        )
    }
}

#[cfg(feature = "s2n-quic")]
impl From<s2n_quic::connection::Error> for crate::Error {
    fn from(value: s2n_quic::connection::Error) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::s2n_quic::connection_error"),
        )
    }
}

#[cfg(feature = "s2n-quic")]
impl From<s2n_quic::provider::StartError> for crate::Error {
    fn from(value: s2n_quic::provider::StartError) -> Self {
        crate::Error::from(
            miette::MietteDiagnostic::new(value.to_string())
                .with_code("jetstream::s2n_quic::start_error"),
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
