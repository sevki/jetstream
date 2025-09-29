mod client;
mod frame;
mod server;
mod protocol;
mod message;
#[cfg(test)]
mod tests;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{ItemTrait, TraitItem};

pub(crate) fn service_impl(
    item: ItemTrait,
    is_async_trait: bool,
) -> TokenStream {
    let trait_name = &item.ident;
    let trait_items = &item.items;
    let vis = &item.vis;

    // Generate protocol metadata
    let service_name = format_ident!("{}Service", trait_name);
    let channel_name = format_ident!("{}Channel", trait_name);
    let digest = sha256::digest(item.to_token_stream().to_string());
    
    #[allow(clippy::to_string_in_format_args)]
    let protocol_version = format!(
        "dev.branch.jetstream.proto/{}/{}.{}.{}-{}",
        trait_name.to_string().to_lowercase(),
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH"),
        digest[0..8].to_string()
    );
    let protocol_version = Literal::string(protocol_version.as_str());
    
    let tag_name = format_ident!("{}_TAG", trait_name.to_string().to_uppercase());
    
    // Generate message structs and collect metadata
    let mut tmsgs = Vec::new();
    let mut rmsgs = Vec::new();
    let mut msg_ids = Vec::new();
    
    for (index, item) in item.items.iter().enumerate() {
        if let TraitItem::Fn(method) = item {
            let method_name = &method.sig.ident;
            
            let request_struct_ident = message::request_struct_name(method_name);
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
        &protocol_version,
    );
    
    // Generate client implementation  
    let client_impl = client::generate_client(
        &channel_name,
        trait_name,
        &item.items,
        &tmsgs,
        &rmsgs,
        &protocol_version,
        &tag_name,
    );
    
    // Generate final trait with attribute
    let trait_attribute = if is_async_trait {
        quote! { #[jetstream::prelude::async_trait] }
    } else {
        quote! { #[jetstream::prelude::make(Send + Sync)] }
    };
    
    let proto_mod = format_ident!("{}_protocol", trait_name.to_string().to_lowercase());
    let digest_lit = Literal::string(digest.as_str());
    
    // Generate message definitions
    let tmsg_definitions = tmsgs.iter().map(|(_ident, def)| quote! { #def });
    let rmsg_definitions = rmsgs.iter().map(|(_ident, def)| quote! { #def });
    
    quote! {
        #vis mod #proto_mod {
            use jetstream::prelude::*;
            use std::io::{self, Read, Write};
            use std::mem;
            use super::#trait_name;
            
            const MESSAGE_ID_START: u8 = 101;
            pub const PROTOCOL_VERSION: &str = #protocol_version;
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