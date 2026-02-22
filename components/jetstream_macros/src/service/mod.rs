mod client;
mod frame;
mod message;
mod server;
mod tests;
mod tests_tracing;
mod tracing;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};

use syn::{ItemTrait, TraitItem};

use crate::service::tracing::take_attributes;

mod kw {
    syn::custom_keyword!(uses);
    syn::custom_keyword!(tracing);
    syn::custom_keyword!(async_trait);
}

/// Parsed service attribute arguments
#[derive(Default)]
pub(crate) struct ServiceAttr {
    pub use_paths: Vec<syn::UseTree>,
    pub enable_tracing: bool,
    pub is_async_trait: bool,
}

impl syn::parse::Parse for ServiceAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attr = ServiceAttr::default();

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::uses) {
                input.parse::<kw::uses>()?;
                let content;
                syn::parenthesized!(content in input);
                let trees = content
                    .parse_terminated(syn::UseTree::parse, syn::Token![,])?;
                attr.use_paths.extend(trees);
            } else if lookahead.peek(kw::tracing) {
                input.parse::<kw::tracing>()?;
                attr.enable_tracing = true;
            } else if lookahead.peek(kw::async_trait) {
                input.parse::<kw::async_trait>()?;
                attr.is_async_trait = true;
            } else {
                return Err(lookahead.error());
            }
            // Trailing comma is optional
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(attr)
    }
}

/// Parses service attribute arguments
pub(crate) fn parse_service_attr(attr: TokenStream) -> ServiceAttr {
    if attr.is_empty() {
        return ServiceAttr::default();
    }

    syn::parse2::<ServiceAttr>(attr).unwrap_or_default()
}

pub(crate) fn service_impl(item: ItemTrait, attr: ServiceAttr) -> TokenStream {
    let ServiceAttr {
        use_paths,
        enable_tracing,
        is_async_trait,
    } = attr;
    let trait_name = &item.ident;
    let maps = take_attributes(
        item.items
            .iter()
            .flat_map(|i| match i {
                TraitItem::Fn(trait_item_fn) => Some(trait_item_fn.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let trait_items = maps.iter().map(|(item, _)| item).collect::<Vec<_>>();
    let vis = &item.vis;

    // Generate protocol metadata
    let service_name = format_ident!("{}Service", trait_name);
    let channel_name = format_ident!("{}Channel", trait_name);
    let digest = Literal::string(
        sha256::digest(item.to_token_stream().to_string()).as_str(),
    );

    // Generate message structs and collect metadata
    let mut tmsgs = Vec::new();
    let mut rmsgs = Vec::new();
    let mut msg_ids = Vec::new();
    let mut method_attrs = Vec::new();

    for (index, item) in item.items.iter().enumerate() {
        if let TraitItem::Fn(method) = item {
            let method_name = &method.sig.ident;

            let request_struct_ident =
                message::request_struct_name(method_name);
            let return_struct_ident = message::return_struct_name(method_name);

            let msg_id = message::generate_msg_id(index, method_name);
            msg_ids.push(msg_id);

            let request_struct = message::generate_input_struct(
                &request_struct_ident,
                &method.sig,
            );
            let return_struct = message::generate_return_struct(
                &return_struct_ident,
                &method.sig,
            );

            tmsgs.push((request_struct_ident, request_struct));
            rmsgs.push((return_struct_ident, return_struct));

            // Collect tracing attributes from method
            let attrs = tracing::extract_method_tracing_attrs(method);
            method_attrs.push(attrs);
        }
    }

    // Generate frame implementations
    let tmessage = frame::generate_tframe(&tmsgs);
    let rmessage = frame::generate_rframe(&rmsgs);

    // Generate server implementation
    let server_impl = server::generate_server(
        &service_name,
        trait_name,
        &item.items,
        &tmsgs,
        &rmsgs,
        &method_attrs,
        enable_tracing,
    );

    // Generate client implementation
    let client_impl = client::generate_client(
        &channel_name,
        trait_name,
        &item.items,
        &tmsgs,
        &method_attrs,
        enable_tracing,
    );

    // Generate final trait with attribute
    let trait_attribute = if is_async_trait {
        quote! { #[jetstream::prelude::async_trait] }
    } else {
        quote! { #[jetstream::prelude::make(Send + Sync)] }
    };

    let proto_mod =
        format_ident!("{}_protocol", trait_name.to_string().to_lowercase());
    let digest_lit = digest.clone();
    let trait_name_lower =
        Literal::string(&trait_name.to_string().to_lowercase());
    let digest_prefix = Literal::string(
        &sha256::digest(item.to_token_stream().to_string())[0..8],
    );

    // Generate message definitions
    let tmsg_definitions = tmsgs.iter().map(|(_ident, def)| quote! { #def });
    let rmsg_definitions = rmsgs.iter().map(|(_ident, def)| quote! { #def });

    // Generate additional use statements
    let additional_uses = use_paths.iter().map(|tree| quote! { use #tree; });

    // r[impl jetstream.macro.source-span]
    quote! {
        #vis mod #proto_mod {
            use jetstream::prelude::*;
            use std::mem;
            use super::#trait_name;
            #(#additional_uses)*

            const MESSAGE_ID_START: u8 = 102;
            /// Error response message type constant
            pub const RERROR: u8 = jetstream::prelude::RJETSTREAMERROR;
            /// Version request message type constant
            pub const TVERSION: u8 = jetstream::prelude::TVERSION;
            /// Version response message type constant
            pub const RVERSION: u8 = jetstream::prelude::RVERSION;
            /// Protocol name — used for routing
            pub const PROTOCOL_NAME: &str = #trait_name_lower;
            /// Protocol version string constructed from the generated crate's version
            pub const PROTOCOL_VERSION: &str = concat!(
                "rs.jetstream.proto/",
                #trait_name_lower,
                "/",
                env!("CARGO_PKG_VERSION_MAJOR"),
                ".",
                env!("CARGO_PKG_VERSION_MINOR"),
                ".",
                env!("CARGO_PKG_VERSION_PATCH"),
                "+",
                #digest_prefix
            );
            const DIGEST: &str = #digest_lit;

            #(#msg_ids)*

            #(#tmsg_definitions)*

            #(#rmsg_definitions)*

            #tmessage

            #rmessage

            #server_impl

            #client_impl
        }

        #trait_attribute
        #vis trait #trait_name {
            #(#trait_items)*
        }
    }
}
