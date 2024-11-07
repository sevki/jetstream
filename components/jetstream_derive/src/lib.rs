#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![recursion_limit = "256"]

extern crate proc_macro;
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod service;
mod wireformat;

/// Derives wire format encoding for structs
#[proc_macro_derive(JetStreamWireFormat)]
pub fn jetstream_wire_format(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    wireformat::wire_format_inner(input).into()
}

/// Service attribute macro for creating RPC services
#[proc_macro_attribute]
pub fn service(_: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemTrait);
    service::service_impl(item).into()
}
