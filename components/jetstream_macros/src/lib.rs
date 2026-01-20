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
#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let is_async_trait =
        !attr.is_empty() && attr.to_string().contains("async_trait");
    let enable_tracing = attr.to_string().contains("tracing");
    let item = parse_macro_input!(item as syn::ItemTrait);

    service::service_impl(item, is_async_trait, enable_tracing).into()
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
