#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Derive
//! This crate provides macros for JetStream.
//!
//! ## `service`
//! The `service` macro is used to define a JetStream service.
//!
//! ### Basic Usage
//! ```ignore
//! #[service]
//! pub trait Echo {
//!     async fn ping(&mut self, message: String) -> Result<String, Error>;
//! }
//! ```
//!
//! ### Tracing Support
//!
//! Add distributed tracing to your services using the `tracing` crate:
//!
//! ```ignore
//! #[service(tracing)]  // Auto-instrument all methods
//! pub trait Echo {
//!     // Custom instrumentation
//!     #[instrument(skip(self), fields(msg_len = message.len()))]
//!     async fn ping(&mut self, message: String) -> Result<String, Error>;
//! }
//! ```
//!
//! See the [Tracing Guide](../../docs/tracing.md) for detailed documentation on tracing support.
//!
//! ### Versioned Services
//!
//! A `#[service]` trait can describe the *history* of an API instead of a
//! single shape of it. Give every method a `#[since("…")]` saying which
//! version introduced it, and redeclare a method under a newer version to
//! replace it:
//!
//! ```ignore
//! #[service]
//! pub trait Echo {
//!     #[since("0.1.0")]
//!     async fn ping(&self) -> Result<()>;
//!     #[since("0.1.0")]
//!     async fn pong(&self) -> Result<()>;
//!     #[since("1.0.0")]
//!     async fn ping(&self, msg: String) -> Result<String>;
//!     #[since("1.0.0")]
//!     async fn shout(&self, msg: String) -> Result<String>;
//! }
//! ```
//!
//! Such a trait is a DSL, not Rust — two `ping` declarations cannot coexist
//! in one trait — so it is never emitted as written. Each distinct `since`
//! becomes a *snapshot* trait named `<Service>V<major>_<minor>_<patch>`
//! (`EchoV0_1_0`, `EchoV1_0_0`), and every snapshot goes through the same
//! expansion an unversioned service does: protocol module, messages, frames,
//! client, server, dispatch.
//!
//! - A declaration is active at version `V` when `since <= V`, and, if an
//!   optional `#[until("…")]` is given, `V < until` (exclusive). Among the
//!   active declarations of one name the greatest `since` wins, so source
//!   order never decides anything.
//! - Wire ids stay `102 + 2 * slot`, where a method's slot is fixed by where
//!   it was *introduced*. Adding a later declaration therefore cannot
//!   renumber a method that already existed.
//! - Snapshots are separate protocols: each gets its own `PROTOCOL_NAME`
//!   (`echov0_1_0`) and `PROTOCOL_VERSION`, and `Tversion` negotiation
//!   already rejects a mismatched name, so one version cannot decode
//!   another's payloads. Choosing which version to speak stays with whoever
//!   opens the connection; the macro only gives the versions distinct
//!   identities.
//! - One type may implement several snapshots. Same-named methods then need
//!   fully qualified calls: `EchoV1_0_0::ping(&echo, msg)`.
//!
//! A trait with no `#[since]` anywhere is untouched by any of this and
//! expands exactly as it always has.
//!
//! ## `JetStreamWireFormat`
//! The `JetStreamWireFormat` macro is used to derive the `WireFormat` trait for a struct.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![recursion_limit = "256"]

extern crate proc_macro;
use proc_macro::TokenStream;
use syn::parse_macro_input;
mod error;

mod service;
#[cfg(test)]
mod tests;
mod utils;
mod wireformat;

/// Derives wire format encoding for structs
#[proc_macro_derive(JetStreamWireFormat, attributes(jetstream))]
pub fn jetstream_wire_format(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    wireformat::wire_format_inner(input).into()
}

/// Service attribute macro for creating RPC services
///
/// ## Attributes
///
/// - `async_trait` - Use async_trait instead of the default make(Send + Sync)
/// - `tracing` - Enable auto-instrumentation for all methods
/// - `uses(path::to::mod::*)` - Add use statements to the generated protocol module.
///   Multiple paths can be specified: `uses(some::mod::*, other::mod::Type)`
///
/// ## Method attributes
///
/// - `#[since("1.2.3")]` - the version that introduced this declaration.
///   Using it anywhere in the trait switches on versioned expansion: one
///   snapshot trait per distinct `since`, named `<Service>V1_2_3`.
/// - `#[until("2.0.0")]` - optional, exclusive: retires the declaration at
///   that version. Must be greater than `since`.
///
/// Versions are quoted three-part semantic versions; prerelease and build
/// metadata are rejected because they have no unambiguous spelling as a Rust
/// identifier. See the crate documentation for the full model.
///
/// ## Example
///
/// ```ignore
/// #[service(uses(some::mod::*, other::mod::Type))]
/// pub trait Backend {
///     async fn read_commit(&mut self, id: CommitId) -> Result<Commit>;
/// }
/// ```
#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = service::parse_service_attr(attr.into());
    let item = parse_macro_input!(item as syn::ItemTrait);

    service::service_impl(item, attr).into()
}

/// Error macro for creating rich Jetstream errors
///
/// # Usage
/// ```ignore
/// err!(message: "simple error")
///
/// err!(
///     code: "jetstream::rpc::timeout",
///     severity: Error,
///     help: "increase timeout value",
///     message: "request timed out after {}ms", timeout_ms
/// )
/// ```
#[proc_macro]
pub fn err(input: TokenStream) -> TokenStream {
    let error_macro: error::ErrorMacro = input.into();
    error_macro.into()
}
