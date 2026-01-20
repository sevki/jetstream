# Error Handling and Propagation

r[jetstream.error.type]
Jetstream rpc services MUST define it's error type to be `miette::Diagnostic`.

r[jetstream.miette.wireformat.miette_diagnostic]
jetstream_wireformat MUST implement WireFormat for [`miette::MietteDiagnostic`](https://docs.rs/miette/latest/miette/struct.MietteDiagnostic.html).

r[jetstream.miette.wireformat.severity]
jetstream_wireformat MUST implement WireFormat for [`miette::Severity`](https://docs.rs/miette/latest/miette/enum.Severity.html).

r[jetstream.miette.wireformat.labeled_span]
jetstream_wireformat MUST implement WireFormat for [`LabeledSpan`](https://docs.rs/miette/latest/miette/struct.LabeledSpan.html).

r[jetstream.miette.wireformat.source_span]
jetstream_wireformat MUST implement WireFormat for [`SourceSpan`](https://docs.rs/miette/latest/miette/struct.SourceSpan.html).

r[jetstream.miette.wireformat.source_offset]
jetstream_wireformat MUST implement WireFormat for [`SourceOffset`](https://docs.rs/miette/latest/miette/struct.SourceOffset.html).

r[jetstream.error.source_code.source_contents]
`SourceCode` MUST implement [`SpanContents`](https://docs.rs/miette/latest/miette/trait.SpanContents.html).

r[jetstream.macro.source_span]
jetstream_macros `#[service]` macro MUST provide source code for miette diagnostics.

r[jetstream.macro.error_type]
`#[service]` macro must generate calls for server and client that are `jetstream::prelude::Error`.

r[jetstream.error_message_frame]
`#[service]` macro must generate a error message type for serializing errors across requests.

r[jetstream.macro.client_error]
When jetstream client recieves an error, it must convert it to a `jetstream::prelude::Error` and return the, something like `Err(jetstream::prelude::Error::from(err_frame))`.

r[jetstream.macro.server_error]
When jetstream server inner returns an error it must be serialized and returned to the client as a Error frame.

## Error Propagation Testing

r[jetstream.test.error_propagation.e2e]
There MUST be an end-to-end test that verifies when a server returns an error, the client receives it with all diagnostic information intact (message, code, severity, help, url, labels).

r[jetstream.test.error_propagation.field_preservation]
There MUST be tests that verify all `MietteDiagnostic` fields (code, severity, help, url, labels) are preserved when errors are transmitted across the wire.

r[jetstream.test.error_propagation.roundtrip]
There MUST be tests that verify `jetstream::Error` can be serialized and deserialized without data loss.

r[jetstream.test.error_propagation.conversions]
There MUST be tests that verify various error types (`std::io::Error`, custom `Diagnostic` types) convert properly to `jetstream::Error` and propagate correctly.
