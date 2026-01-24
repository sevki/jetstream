use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, TraitItem};

use crate::utils::case_conversion::IdentCased;
#[allow(clippy::too_many_arguments)]
pub fn generate_client(
    channel_name: &Ident,
    trait_name: &Ident,
    trait_items: &[TraitItem],
    tmsgs: &[(Ident, TokenStream)],
    method_attrs: &[Vec<Attribute>],
    enable_tracing: bool,
) -> TokenStream {
    let client_calls =
        generate_client_calls(trait_items, tmsgs, method_attrs, enable_tracing);

    // Add RPC-level tracing span if tracing is enabled
    let _rpc_span = if enable_tracing {
        quote! {
            let _span = tracing::debug_span!(
                "rpc_client",
                service = stringify!(#trait_name),
                tag = frame.tag
            );
            let _enter = _span.enter();
        }
    } else {
        quote! {}
    };

    quote! {
        pub struct #channel_name {
            mux: Mux<Self>,
        }

        impl #channel_name {
            pub fn new(max_concurrent_requests:u16,inner: Box<dyn ClientTransport<Self>>) -> Self {
                Self { mux: Mux::new(max_concurrent_requests,inner) }
            }
        }

        impl Protocol for #channel_name {
            type Request = Tmessage;
            type Response = Rmessage;
            // r[impl jetstream.macro.error_type]
            type Error = Error;
            const VERSION: &'static str = PROTOCOL_VERSION;
        }

        impl #trait_name for #channel_name
        {
            #(#client_calls)*
        }
    }
}

fn generate_client_calls(
    trait_items: &[TraitItem],
    tmsgs: &[(Ident, TokenStream)],
    method_attrs: &[Vec<Attribute>],
    enable_tracing: bool,
) -> Vec<TokenStream> {
    trait_items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            if let TraitItem::Fn(method) = item {
                let method_name = &method.sig.ident;
                let reciever = match &method.sig.receiver(){
                    Some(recv) => {
                        if recv.mutability.is_some() {
                            quote! {&mut self}
                        } else {
                            quote! { &self }
                        }
                    },
                    None => quote!{},
                };
                let variant_name: Ident = IdentCased(method_name.clone()).to_pascal_case().into();
                let retn = &method.sig.output;
                let is_async = method.sig.asyncness.is_some();
                let maybe_async = if is_async { quote! { async } } else { quote! {} };

                let request_struct_ident = &tmsgs[index].0;

                let inputs = method.sig.inputs.iter().map(|arg| {
                    match arg {
                        syn::FnArg::Typed(pat) => {
                            let name = pat.pat.clone();
                            let ty = pat.ty.clone();
                            quote! { #name: #ty, }
                        }
                        syn::FnArg::Receiver(_) => quote! {},
                    }
                });

                let args = method.sig.inputs.iter().filter_map(|arg| {
                    match arg {
                        syn::FnArg::Typed(pat) => {
                            let name = pat.pat.clone();
                            let ty = &pat.ty;
                            // Skip Context type - it's not in the request struct
                            if let syn::Type::Path(type_path) = &**ty {
                                if let Some(segment) = type_path.path.segments.last() {
                                    if segment.ident == "Context" {
                                        return None;
                                    }
                                }
                            }
                            Some(quote! { #name, })
                        }
                        syn::FnArg::Receiver(_) => None,
                    }
                });

                // Get tracing attributes for this method
                let attrs = &method_attrs[index];

                // If enable_tracing is true and no explicit attributes, add default
                let tracing_attrs: Vec<TokenStream> = if enable_tracing && attrs.is_empty() {
                    vec![quote! { #[tracing::instrument(skip(self))] }]
                } else {
                    attrs.iter().map(|attr| quote! { #attr }).collect()
                };

                // r[impl jetstream.macro.client_error]
                Some(quote! {
                    #(#tracing_attrs)*
                    #maybe_async fn #method_name(#reciever, #(#inputs)*) #retn {
                        let req = Tmessage::#variant_name(#request_struct_ident {
                            #(#args)*
                        });
                        let context = Context::default();
                        let rframe = self.mux.rpc(context, req).await.await?;
                        let rmsg = rframe.msg;
                        match rmsg {
                            Rmessage::#variant_name(msg) => Ok(msg.0),
                            // When client receives an error frame, convert it to jetstream::prelude::Error
                            Rmessage::Error(err) => Err(err),
                            _ => Err(Error::InvalidResponse),
                        }
                    }
                })
            } else {
                None
            }
        })
        .collect()
}
