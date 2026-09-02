use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, TraitItem};

use crate::{
    service::subscription::{done_struct_name, Streaming},
    utils::case_conversion::IdentCased,
};

#[allow(clippy::too_many_arguments)]
pub fn generate_server(
    service_name: &Ident,
    trait_name: &Ident,
    trait_items: &[TraitItem],
    tmsgs: &[(Ident, TokenStream)],
    rmsgs: &[(Ident, TokenStream)],
    method_attrs: &[Vec<Attribute>],
    enable_tracing: bool,
    streaming: &HashMap<String, Streaming>,
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

                // r[impl jetstream.subscription.surface.declared]
                // A streaming method is not answerable with one frame,
                // and the dispatcher routes it to `rpc_stream` before it
                // gets here. This arm exists only so the match is
                // exhaustive; reaching it means the routing is wrong.
                if streaming.contains_key(&method_name.to_string()) {
                    return Some(quote! {
                        {
                            let _ = msg;
                            Err(Error::new(
                                "a subscription cannot be answered with one frame",
                            ))
                        }
                    });
                }

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

    // r[impl jetstream.subscription.cancel]
    // The dispatcher takes cancellations before they reach here, so this
    // arm exists to make the match exhaustive. Reaching it means a
    // cancellation was routed as an ordinary call.
    let cancel_match_arm = if streaming.is_empty() {
        quote! {}
    } else {
        quote! {
            Tmessage::Cancel(_) => Err(Error::new(
                "a cancellation is not a unary call",
            )),
        }
    };

    // r[impl jetstream.version.framer.server-dispatch]
    // Version negotiation match arm — handles Tversion before service methods
    let version_match_arm = quote! {
        Tmessage::Version(tversion) => {
            use std::str::FromStr;
            let client_version = jetstream::prelude::Version::from_str(&tversion.version)
                .map_err(|e| Error::new(e))?;
            match Self::version(client_version) {
                Ok(negotiated) => Ok(Rmessage::Version(jetstream::prelude::Rversion {
                    msize: tversion.msize,
                    version: negotiated.to_string(),
                })),
                Err(_) => Ok(Rmessage::Version(jetstream::prelude::Rversion {
                    msize: 0,
                    version: "unknown".to_string(),
                })),
            }
        }
    };

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
    let trait_methods = generate_trait_methods(
        trait_items,
        method_attrs,
        enable_tracing,
        streaming,
    );

    let streaming_impl =
        generate_streaming(trait_items, tmsgs, rmsgs, streaming);
    let tcancel_impl = if streaming.is_empty() {
        quote! {}
    } else {
        // r[impl jetstream.subscription.cancel]
        quote! {
            fn tcancel(oldtag: u16, binding: u64) -> Option<Tmessage> {
                Some(Tmessage::Cancel(
                    jetstream::prelude::subscription::Tcancel {
                        oldtag,
                        binding,
                    },
                ))
            }
        }
    };

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
            const NAME: &'static str = PROTOCOL_NAME;

            #tcancel_impl
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
                        #version_match_arm
                        #cancel_match_arm
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

            #streaming_impl
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
    streaming: &HashMap<String, Streaming>,
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

                // A subscription method is not `async`: it hands back a
                // stream, which is where the waiting happens.
                let maybe_await =
                    if streaming.contains_key(&method_name.to_string()) {
                        quote! {}
                    } else {
                        quote! { .await }
                    };

                Some(quote! {
                    #(#tracing_attrs)*
                    #method_sig {
                        self.inner.#method_name(#(#params),*) #maybe_await
                    }
                })
            } else {
                None
            }
        })
        .collect()
}

/// r[impl jetstream.subscription.dispatch.declared]
/// Everything the dispatcher needs from a protocol that has
/// subscriptions: what streams, what cancels, what answers a
/// cancellation, what terminates one cut short, and how to serve one.
///
/// A protocol with none emits nothing here, so the defaults stand and it
/// compiles exactly as it did — r[jetstream.subscription.compat.rpc-layer].
fn generate_streaming(
    trait_items: &[TraitItem],
    tmsgs: &[(Ident, TokenStream)],
    rmsgs: &[(Ident, TokenStream)],
    streaming: &HashMap<String, Streaming>,
) -> TokenStream {
    if streaming.is_empty() {
        return quote! {};
    }

    let mut request_ids = Vec::new();
    let mut serve_arms = Vec::new();

    for (index, item) in trait_items.iter().enumerate() {
        let TraitItem::Fn(method) = item else {
            continue;
        };
        let method_name = &method.sig.ident;
        if !streaming.contains_key(&method_name.to_string()) {
            continue;
        }
        let name: IdentCased = method_name.into();
        let variant_name: Ident = name.to_pascal_case().into();
        let request_id = Ident::new(
            &format!("T{}", method_name.to_string().to_uppercase()),
            method_name.span(),
        );
        request_ids.push(request_id);

        let return_struct_ident = &rmsgs[index].0;
        let done_ident = done_struct_name(return_struct_ident);
        let _ = &tmsgs[index];

        // The arguments the trait method takes, read out of the request
        // struct — with `ctx` passed through as it is for a unary call.
        let params = method.sig.inputs.iter().filter_map(|arg| match arg {
            syn::FnArg::Typed(pat) => {
                let name = pat.pat.clone();
                if let syn::Type::Path(type_path) = &*pat.ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        if segment.ident == "Context" {
                            return Some(quote! { ctx });
                        }
                    }
                }
                Some(quote! { msg.#name })
            }
            syn::FnArg::Receiver(_) => None,
        });

        serve_arms.push(quote! {
            Tmessage::#variant_name(msg) => {
                // r[impl jetstream.subscription.cancel]
                // Serving is where the producer is handed the token
                // that says its subscriber has gone.
                // Through the generated wrapper, not `self.inner`: that
                // is where `#[tracing::instrument]` is installed, and a
                // unary call reaches it as `self.#method_name`. Calling
                // the inner trait directly here made subscription spans
                // the only ones that silently went missing.
                let items = self
                    .#method_name(#(#params),*)
                    .serve(cancel);
                // A terminal item ends the response stream, rather than
                // being mapped and left to the source to stop on its
                // own. The dispatcher frees the tag on `Out::Ended` and
                // nowhere else, so a source that yields `Err` (or its
                // terminator) and then hangs would hold the tag open for
                // the life of the connection and never produce the
                // synthetic terminator that releases the caller. Ending
                // here also means nothing can be delivered *after* an
                // ending, which is the guarantee the surface makes.
                Box::pin(jetstream::prelude::futures::stream::unfold(
                    (items, false),
                    move |(mut items, finished)| async move {
                        use jetstream::prelude::futures::StreamExt as _;
                        if finished {
                            return None;
                        }
                        let (msg, finished) = match items.next().await? {
                            Ok(Item::Next(value)) => (
                                Rmessage::#variant_name(
                                    #return_struct_ident(value),
                                ),
                                false,
                            ),
                            Ok(Item::Done(value)) => (
                                Rmessage::Done(Done::#variant_name(
                                    #done_ident(value),
                                )),
                                true,
                            ),
                            // r[impl jetstream.subscription.surface.termination]
                            // A producer failure ends *this* subscription,
                            // not the lane. Returning `Err` here made the
                            // item a transport error, which `server::run`
                            // propagates — so one failing room tore down
                            // every other subscription and every unary call
                            // sharing the lane, and the caller never reached
                            // the error arm its generated client has. As an
                            // error frame under this tag it is the failure
                            // the surface has to distinguish from a normal
                            // end, delivered to the one subscriber it
                            // concerns.
                            Err(err) => (Rmessage::Error(err), true),
                        };
                        Some((
                            Ok(Frame { tag, msg }),
                            (items, finished),
                        ))
                    },
                )) as jetstream::prelude::server::ResponseStream<Self>
            }
        });
    }

    quote! {
        // r[impl jetstream.subscription.surface.declared]
        fn is_streaming(message_type: u8) -> bool {
            matches!(message_type, #(#request_ids)|*)
        }

        // r[impl jetstream.subscription.cancel]
        fn cancel_target(frame: &Frame<Tmessage>) -> Option<u16> {
            match &frame.msg {
                Tmessage::Cancel(cancel) => Some(cancel.oldtag),
                _ => None,
            }
        }

        // r[impl jetstream.subscription.cancel]
        fn cancel_ack(oldtag: u16) -> Option<Rmessage> {
            Some(Rmessage::CancelAck(
                jetstream::prelude::subscription::Rcancel { oldtag },
            ))
        }

        // r[impl jetstream.subscription.dispatch.terminator]
        // No result, because a producer stopped by cancellation never
        // produced one. Its job is to free the tag.
        fn cancelled_terminator(_method: u8) -> Option<Rmessage> {
            Some(Rmessage::Done(Done::Cancelled))
        }

        fn rpc_stream(
            &mut self,
            ctx: Context,
            frame: Frame<<Self as Protocol>::Request>,
            cancel: jetstream::prelude::subscription::CancellationToken,
        ) -> impl ::core::future::Future<
            Output = jetstream::prelude::server::ResponseStream<Self>,
        > + Send + Sync {
            Box::pin(async move {
                let tag = frame.tag;
                match frame.msg {
                    #(#serve_arms)*
                    _ => Box::pin(
                        jetstream::prelude::futures::stream::empty(),
                    ) as jetstream::prelude::server::ResponseStream<Self>,
                }
            })
        }
    }
}
