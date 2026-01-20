#![deny(missing_docs)]
//! Error handling for Jetstream
//!
#![doc = concat!("<div>", include_str!("output.svg"), "</div>")]

use std::ops::Deref;

mod extensions;

use jetstream_wireformat::WireFormat;
use miette::{Diagnostic, LabeledSpan, MietteDiagnostic, Severity};

/// Error represents a collection of errors that occurred during a Jetstream operation.
pub struct Error(miette::MietteDiagnostic);

/// IntoError trait allows conversion of various error types into Jetstream Error.
pub trait IntoError: Send + Sync {
    /// Converts the error into a Jetstream Error.
    fn into_error(self) -> Error;
}

impl<T: Diagnostic + Send + Sync + std::fmt::Display> IntoError for T {
    fn into_error(self) -> Error {
        Error::from_diagnostic(self)
    }
}

impl Error {
    /// Gets the inner miette::MietteDiagnostic.
    pub fn into_inner(self) -> miette::MietteDiagnostic {
        self.0
    }
}

/// Common error constants for RPC operations.
impl Error {
    /// Error returned when an invalid response is received.
    #[allow(non_upper_case_globals)]
    pub const InvalidResponse: Error = Error(MietteDiagnostic {
        message: String::new(),
        code: None,
        severity: None,
        help: None,
        url: None,
        labels: None,
    });

    /// Creates a new Error from a string.
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        let location = std::panic::Location::caller();
        #[cfg(feature = "test-paths")]
        let dir = "/root/test_dir";
        #[cfg(not(feature = "test-paths"))]
        let dir = env!("CARGO_MANIFEST_DIR");
        let label = format!(
            "file://{}/{}:{}:{}",
            dir,
            location.file(),
            location.line(),
            location.column()
        );
        Error(miette::MietteDiagnostic::new(message.into()).with_url(label))
    }

    /// Creates an invalid response error with a custom message.
    pub fn invalid_response(message: impl Into<String>) -> Self {
        Error(
            MietteDiagnostic::new(message.into())
                .with_code("jetstream::invalid_response"),
        )
    }

    /// Creates an Error from a Diagnostic type.
    pub fn from_diagnostic<D: Diagnostic + std::fmt::Display>(
        value: D,
    ) -> Self {
        let mut diagnostic = MietteDiagnostic::new(value.to_string());

        if let Some(code) = value.code() {
            diagnostic = diagnostic.with_code(code.to_string());
        }
        if let Some(severity) = value.severity() {
            diagnostic = diagnostic.with_severity(severity);
        }
        if let Some(help) = value.help() {
            diagnostic = diagnostic.with_help(help.to_string());
        }
        if let Some(url) = value.url() {
            diagnostic = diagnostic.with_url(url.to_string());
        }
        if let Some(labels) = value.labels() {
            diagnostic = diagnostic.with_labels(labels);
        }

        Error(diagnostic)
    }
}

impl Error {
    /// Creates an Error with a custom code.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.0 = self.0.with_code(code.into());
        self
    }
    /// Creates an Error with a custom severity.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.0 = self.0.with_severity(severity);
        self
    }
    /// Creates an Error with a custom help message.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.0 = self.0.with_help(help.into());
        self
    }
    /// Creates an Error with a custom URL.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.0 = self.0.with_url(url.into());
        self
    }
    /// Adds labels to the error.
    pub fn with_labels(
        mut self,
        labels: impl IntoIterator<Item = LabeledSpan>,
    ) -> Self {
        self.0 = self.0.with_labels(labels);
        self
    }

    /// Adds a label to the error.
    pub fn with_label(mut self, label: LabeledSpan) -> Self {
        self.0 = self.0.with_label(label);
        self
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error(
            MietteDiagnostic::new(value.to_string())
                .with_code(value.kind().to_string()),
        )
    }
}

impl From<MietteDiagnostic> for Error {
    fn from(value: MietteDiagnostic) -> Self {
        Error(value)
    }
}

impl Deref for Error {
    type Target = miette::MietteDiagnostic;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {}

impl Diagnostic for Error {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.0.code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.0.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.0.help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.0.url()
    }

    fn labels(
        &self,
    ) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.0.labels()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.0.source_code()
    }

    fn related<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        self.0.related()
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        self.0.diagnostic_source()
    }
}

impl From<Error> for miette::MietteDiagnostic {
    fn from(val: Error) -> Self {
        val.0
    }
}

/// Result is a type alias for a Result type that uses the Error type.
pub type Result<T> = std::result::Result<T, Error>;

impl WireFormat for Error {
    fn byte_size(&self) -> u32 {
        self.0.byte_size()
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>
    where
        Self: Sized,
    {
        self.0.encode(writer)
    }

    fn decode<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        Ok(Error(miette::MietteDiagnostic::decode(reader)?))
    }
}

#[cfg(test)]
mod tests;
