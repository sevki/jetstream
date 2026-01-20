use miette::{LabeledSpan, Severity, SourceOffset, SourceSpan};

use crate::WireFormat;
/*pub struct MietteDiagnostic {
    /// Displayed diagnostic message
    pub message: String,
    /// Unique diagnostic code to look up more information
    /// about this Diagnostic. Ideally also globally unique, and documented
    /// in the toplevel crate's documentation for easy searching.
    /// Rust path format (`foo::bar::baz`) is recommended, but more classic
    /// codes like `E0123` will work just fine
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub code: Option<String>,
    /// [`Diagnostic`] severity. Intended to be used by
    /// [`ReportHandler`](crate::ReportHandler)s to change the way different
    /// [`Diagnostic`]s are displayed. Defaults to [`Severity::Error`]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub severity: Option<Severity>,
    /// Additional help text related to this Diagnostic
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub help: Option<String>,
    /// URL to visit for a more detailed explanation/help about this
    /// [`Diagnostic`].
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub url: Option<String>,
    /// Labels to apply to this `Diagnostic`'s [`Diagnostic::source_code`]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub labels: Option<Vec<LabeledSpan>>,
} */

/// r[impl jetstream.miette.wireformat.miette_diagnostic]
/// MietteDiagnostic is encoded as:
/// - message: String
/// - code: Option<String>
/// - severity: Option<Severity>
/// - help: Option<String>
/// - url: Option<String>
/// - labels: Option<Vec<LabeledSpan>>
impl WireFormat for miette::MietteDiagnostic {
    fn byte_size(&self) -> u32 {
        self.message.byte_size()
            + self.code.byte_size()
            + self.severity.byte_size()
            + self.help.byte_size()
            + self.url.byte_size()
            + self.labels.byte_size()
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>
    where
        Self: Sized,
    {
        self.message.encode(writer)?;
        self.code.encode(writer)?;
        self.severity.encode(writer)?;
        self.help.encode(writer)?;
        self.url.encode(writer)?;
        self.labels.encode(writer)
    }

    fn decode<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let message: String = WireFormat::decode(reader)?;
        let code: Option<String> = WireFormat::decode(reader)?;
        let severity: Option<Severity> = WireFormat::decode(reader)?;
        let help: Option<String> = WireFormat::decode(reader)?;
        let url: Option<String> = WireFormat::decode(reader)?;
        let labels: Option<Vec<LabeledSpan>> = WireFormat::decode(reader)?;

        let mut diagnostic = miette::MietteDiagnostic::new(message);
        if let Some(code) = code {
            diagnostic = diagnostic.with_code(code);
        }
        if let Some(severity) = severity {
            diagnostic = diagnostic.with_severity(severity);
        }
        if let Some(help) = help {
            diagnostic = diagnostic.with_help(help);
        }
        if let Some(url) = url {
            diagnostic = diagnostic.with_url(url);
        }
        if let Some(labels) = labels {
            diagnostic = diagnostic.with_labels(labels);
        }

        Ok(diagnostic)
    }
}

/// r[impl jetstream.miette.wireformat.severity]
/// Severity is encoded as a single byte:
/// - 0: Advice
/// - 1: Warning
/// - 2: Error
impl WireFormat for Severity {
    fn byte_size(&self) -> u32 {
        1
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>
    where
        Self: Sized,
    {
        let byte = match self {
            Severity::Advice => 0u8,
            Severity::Warning => 1u8,
            Severity::Error => 2u8,
        };
        byte.encode(writer)
    }

    fn decode<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let byte = u8::decode(reader)?;
        match byte {
            0 => Ok(Severity::Advice),
            1 => Ok(Severity::Warning),
            2 => Ok(Severity::Error),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid Severity value: {}", byte),
            )),
        }
    }
}
/*pub struct LabeledSpan {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    label: Option<String>,
    span: SourceSpan,
    primary: bool,
} */

/// r[impl jetstream.miette.wireformat.labeled_span]
/// LabeledSpan is encoded as:
/// - label: Option<String>
/// - span: SourceSpan (offset + length as usize)
/// - primary: bool
impl WireFormat for miette::LabeledSpan {
    fn byte_size(&self) -> u32 {
        self.label().map(|s| s.to_string()).byte_size()
            + self.inner().byte_size()
            + self.primary().byte_size()
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>
    where
        Self: Sized,
    {
        self.label().map(|s| s.to_string()).encode(writer)?;
        self.inner().encode(writer)?;
        self.primary().encode(writer)
    }

    fn decode<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let label: Option<String> = WireFormat::decode(reader)?;
        let span: SourceSpan = WireFormat::decode(reader)?;
        let primary: bool = WireFormat::decode(reader)?;

        if primary {
            Ok(LabeledSpan::new_primary_with_span(label, span))
        } else {
            Ok(LabeledSpan::new_with_span(label, span))
        }
    }
}

/*pub struct SourceSpan {
    /// The start of the span.
    offset: SourceOffset,
    /// The total length of the span
    length: usize,
} */

/// r[impl jetstream.miette.wireformat.source_span]
/// SourceSpan is encoded as offset (usize) followed by length (usize).
impl WireFormat for miette::SourceSpan {
    fn byte_size(&self) -> u32 {
        (std::mem::size_of::<usize>() * 2) as u32
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>
    where
        Self: Sized,
    {
        self.offset().encode(writer)?;
        self.len().encode(writer)
    }

    fn decode<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let offset = usize::decode(reader)?;
        let length = usize::decode(reader)?;
        Ok(SourceSpan::new(SourceOffset::from(offset), length))
    }
}

/*
/**
"Raw" type for the byte offset from the beginning of a [`SourceCode`].
*/
pub type ByteOffset = usize;

/**
Newtype that represents the [`ByteOffset`] from the beginning of a [`SourceCode`]
*/
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SourceOffset(ByteOffset); */
/// r[impl jetstream.miette.wireformat.source_offset]
impl WireFormat for miette::SourceOffset {
    fn byte_size(&self) -> u32 {
        std::mem::size_of::<usize>() as u32
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>
    where
        Self: Sized,
    {
        self.offset().encode(writer)
    }

    fn decode<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let offset = usize::decode(reader)?;
        Ok(SourceOffset::from(offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Helper function to encode a value and return the bytes
    fn encode_to_bytes<T: WireFormat>(value: &T) -> Vec<u8> {
        let mut buffer = Vec::new();
        value.encode(&mut buffer).expect("encoding failed");
        buffer
    }

    /// Helper function to decode a value from bytes
    fn decode_from_bytes<T: WireFormat>(bytes: &[u8]) -> T {
        let mut cursor = Cursor::new(bytes);
        T::decode(&mut cursor).expect("decoding failed")
    }

    /// Helper function to round-trip encode/decode a value
    fn round_trip<T: WireFormat>(value: &T) -> T {
        let bytes = encode_to_bytes(value);
        decode_from_bytes(&bytes)
    }

    /// r[verify jetstream.miette.wireformat.severity]
    #[test]
    fn test_severity_advice() {
        let severity = Severity::Advice;
        let bytes = encode_to_bytes(&severity);
        assert_eq!(bytes, vec![0]);
        let decoded: Severity = decode_from_bytes(&bytes);
        assert_eq!(decoded, Severity::Advice);
    }

    /// r[verify jetstream.miette.wireformat.severity]
    #[test]
    fn test_severity_warning() {
        let severity = Severity::Warning;
        let bytes = encode_to_bytes(&severity);
        assert_eq!(bytes, vec![1]);
        let decoded: Severity = decode_from_bytes(&bytes);
        assert_eq!(decoded, Severity::Warning);
    }

    /// r[verify jetstream.miette.wireformat.severity]
    #[test]
    fn test_severity_error() {
        let severity = Severity::Error;
        let bytes = encode_to_bytes(&severity);
        assert_eq!(bytes, vec![2]);
        let decoded: Severity = decode_from_bytes(&bytes);
        assert_eq!(decoded, Severity::Error);
    }

    /// r[verify jetstream.miette.wireformat.severity]
    #[test]
    fn test_severity_invalid() {
        let bytes = vec![3u8]; // Invalid severity value
        let mut cursor = Cursor::new(&bytes);
        let result = Severity::decode(&mut cursor);
        assert!(result.is_err());
    }

    /// r[verify jetstream.miette.wireformat.severity]
    #[test]
    fn test_severity_byte_size() {
        assert_eq!(Severity::Advice.byte_size(), 1);
        assert_eq!(Severity::Warning.byte_size(), 1);
        assert_eq!(Severity::Error.byte_size(), 1);
    }

    /// r[verify jetstream.miette.wireformat.source_offset]
    #[test]
    fn test_source_offset_round_trip() {
        let offset = SourceOffset::from(42usize);
        let decoded = round_trip(&offset);
        assert_eq!(decoded.offset(), 42);
    }

    /// r[verify jetstream.miette.wireformat.source_offset]
    #[test]
    fn test_source_offset_zero() {
        let offset = SourceOffset::from(0usize);
        let decoded = round_trip(&offset);
        assert_eq!(decoded.offset(), 0);
    }

    /// r[verify jetstream.miette.wireformat.source_offset]
    #[test]
    fn test_source_offset_large() {
        let offset = SourceOffset::from(1_000_000usize);
        let decoded = round_trip(&offset);
        assert_eq!(decoded.offset(), 1_000_000);
    }

    /// r[verify jetstream.miette.wireformat.source_offset]
    #[test]
    fn test_source_offset_byte_size() {
        let offset = SourceOffset::from(42usize);
        assert_eq!(offset.byte_size(), std::mem::size_of::<usize>() as u32);
    }

    /// r[verify jetstream.miette.wireformat.source_span]
    #[test]
    fn test_source_span_round_trip() {
        let span = SourceSpan::new(SourceOffset::from(10usize), 20);
        let decoded = round_trip(&span);
        assert_eq!(decoded.offset(), 10);
        assert_eq!(decoded.len(), 20);
    }

    /// r[verify jetstream.miette.wireformat.source_span]
    #[test]
    fn test_source_span_zero() {
        let span = SourceSpan::new(SourceOffset::from(0usize), 0);
        let decoded = round_trip(&span);
        assert_eq!(decoded.offset(), 0);
        assert_eq!(decoded.len(), 0);
    }

    /// r[verify jetstream.miette.wireformat.source_span]
    #[test]
    fn test_source_span_byte_size() {
        let span = SourceSpan::new(SourceOffset::from(10usize), 20);
        assert_eq!(span.byte_size(), (std::mem::size_of::<usize>() * 2) as u32);
    }

    /// r[verify jetstream.miette.wireformat.labeled_span]
    #[test]
    fn test_labeled_span_with_label() {
        let span = LabeledSpan::new_with_span(
            Some("test label".to_string()),
            (10, 20),
        );
        let decoded = round_trip(&span);
        assert_eq!(decoded.label(), Some("test label"));
        assert_eq!(decoded.offset(), 10);
        assert_eq!(decoded.len(), 20);
        assert!(!decoded.primary());
    }

    /// r[verify jetstream.miette.wireformat.labeled_span]
    #[test]
    fn test_labeled_span_without_label() {
        let span = LabeledSpan::new_with_span(None, (5, 15));
        let decoded = round_trip(&span);
        assert_eq!(decoded.label(), None);
        assert_eq!(decoded.offset(), 5);
        assert_eq!(decoded.len(), 15);
        assert!(!decoded.primary());
    }

    /// r[verify jetstream.miette.wireformat.labeled_span]
    #[test]
    fn test_labeled_span_primary() {
        let span = LabeledSpan::new_primary_with_span(
            Some("primary label".to_string()),
            (0, 100),
        );
        let decoded = round_trip(&span);
        assert_eq!(decoded.label(), Some("primary label"));
        assert_eq!(decoded.offset(), 0);
        assert_eq!(decoded.len(), 100);
        assert!(decoded.primary());
    }

    /// r[verify jetstream.miette.wireformat.miette_diagnostic]
    #[test]
    fn test_miette_diagnostic_simple() {
        let diagnostic = miette::MietteDiagnostic::new("Test error message");
        let decoded = round_trip(&diagnostic);
        assert_eq!(decoded.message, "Test error message");
        assert_eq!(decoded.code, None);
        assert_eq!(decoded.severity, None);
        assert_eq!(decoded.help, None);
        assert_eq!(decoded.url, None);
        assert_eq!(decoded.labels, None);
    }

    /// r[verify jetstream.miette.wireformat.miette_diagnostic]
    #[test]
    fn test_miette_diagnostic_with_code() {
        let diagnostic =
            miette::MietteDiagnostic::new("Error").with_code("E0001");
        let decoded = round_trip(&diagnostic);
        assert_eq!(decoded.message, "Error");
        assert_eq!(decoded.code, Some("E0001".to_string()));
    }

    /// r[verify jetstream.miette.wireformat.miette_diagnostic]
    #[test]
    fn test_miette_diagnostic_with_severity() {
        let diagnostic = miette::MietteDiagnostic::new("Warning message")
            .with_severity(Severity::Warning);
        let decoded = round_trip(&diagnostic);
        assert_eq!(decoded.message, "Warning message");
        assert_eq!(decoded.severity, Some(Severity::Warning));
    }

    /// r[verify jetstream.miette.wireformat.miette_diagnostic]
    #[test]
    fn test_miette_diagnostic_with_help() {
        let diagnostic = miette::MietteDiagnostic::new("Error")
            .with_help("Try this instead");
        let decoded = round_trip(&diagnostic);
        assert_eq!(decoded.help, Some("Try this instead".to_string()));
    }

    /// r[verify jetstream.miette.wireformat.miette_diagnostic]
    #[test]
    fn test_miette_diagnostic_with_url() {
        let diagnostic = miette::MietteDiagnostic::new("Error")
            .with_url("https://example.com/help");
        let decoded = round_trip(&diagnostic);
        assert_eq!(decoded.url, Some("https://example.com/help".to_string()));
    }

    /// r[verify jetstream.miette.wireformat.miette_diagnostic]
    #[test]
    fn test_miette_diagnostic_with_labels() {
        let labels = vec![
            LabeledSpan::new_with_span(Some("first".to_string()), (0, 10)),
            LabeledSpan::new_primary_with_span(
                Some("second".to_string()),
                (20, 30),
            ),
        ];
        let diagnostic = miette::MietteDiagnostic::new("Error with labels")
            .with_labels(labels);
        let decoded = round_trip(&diagnostic);

        assert_eq!(decoded.message, "Error with labels");
        let decoded_labels = decoded.labels.expect("labels should be present");
        assert_eq!(decoded_labels.len(), 2);
        assert_eq!(decoded_labels[0].label(), Some("first"));
        assert_eq!(decoded_labels[0].offset(), 0);
        assert_eq!(decoded_labels[0].len(), 10);
        assert!(!decoded_labels[0].primary());
        assert_eq!(decoded_labels[1].label(), Some("second"));
        assert_eq!(decoded_labels[1].offset(), 20);
        assert_eq!(decoded_labels[1].len(), 30);
        assert!(decoded_labels[1].primary());
    }

    /// r[verify jetstream.miette.wireformat.miette_diagnostic]
    #[test]
    fn test_miette_diagnostic_full() {
        let labels = vec![LabeledSpan::at(0..10, "here")];
        let diagnostic = miette::MietteDiagnostic::new("Full diagnostic")
            .with_code("TEST::E001")
            .with_severity(Severity::Error)
            .with_help("Check the documentation")
            .with_url("https://docs.example.com")
            .with_labels(labels);

        let decoded = round_trip(&diagnostic);

        assert_eq!(decoded.message, "Full diagnostic");
        assert_eq!(decoded.code, Some("TEST::E001".to_string()));
        assert_eq!(decoded.severity, Some(Severity::Error));
        assert_eq!(decoded.help, Some("Check the documentation".to_string()));
        assert_eq!(decoded.url, Some("https://docs.example.com".to_string()));
        assert!(decoded.labels.is_some());
        let labels = decoded.labels.unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].label(), Some("here"));
    }

    /// r[verify jetstream.miette.wireformat.severity]
    #[test]
    fn test_option_severity_none() {
        let opt: Option<Severity> = None;
        let decoded: Option<Severity> = round_trip(&opt);
        assert_eq!(decoded, None);
    }

    /// r[verify jetstream.miette.wireformat.severity]
    #[test]
    fn test_option_severity_some() {
        let opt: Option<Severity> = Some(Severity::Warning);
        let decoded: Option<Severity> = round_trip(&opt);
        assert_eq!(decoded, Some(Severity::Warning));
    }

    /// r[verify jetstream.miette.wireformat.labeled_span]
    #[test]
    fn test_vec_labeled_span() {
        let spans = vec![
            LabeledSpan::at(0..5, "first"),
            LabeledSpan::at(10..15, "second"),
            LabeledSpan::at(20..25, "third"),
        ];
        let decoded: Vec<LabeledSpan> = round_trip(&spans);
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].label(), Some("first"));
        assert_eq!(decoded[1].label(), Some("second"));
        assert_eq!(decoded[2].label(), Some("third"));
    }
}
