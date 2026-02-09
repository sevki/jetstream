# Error Handling and Propagation

## Error Type

r[jetstream.error.v2.type]
`jetstream::Error` MUST be a struct containing a boxed `ErrorInner` along with lazily-initialized backtrace and diagnostic state. The design is modeled after [`TracedError`](https://docs.rs/tracing-error/latest/tracing_error/struct.TracedError.html) from the `tracing-error` crate: an error wrapper that bundles diagnostic information with a [`SpanTrace`](https://docs.rs/tracing-error/latest/tracing_error/struct.SpanTrace.html) captured at the point of creation. The `Box` indirection on `ErrorInner` keeps `Error` efficient for passing through `Result`. The struct MUST be `Clone`.

r[jetstream.error.v2.result-size]
`jetstream::Error` MUST NOT trigger [`clippy::result_large_err`](https://rust-lang.github.io/rust-clippy/master/index.html#result_large_err). The overall size of `Error` MUST be kept small enough that returning `Result<T, Error>` does not incur unnecessary stack copies. This is achieved by boxing the `ErrorInner` and using indirection for large fields.

r[jetstream.error.v2.inner]
`ErrorInner` MUST be a `#[derive(JetStreamWireFormat)]` struct containing:
- `message: String` — the human-readable error message.
- `code: Option<String>` — an optional machine-readable error code (e.g. `"server::validation::E001"`).
- `help: Option<String>` — optional help text with remediation guidance.
- `url: Option<String>` — optional URL for further documentation.

The `Error` struct itself MUST additionally contain:
- `span_trace: Option<Box<SpanTrace>>` — the captured tracing span trace, present when the error was constructed locally, `None` when decoded from the wire.
- `backtrace: OnceLock<Box<Backtrace>>` — lazily computed structured backtrace derived from the span trace (for local errors) or decoded from the wire (for remote errors).
- `diagnostics: OnceLock<Vec<TraceDiagnostic>>` — lazily computed miette diagnostic entries derived from the backtrace.

The `message`, `code`, `help`, `url`, and the structured `Backtrace` are the fields that travel over the wire. Severity is not stored directly — it is derived from the tracing `Level` of the backtrace frames.

r[jetstream.error.v2.into-error]
`jetstream_error` MUST define an `IntoError` trait with an `into_error(self) -> Error` method, and provide a blanket implementation for all `T: std::error::Error + Send + Sync`. This trait MUST be re-exported by `jetstream_rpc` and used as the bound for the `Protocol::Error` associated type, enabling the `#[service]` macro to convert arbitrary error types into `jetstream::Error`.

r[jetstream.error.v2.span-trace]
`jetstream::Error` MUST carry an `Option<Box<SpanTrace>>` that provides an async-aware "logical backtrace" of the active `tracing` spans when the error was created locally. When the error is decoded from the wire, the span trace is `None` and the backtrace is populated directly from the decoded wire data.

r[jetstream.error.v2.span-trace.capture]
When a `jetstream::Error` is constructed locally (via `Error::new`, `Error::with_code`, `Error::from_std_error`, or `From` conversions), `SpanTrace::capture()` MUST be called automatically to record the current span context.

r[jetstream.error.v2.span-trace.format]
The captured `SpanTrace` MUST be converted to a structured `Backtrace` type (see `jetstream.error.v2.backtrace`) for both wire encoding and diagnostic rendering. The conversion from `SpanTrace` to `Backtrace` MUST extract each span's metadata (name, target, module, file, line, level) and field values, using a string interning table for deduplication. The `Backtrace` is computed lazily via `OnceLock` and cached for the lifetime of the `Error`.

r[jetstream.error.v2.std-error]
`jetstream::Error` MUST implement `std::error::Error`, `Display`, `Debug`, and `Clone`. The `Display` implementation MUST format as `[{code}] {message}` when a code is present, or just `{message}` otherwise. The `Debug` implementation MUST show the `message`, `code`, `help`, and `url` fields.

r[jetstream.error.v2.builders]
`jetstream::Error` MUST provide builder methods `set_code`, `set_help`, and `set_url` that return `Self` for chaining. Accessor methods `message()`, `code()`, `help_text()`, and `url()` MUST be provided. Convenience constructors `Error::invalid_response(message)` MUST also be available.

## Backtrace

r[jetstream.error.v2.backtrace]
The `Backtrace` type MUST be a `#[derive(JetStreamWireFormat)]` struct containing:
- `intern_table: Vec<String>` — a deduplicated string interning table.
- `frames: Vec<Frame>` — the ordered span entries from innermost to outermost.

The `Backtrace` MUST implement `Default` (empty frames) and `Clone`.

r[jetstream.error.v2.backtrace.frame]
Each `Frame` MUST be a `#[derive(JetStreamWireFormat)]` struct containing:
- `msg: String` — the span name.
- `name: u16` — index into the intern table for the span name.
- `target: u16` — index into the intern table for the target.
- `module: u16` — index into the intern table for the module path.
- `file: u16` — index into the intern table for the source file path.
- `line: u16` — the source line number.
- `fields: Vec<FieldPair>` — the span's recorded field values.
- `level: Level` — the tracing verbosity level, encoded via a custom `LevelCodec`.

r[jetstream.error.v2.backtrace.field-pair]
`FieldPair` MUST be a `#[derive(JetStreamWireFormat)]` struct with `key: u16` and `value: u16`, both indices into the intern table.

r[jetstream.error.v2.backtrace.from-spantrace]
A `backtrace_from_spantrace` function MUST convert a `&SpanTrace` into a `Box<Backtrace>` by iterating over the span trace's spans via `SpanTrace::with_spans`, extracting metadata and field values, interning all strings, and building the `Frame` list. If the span trace status is not `CAPTURED`, an empty default `Backtrace` MUST be returned.

r[jetstream.error.v2.backtrace.level-codec]
The `tracing::Level` type MUST be encoded as a single `u8` via a custom `LevelCodec` (used with `#[jetstream(with(...))]`): TRACE=0, DEBUG=1, INFO=2, WARN=3, ERROR=4.

## Error Reporting

r[jetstream.error.v2.reporting.miette-feature]
The `miette` dependency MUST be optional, gated behind a `miette` cargo feature flag that is enabled by default. When the `miette` feature is enabled, `jetstream::Error` MUST implement `miette::Diagnostic`. When the `miette` feature is disabled, the `miette` crate MUST NOT be compiled, and `jetstream::Error` MUST NOT implement `miette::Diagnostic`. All miette-related types (`DiagnosticIter`, miette-specific fields on `TraceDiagnostic`, the `Severity` bridge enum) MUST be gated behind `#[cfg(feature = "miette")]`.

r[jetstream.error.v2.reporting.plain-text]
When the `miette` feature is disabled, `jetstream::Error`'s `Display` implementation MUST render the span trace as plain text appended after the error message. Each backtrace frame MUST be printed with the span name, source file and line (when available), and field values. The format MUST be:
```
[CODE] message

  in span_name
    at file/path.rs:42
    with field1: value1, field2: value2
```

r[jetstream.error.v2.reporting.miette]
When the `miette` feature is enabled, `jetstream::Error` MUST implement [`miette::Diagnostic`](https://docs.rs/miette/latest/miette/trait.Diagnostic.html) so that errors can be rendered with miette's fancy diagnostic output (colors, code frames, help text) when displayed to humans. The `Diagnostic` implementation MUST delegate `code()`, `help()`, and `url()` to the corresponding `ErrorInner` fields, and derive `severity()` from the backtrace frames' tracing levels.

r[jetstream.error.v2.reporting.span-trace-section]
When rendering a `jetstream::Error` through miette, each span in the backtrace MUST be rendered as a separate `TraceDiagnostic` entry via `Diagnostic::related()`. Each `TraceDiagnostic` MUST display the span name, and when source code is available, render a miette source snippet with a labeled span pointing to the originating file and line, annotated with the span name and field values (e.g. `validate_request(user_id: 42, email: "user@example.com")`).

r[jetstream.error.v2.reporting.related]
The `Diagnostic::related()` implementation MUST return an iterator over `TraceDiagnostic` entries, one per backtrace frame. Each `TraceDiagnostic` MUST implement `miette::Diagnostic` with:
- `severity()` derived from the frame's tracing `Level` (ERROR and WARN map to miette severities; DEBUG, TRACE, INFO return `None`).
- `source_code()` returning a `miette::NamedSource` when the source file can be read from disk.
- `labels()` returning a `LabeledSpan` pointing to the span's declaration line, with label text showing the span name and its field values.

r[jetstream.error.v2.reporting.source-resolution]
Source file resolution MUST attempt to read source files by walking up parent directories from the current working directory, since `file!()` paths from tracing metadata are workspace-relative. Up to 10 parent directory levels MUST be tried. When a source file cannot be found, the `TraceDiagnostic` MUST still be rendered but without a source code snippet.

## Source Information

r[jetstream.error.v2.source-info]
Source location information (file path, line number, module path) MUST be carried within each `Frame` in the `Backtrace`, referencing the intern table by index. The innermost frame's metadata provides the error's origin location.

r[jetstream.error.v2.source-info.client-render]
When rendering a `jetstream::Error` on the client side (via `miette::Diagnostic`), source information from the backtrace frames MUST be used to produce miette-style source code snippets. The diagnostic output MUST include the originating file and line from each frame, with labeled spans pointing to the function declaration, so the reader can locate the error origin in the codebase.

r[jetstream.error.v2.source-info.disable]
A cargo feature flag `source-info` MUST be defined and enabled by default. This flag MUST gate the `source-map` dependency and all source file reading logic (`read_source_file()`, `MapFileStore`, `LineColumnPosition` resolution). When the `source-info` feature is disabled (but `miette` is enabled), `TraceDiagnostic` entries MUST be produced with `source: None` so that miette renders diagnostics without source code snippets. When both `miette` and `source-info` are disabled, source file paths and line numbers from the backtrace frames MUST still be available for plain-text rendering via `Display`.

r[jetstream.error.v2.source-info.miette-integration]
When source information is available in the backtrace frames and the corresponding source files can be read from disk, the `miette::Diagnostic` implementation MUST render source code snippets via `Diagnostic::source_code()` on each `TraceDiagnostic`, with `labels()` pointing to the span's originating line. The label text MUST include the span name followed by its field values in parentheses.

## Tracing Subscriber Integration

r[jetstream.error.v2.error-layer]
Applications using jetstream error propagation MUST install [`tracing_error::ErrorLayer`](https://docs.rs/tracing-error/latest/tracing_error/struct.ErrorLayer.html) into their `tracing-subscriber` registry for `SpanTrace` capture to function.

## Wire Format

r[jetstream.error.v2.wireformat]
`jetstream::Error` MUST implement `WireFormat`. The implementation MUST encode the `ErrorInner` fields (message, code, help, url) followed by the `Backtrace`. On decode, the `ErrorInner` and `Backtrace` are read from the wire; the `span_trace` field is set to `None` (since the error originated remotely) and the `backtrace` `OnceLock` is pre-populated with the decoded value.

r[jetstream.error.v2.wireformat.message]
The wire encoding MUST include the error message as a `String`.

r[jetstream.error.v2.wireformat.code]
The wire encoding MUST include the error code as an `Option<String>`.

r[jetstream.error.v2.wireformat.backtrace]
The wire encoding MUST include the `Backtrace` struct, which is derived via `#[derive(JetStreamWireFormat)]`. The `Backtrace` encodes an intern table (`Vec<String>`) followed by a list of `Frame` structs, each containing metadata indices and field pairs.

r[jetstream.error.v2.wireformat.intern-table]
The `Backtrace` MUST use a string interning table: a deduplicated `Vec<String>` is written first, and each `Frame` field (name, target, module, file) and each `FieldPair` (key, value) references strings by `u16` index. This avoids transmitting the same file path or field name multiple times when several frames share common strings.

r[jetstream.error.v2.wireformat.frame]
Each `Frame` MUST encode: `msg` (span name as `String`), `name`/`target`/`module`/`file` (as `u16` intern table indices), `line` (as `u16`), `fields` (as `Vec<FieldPair>`), and `level` (as `u8` via `LevelCodec`).

r[jetstream.error.v2.wireformat.error-frame]
`jetstream_rpc` MUST define an `ErrorFrame` struct wrapping `jetstream_error::Error` that implements the `Framer` trait with a dedicated message type constant (`RJETSTREAMERROR`). The `ErrorFrame` MUST delegate encoding and decoding to `Error`'s `WireFormat` implementation. Bidirectional `From` conversions between `ErrorFrame` and `Error` MUST be provided.

## Macro Integration

r[jetstream.macro.error-type]
`#[service]` macro MUST generate `Protocol` implementations for both server and client that use `jetstream::prelude::Error` as the `Protocol::Error` associated type.

r[jetstream.error-message-frame]
`#[service]` macro MUST generate an `Error` variant in the response message enum (`Rmessage`) for serializing errors across requests. The error variant MUST use `jetstream::prelude::Error` as its payload and MUST be assigned the `RERROR` message type constant.

r[jetstream.macro.server-error]
When a jetstream server handler's inner implementation returns an error, the `#[service]` macro generated `rpc` method MUST convert the error to `Rmessage::Error(err)` so it is serialized as an error frame and returned to the client.

r[jetstream.macro.client-error]
When a jetstream client receives an `Rmessage::Error(err)` variant, the `#[service]` macro generated client method MUST return `Err(err)`, propagating the deserialized `jetstream::prelude::Error` to the caller.

r[jetstream.macro.source-span]
The `#[service]` macro MUST generate a protocol module containing message type definitions, constants, and the protocol version string. The module MUST re-export `jetstream::prelude::*` and define `RERROR` as `jetstream::prelude::RJETSTREAMERROR`.

r[jetstream.macro.tracing-instrument]
When the `#[service(tracing)]` attribute is used, the generated client methods MUST be annotated with `#[tracing::instrument]` (skipping `self`) so that the `SpanTrace` captured on error contains meaningful RPC method context (method name, request parameters).

## Error Propagation Testing

r[jetstream.test.error-propagation.e2e]
There MUST be an end-to-end test that verifies when a server returns an error, the client receives it with the message and code intact.

r[jetstream.test.error-propagation.roundtrip]
There MUST be tests that verify `jetstream::Error` (including its `Backtrace`) can be serialized and deserialized via `WireFormat` without data loss.

r[jetstream.test.error-propagation.conversions]
There MUST be tests that verify various error types (`std::io::Error`, custom error types) convert properly to `jetstream::Error` with a captured `SpanTrace`.

r[jetstream.test.error-propagation.span-trace-capture]
There MUST be a test that verifies `SpanTrace` is captured when creating errors inside `#[tracing::instrument]`-ed functions, and that the resulting miette diagnostic output contains the expected span names, field values, and source locations.

r[jetstream.test.error-propagation.span-trace-across-wire]
There MUST be an end-to-end test that verifies the server-side `Backtrace` (derived from the server's `SpanTrace`) is transmitted to the client and is readable in the received error's diagnostic output.
