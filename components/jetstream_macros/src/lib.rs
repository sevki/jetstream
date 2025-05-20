#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
//! # JetStream Derive
//! This crate provides macros for JetStream.
//! ## `service`
//! The `service` macro is used to define a JetStream service.
//!
//! ## `JetStreamWireFormat`
//! The `JetStreamWireFormat` macro is used to derive the `WireFormat` trait for a struct.
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![recursion_limit = "256"]

extern crate proc_macro;
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod service;
mod wireformat;
#[cfg(test)]
mod tests;

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
    let item = parse_macro_input!(item as syn::ItemTrait);
    service::service_impl(item, is_async_trait).into()
}
