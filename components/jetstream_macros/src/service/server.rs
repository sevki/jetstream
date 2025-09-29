use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Ident, TraitItem};

use crate::utils::case_conversion::IdentCased;

fn handle_receiver(recv: &syn::Receiver) -> TokenStream {
    let mutability = &recv.mutability;
    let reference = &recv.reference;

    match (reference, mutability) {
        (Some(_), Some(_)) => quote! { &mut self.inner, },
        (Some(_), None) => quote! { &self.inner, },
        (None, _) => quote! { self.inner, },
    }
}

pub fn generate_server(
    service_name: &Ident,
    trait_name: &Ident,
    trait_items: &[TraitItem],
    tmsgs: &[(Ident, TokenStream)],
    rmsgs: &[(Ident, TokenStream)],
    _protocol_version: &Literal,
) -> TokenStream {
    let match_arms = generate_match_arms(tmsgs.iter().map(|(id, ts)| (id.clone(), ts.clone())));
    let match_arm_bodies: Vec<TokenStream> = trait_items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| match item {
            TraitItem::Fn(method) => {
                let method_name = &method.sig.ident;
                let name: IdentCased = method_name.into();
                let variant_name: Ident = name.to_pascal_case().into();
                let return_struct_ident = &rmsgs[index].0;
                
                let variables_spread = method.sig.inputs.iter().map(|arg| match arg {
                    syn::FnArg::Typed(pat) => {
                        let name = pat.pat.clone();
                        quote! { msg.#name, }
                    }
                    syn::FnArg::Receiver(recv) => handle_receiver(recv),
                });
                
                Some(quote! {
                    {
                        let msg = #trait_name::#method_name(
                            #(#variables_spread)*
                        ).await?;
                        let ret = #return_struct_ident(msg);
                        Ok(Rmessage::#variant_name(ret))
                    }
                })
            }
            _ => None,
        })
        .collect();
        
    let matches = std::iter::zip(match_arms, match_arm_bodies.iter()).map(
        |(arm, body)| quote! { #arm => #body }
    );

    quote! {
        #[derive(Clone)]
        pub struct #service_name<T: #trait_name> {
            pub inner: T,
        }

        impl<T> Protocol for #service_name<T>
        where
            T: #trait_name + Send + Sync + Sized
        {
            type Request = Tmessage;
            type Response = Rmessage;
            type Error = Error;
            const VERSION: &'static str = PROTOCOL_VERSION;

            fn rpc(&mut self, frame: Frame<<Self as Protocol>::Request>) -> impl ::core::future::Future<
                Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
            > + Send + Sync {
                Box::pin(async move {
                    let req: <Self as Protocol>::Request = frame.msg;
                    let res: Result<<Self as Protocol>::Response, Self::Error> = match req {
                        #(#matches)*
                    };
                    let rframe: Frame<<Self as Protocol>::Response> = Frame::from((frame.tag, res?));
                    Ok(rframe)
                })
            }
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