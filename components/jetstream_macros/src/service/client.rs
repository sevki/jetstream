use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Attribute, Ident, TraitItem};

use crate::utils::case_conversion::IdentCased;
#[allow(clippy::too_many_arguments)]
pub fn generate_client(
    channel_name: &Ident,
    trait_name: &Ident,
    trait_items: &[TraitItem],
    tmsgs: &[(Ident, TokenStream)],
    _rmsgs: &[(Ident, TokenStream)],
    _protocol_version: &Literal,
    tag_name: &Ident,
    method_attrs: &[Vec<Attribute>],
    enable_tracing: bool,
) -> TokenStream {
    let client_calls = generate_client_calls(
        trait_items,
        tmsgs,
        tag_name,
        method_attrs,
        enable_tracing,
    );

    // Add RPC-level tracing span if tracing is enabled
    let rpc_span = if enable_tracing {
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
        pub struct #channel_name<'a> {
            pub inner: Box<&'a mut dyn ClientTransport<Self>>,
        }

        impl<'a> Protocol for #channel_name<'a>
        {
            type Request = Tmessage;
            type Response = Rmessage;
            type Error = Error;
            const VERSION: &'static str = PROTOCOL_VERSION;

            fn rpc(&mut self, frame: Frame<<Self as Protocol>::Request>) -> impl ::core::future::Future<
                Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
            > + Send + Sync {
                use futures::{SinkExt, StreamExt};
                Box::pin(async move {
                    #rpc_span
                    self.inner
                        .send(frame)
                        .await?;
                    let frame = self.inner.next().await.unwrap()?;
                    Ok(frame)
                })
            }
        }

        lazy_static::lazy_static! {
            static ref #tag_name: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);
        }

        impl<'a> #trait_name for #channel_name<'a>
        {
            #(#client_calls)*
        }
    }
}

fn generate_client_calls(
    trait_items: &[TraitItem],
    tmsgs: &[(Ident, TokenStream)],
    tag_name: &Ident,
    method_attrs: &[Vec<Attribute>],
    enable_tracing: bool,
) -> Vec<TokenStream> {
    trait_items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            if let TraitItem::Fn(method) = item {
                let method_name = &method.sig.ident;
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

                let args = method.sig.inputs.iter().map(|arg| {
                    match arg {
                        syn::FnArg::Typed(pat) => {
                            let name = pat.pat.clone();
                            quote! { #name, }
                        }
                        syn::FnArg::Receiver(_) => quote! {},
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

                Some(quote! {
                    #(#tracing_attrs)*
                    #maybe_async fn #method_name(&mut self, #(#inputs)*) #retn {
                        let tag = #tag_name.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let req = Tmessage::#variant_name(#request_struct_ident {
                            #(#args)*
                        });
                        let tframe = Frame::from((tag, req));
                        let rframe = self.rpc(tframe).await?;
                        let rmsg = rframe.msg;
                        match rmsg {
                            Rmessage::#variant_name(msg) => Ok(msg.0),
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
