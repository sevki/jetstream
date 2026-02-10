#![deny(missing_docs)]
#![deny(clippy::result_large_err)]
//! Error handling for Jetstream
//!
//! r[impl jetstream.error.v2.type]
//! r[impl jetstream.error.v2.source-info.disable]
//! `jetstream::Error` is a struct containing a `Box<ErrorInner>` (message, code, help, url),
//! an optional `SpanTrace` captured at creation, and lazily-initialized `Backtrace` and
//! diagnostics state. The design is modeled after
//! [`TracedError`](https://docs.rs/tracing-error/latest/tracing_error/struct.TracedError.html)
//! from the `tracing-error` crate. The `Box` indirection on `ErrorInner` keeps `Error`
//! efficient for passing through `Result`.
#![doc = concat!("<div>", include_str!("output.svg"), "</div>")]

mod backtrace;
/// Structured span trace types for wire format serialization.
pub mod coding;
mod extensions;
mod result_size;

#[cfg(test)]
mod tests;

use std::sync::OnceLock;

use jetstream_wireformat::{JetStreamWireFormat, WireFormat};

use tracing_error::SpanTrace;

use crate::backtrace::{backtrace_from_spantrace, Backtrace, TraceDiagnostic};
// r[impl jetstream.error.v2.inner]
#[derive(Debug, Clone, JetStreamWireFormat)]
struct ErrorInner {
    message: String,
    code: Option<String>,
    help: Option<String>,
    url: Option<String>,
}

/// `Error` is the main error type for Jetstream
// r[impl jetstream.error.v2.span-trace]
#[derive(Clone)]
pub struct Error {
    inner: Box<ErrorInner>,
    span_trace: Option<Box<SpanTrace>>,
    backtrace: OnceLock<Box<Backtrace>>,
    diagnostics: OnceLock<Vec<TraceDiagnostic>>,
}

// r[impl jetstream.error.v2.into-error]
/// IntoError trait allows conversion of various error types into Jetstream Error.
pub trait IntoError: Send + Sync {
    /// Converts the error into a Jetstream Error.
    fn into_error(self) -> Error;
}

impl<T: std::error::Error + Send + Sync> IntoError for T {
    fn into_error(self) -> Error {
        Error::from_std_error(&self)
    }
}

impl Error {
    /// Returns the error message.
    pub fn message(&self) -> &str {
        &self.inner.message
    }

    /// Returns the error code, if any.
    pub fn code(&self) -> Option<&str> {
        self.inner.code.as_deref()
    }

    /// Returns the help text, if any.
    pub fn help_text(&self) -> Option<&str> {
        self.inner.help.as_deref()
    }

    /// Returns the URL, if any.
    pub fn url(&self) -> Option<&str> {
        self.inner.url.as_deref()
    }
}

/// r[impl jetstream.error.v2.span-trace.capture]
/// Construction methods — all paths automatically capture a SpanTrace.
impl Error {
    fn backtrace(&self) -> &Backtrace {
        self.backtrace.get_or_init(|| match &self.span_trace {
            Some(span) => backtrace_from_spantrace(span),
            None => Box::new(Backtrace::default()),
        })
    }

    fn diagnostics(&self) -> &Vec<TraceDiagnostic> {
        self.diagnostics
            .get_or_init(|| self.backtrace().diagnostics())
    }

    /// Creates a new `Error` from a message and an optional `SpanTrace`.
    #[track_caller]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            inner: Box::new(ErrorInner {
                message: message.into(),
                code: None,
                help: None,
                url: None,
            }),
            // r[impl jetstream.error.v2.span-trace.capture]
            span_trace: Some(Box::new(SpanTrace::capture())),
            backtrace: OnceLock::new(),
            diagnostics: OnceLock::new(),
        }
    }

    /// Creates a new `Error` with a message and an error code.
    #[track_caller]
    pub fn with_code(
        message: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        Self {
            inner: Box::new(ErrorInner {
                message: message.into(),
                code: Some(code.into()),
                help: None,
                url: None,
            }),
            // r[impl jetstream.error.v2.span-trace.capture]
            span_trace: Some(Box::new(SpanTrace::capture())),
            backtrace: OnceLock::new(),
            diagnostics: OnceLock::new(),
        }
    }

    /// Creates an invalid response error with a custom message.
    pub fn invalid_response(message: impl Into<String>) -> Self {
        Self::with_code(message, "jetstream::error::invalid_response")
    }

    /// Creates an Error from any std::error::Error type.
    pub fn from_std_error(err: &dyn std::error::Error) -> Self {
        Self {
            inner: Box::new(ErrorInner {
                message: err.to_string(),
                code: None,
                help: None,
                url: None,
            }),
            // r[impl jetstream.error.v2.span-trace.capture]
            span_trace: Some(Box::new(SpanTrace::capture())),
            backtrace: OnceLock::new(),
            diagnostics: OnceLock::new(),
        }
    }
}

/// r[impl jetstream.error.v2.builders]
/// Builder methods for setting optional fields.
impl Error {
    /// Sets the error code.
    pub fn set_code(mut self, code: impl Into<String>) -> Self {
        self.inner.code = Some(code.into());
        self
    }

    /// Sets the help text.
    pub fn set_help(mut self, help: impl Into<String>) -> Self {
        self.inner.help = Some(help.into());
        self
    }

    /// Sets the URL for further documentation.
    pub fn set_url(mut self, url: impl Into<String>) -> Self {
        self.inner.url = Some(url.into());
        self
    }
}

/// r[impl jetstream.error.v2.wireformat]
/// r[impl jetstream.error.v2.wireformat.message]
/// r[impl jetstream.error.v2.wireformat.code]
/// r[impl jetstream.error.v2.wireformat.backtrace]
impl WireFormat for Error {
    fn byte_size(&self) -> u32 {
        self.inner.byte_size() + self.backtrace().byte_size()
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>
    where
        Self: Sized,
    {
        self.inner.encode(writer)?;
        self.backtrace().encode(writer)
    }

    fn decode<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let inner = Box::new(ErrorInner::decode(reader)?);
        let bt = Box::new(Backtrace::decode(reader)?);
        let backtrace = OnceLock::new();
        backtrace.get_or_init(|| bt);
        Ok(Self {
            inner,
            span_trace: None,
            backtrace,
            diagnostics: OnceLock::new(),
        })
    }
}

// r[impl jetstream.error.v2.std-error]
impl std::error::Error for Error {}

// r[impl jetstream.error.v2.std-error]
// r[impl jetstream.error.v2.reporting.plain-text]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(code) = &self.inner.code {
            write!(f, "[{}] ", code)?;
        }
        write!(f, "{}", self.inner.message)?;

        #[cfg(not(feature = "miette"))]
        {
            let diagnostics = self.diagnostics();
            if !diagnostics.is_empty() {
                writeln!(f)?;
                for diag in diagnostics {
                    writeln!(f)?;
                    write!(f, "  in {}", diag.msg)?;
                    if let Some(file) = &diag.file {
                        write!(f, "\n    at {}:{}", file, diag.line)?;
                    }
                    if !diag.fields.is_empty() {
                        write!(f, "\n    with {}", diag.fields.join(", "))?;
                    }
                }
            }
        }

        Ok(())
    }
}

// r[impl jetstream.error.v2.std-error]
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("message", &self.inner.message)
            .field("code", &self.inner.code)
            .field("help", &self.inner.help)
            .field("url", &self.inner.url)
            .finish()
    }
}

// r[impl jetstream.error.v2.reporting.miette-feature]
// r[impl jetstream.error.v2.reporting.miette]
// r[impl jetstream.error.v2.reporting.span-trace-section]
// r[impl jetstream.error.v2.reporting.related]
// r[impl jetstream.error.v2.source-info.miette-integration]
// r[impl jetstream.error.v2.source-info.client-render]
#[cfg(feature = "miette")]
impl ::miette::Diagnostic for Error {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.inner
            .code
            .as_ref()
            .map(|code| Box::new(code.clone()) as Box<dyn std::fmt::Display>)
    }

    fn severity(&self) -> Option<::miette::Severity> {
        self.backtrace().severity().map(Into::into)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.inner
            .help
            .as_ref()
            .map(|help| Box::new(help.clone()) as Box<dyn std::fmt::Display>)
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.inner
            .url
            .as_ref()
            .map(|url| Box::new(url.clone()) as Box<dyn std::fmt::Display>)
    }

    fn source_code(&self) -> Option<&dyn ::miette::SourceCode> {
        None
    }

    fn labels(
        &self,
    ) -> Option<Box<dyn Iterator<Item = ::miette::LabeledSpan> + '_>> {
        None
    }

    fn related<'a>(
        &'a self,
    ) -> Option<Box<dyn Iterator<Item = &'a dyn ::miette::Diagnostic> + 'a>>
    {
        let diagnostics = self.diagnostics();
        if diagnostics.is_empty() {
            None
        } else {
            Some(Box::new(DiagnosticIter {
                diagnostics,
                index: 0,
            }))
        }
    }

    fn diagnostic_source(&self) -> Option<&dyn ::miette::Diagnostic> {
        None
    }
}

#[cfg(feature = "miette")]
struct DiagnosticIter<'a> {
    diagnostics: &'a [TraceDiagnostic],
    index: usize,
}

#[cfg(feature = "miette")]
impl<'a> Iterator for DiagnosticIter<'a> {
    type Item = &'a dyn ::miette::Diagnostic;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.diagnostics.len() {
            let diag = &self.diagnostics[self.index];
            self.index += 1;
            Some(diag as &dyn ::miette::Diagnostic)
        } else {
            None
        }
    }
}

/// Result is a type alias for a Result type that uses the Error type.
pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::from_std_error(&err)
    }
}
