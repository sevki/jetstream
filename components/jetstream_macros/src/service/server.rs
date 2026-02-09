use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, TraitItem};

use crate::utils::case_conversion::IdentCased;

#[allow(clippy::too_many_arguments)]
pub fn generate_server(
    service_name: &Ident,
    trait_name: &Ident,
    trait_items: &[TraitItem],
    tmsgs: &[(Ident, TokenStream)],
    rmsgs: &[(Ident, TokenStream)],
    method_attrs: &[Vec<Attribute>],
    enable_tracing: bool,
) -> TokenStream {
    let match_arms = generate_match_arms(
        tmsgs.iter().map(|(id, ts)| (id.clone(), ts.clone())),
    );
    let match_arm_bodies: Vec<TokenStream> = trait_items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| match item {
            TraitItem::Fn(method) => {
                let method_name = &method.sig.ident;
                let name: IdentCased = method_name.into();
                let variant_name: Ident = name.to_pascal_case().into();
                let return_struct_ident = &rmsgs[index].0;

                // Get the method parameters (excluding self and Context)
                // Context is passed separately via _ctx parameter
                let params =
                    method.sig.inputs.iter().filter_map(|arg| match arg {
                        syn::FnArg::Typed(pat) => {
                            let name = pat.pat.clone();
                            let ty = &pat.ty;
                            // Skip Context type - it's not in the message struct
                            if let syn::Type::Path(type_path) = &**ty {
                                if let Some(segment) =
                                    type_path.path.segments.last()
                                {
                                    if segment.ident == "Context" {
                                        return Some(quote! { ctx });
                                    }
                                }
                            }
                            Some(quote! { msg.#name })
                        }
                        syn::FnArg::Receiver(_) => None,
                    });

                Some(quote! {
                    {
                        match self.#method_name(#(#params),*).await {
                            Ok(result) => {
                                let ret = #return_struct_ident(result);
                                Ok(Rmessage::#variant_name(ret))
                            }
                            Err(err) => Err(err.into()),
                        }
                    }
                })
            }
            _ => None,
        })
        .collect();

    let matches = std::iter::zip(match_arms, match_arm_bodies.iter())
        .map(|(arm, body)| quote! { #arm => #body });

    // Add RPC-level tracing span if tracing is enabled
    let rpc_span = if enable_tracing {
        quote! {
            let _span = tracing::debug_span!(
                "rpc_server",
                service = stringify!(#trait_name),
                tag = frame.tag
            );
            let _enter = _span.enter();
        }
    } else {
        quote! {}
    };

    // Generate trait implementation methods
    let trait_methods =
        generate_trait_methods(trait_items, method_attrs, enable_tracing);

    quote! {
        #[derive(Clone, Debug)]
        pub struct #service_name<T: #trait_name> {
            pub inner: T,
        }

        impl<T> Protocol for #service_name<T>
        where
            T: #trait_name + Send + Sync + Sized
        {
            type Request = Tmessage;
            type Response = Rmessage;
            // r[impl jetstream.macro.error-type]
            type Error = Error;
            const VERSION: &'static str = PROTOCOL_VERSION;
        }

        impl<T> Server for #service_name<T>
        where
            T: #trait_name + Send + Sync + Sized
        {
            fn rpc(&mut self, ctx: Context, frame: Frame<<Self as Protocol>::Request>) -> impl ::core::future::Future<
                Output = Result<Frame<<Self as Protocol>::Response>>,
            > + Send + Sync {
                Box::pin(async move {
                    #rpc_span
                    let req: <Self as Protocol>::Request = frame.msg;
                    let res: std::result::Result<<Self as Protocol>::Response, Self::Error> = match req {
                        #(#matches)*
                    };
                    // r[impl jetstream.macro.server-error]
                    // When server inner returns an error, serialize it as an Error frame
                    let response = match res {
                        Ok(msg) => msg,
                        Err(err) => Rmessage::Error(err),
                    };
                    let rframe: Frame<<Self as Protocol>::Response> = Frame::from((frame.tag, response));
                    Ok(rframe)
                })
            }
        }

        impl<T> #trait_name for #service_name<T>
        where
            T: #trait_name + Send + Sync + Sized
        {
            #(#trait_methods)*
        }
    }
}

fn generate_match_arms(
    tmsgs: impl Iterator<Item = (Ident, TokenStream)>,
) -> impl Iterator<Item = TokenStream> {
    tmsgs.map(|(ident, _)| {
        let name: IdentCased = (&ident).into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        quote! { Tmessage::#variant_name(msg) }
    })
}

fn generate_trait_methods(
    trait_items: &[TraitItem],
    method_attrs: &[Vec<Attribute>],
    enable_tracing: bool,
) -> Vec<TokenStream> {
    trait_items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            if let TraitItem::Fn(method) = item {
                let method_sig = &method.sig;
                let method_name = &method_sig.ident;

                // Get the method parameters (excluding self)
                // Pass all parameters including Context
                let params =
                    method_sig.inputs.iter().filter_map(|arg| match arg {
                        syn::FnArg::Typed(pat) => Some(pat.pat.clone()),
                        syn::FnArg::Receiver(_) => None,
                    });

                // Get tracing attributes for this method
                let attrs = &method_attrs[index];

                // r[impl jetstream.macro.tracing-instrument]
                // If enable_tracing is true and no explicit attributes, add default
                let tracing_attrs: Vec<TokenStream> =
                    if enable_tracing && attrs.is_empty() {
                        vec![quote! { #[tracing::instrument(skip(self))] }]
                    } else {
                        attrs.iter().map(|attr| quote! { #attr }).collect()
                    };

                Some(quote! {
                    #(#tracing_attrs)*
                    #method_sig {
                        self.inner.#method_name(#(#params),*).await
                    }
                })
            } else {
                None
            }
        })
        .collect()
}
