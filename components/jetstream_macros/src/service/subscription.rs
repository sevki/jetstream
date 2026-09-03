//! `#[subscription]`: the declaration that makes a method streaming.
//!
//! r[impl jetstream.subscription.surface.declared]
//! The declaration is the protocol's, not the call site's — a dispatcher
//! has to route on it before it decodes the payload, and a caller must
//! not be able to turn a unary method into a subscription by asking
//! differently.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, TraitItemFn, Type};

/// A method declared `#[subscription]`.
pub struct Streaming {
    /// `T` in `Subscription<T, D>` — one item of the sequence.
    pub item: Type,
    /// `D` — what the end carries.
    pub done: Type,
}

// r[impl jetstream.subscription.lossy.declared]
// `lossy` is parsed and refused rather than carried: nothing downstream
// can act on it yet, and a field nobody reads is a promise nobody keeps.

/// Find `#[subscription]` on a method, take it off, and read the types
/// out of the return position.
///
/// Taking it off matters: the attribute is the macro's own, and a trait
/// emitted with it still attached does not compile.
pub fn take(method: &mut TraitItemFn) -> Result<Option<Streaming>, syn::Error> {
    let Some(at) = method
        .attrs
        .iter()
        .position(|a| a.path().is_ident("subscription"))
    else {
        return Ok(None);
    };
    let attr = method.attrs.remove(at);

    let mut lossy = false;
    if !matches!(attr.meta, syn::Meta::Path(_)) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("lossy") {
                lossy = true;
                Ok(())
            } else {
                Err(meta.error("expected `lossy`"))
            }
        })?;
    }

    if lossy {
        return Err(syn::Error::new_spanned(
            &method.sig.ident,
            "#[subscription(lossy)] is not implemented yet: the datagram \
             realisation it needs has not landed. Accepting it here would \
             give a reliable subscription to a caller who asked for a \
             lossy one, which is worse than refusing.",
        ));
    }

    let (item, done) = subscription_types(method)?;
    Ok(Some(Streaming { item, done }))
}

/// Read `T` and `D` out of `-> Subscription<T, D>`.
fn subscription_types(
    method: &TraitItemFn,
) -> Result<(Type, Type), syn::Error> {
    let bad = |span| {
        syn::Error::new(
            span,
            "a #[subscription] method must return `Subscription<Item, Done>`",
        )
    };

    let syn::ReturnType::Type(_, ty) = &method.sig.output else {
        return Err(bad(method.sig.ident.span()));
    };
    let Type::Path(path) = &**ty else {
        return Err(bad(method.sig.ident.span()));
    };
    let Some(last) = path.path.segments.last() else {
        return Err(bad(method.sig.ident.span()));
    };
    if last.ident != "Subscription" {
        return Err(bad(method.sig.ident.span()));
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return Err(bad(method.sig.ident.span()));
    };
    let mut types = args.args.iter().filter_map(|a| match a {
        syn::GenericArgument::Type(t) => Some(t.clone()),
        _ => None,
    });
    match (types.next(), types.next()) {
        (Some(item), Some(done)) => Ok((item, done)),
        _ => Err(bad(method.sig.ident.span())),
    }
}

/// The name of the struct holding a subscription's terminal value.
pub fn done_struct_name(return_struct: &Ident) -> Ident {
    format_ident!("{}Done", return_struct)
}

/// r[impl jetstream.subscription.termination.discriminant]
/// The terminator enum: one variant per streaming method, encoded with
/// the message id of the request that opened the subscription in front
/// of it.
///
/// `RDONE` is one **global** message id, which is what keeps
/// `102 + 2 * index` intact for every other method — and a protocol with
/// two subscription methods then has two terminal types under that one
/// id. The tag would tell them apart, and `decode` never sees the tag.
/// So the payload names its method.
pub fn generate_done_enum(variants: &[(Ident, Ident, Ident)]) -> TokenStream {
    // (variant name, done struct, request message id const)
    let decls = variants.iter().map(|(variant, done, _)| {
        quote! { #variant(#done), }
    });
    let byte_sizes = variants.iter().map(|(variant, ..)| {
        quote! { Done::#variant(v) => 1 + v.byte_size() }
    });
    let encodes = variants.iter().map(|(variant, _, id)| {
        quote! {
            Done::#variant(v) => {
                WireFormat::encode(&#id, writer)?;
                v.encode(writer)
            }
        }
    });
    let decodes = variants.iter().map(|(variant, _, id)| {
        quote! {
            #id => Ok(Done::#variant(WireFormat::decode(reader)?)),
        }
    });

    quote! {
        /// r[impl jetstream.subscription.dispatch.terminator]
        /// The end of a subscription, and what it ended with. The
        /// payload names its method, because `RDONE` is one global id
        /// and a decoder is handed the type byte alone.
        #[derive(Debug)]
        pub enum Done {
            #(#decls)*
            /// r[impl jetstream.subscription.dispatch.terminator]
            /// The subscription was cut short, so it carries no result:
            /// a producer stopped by cancellation never produced one,
            /// and a default value in its place would report a result
            /// that did not happen. Its only job is to free the tag.
            ///
            /// Method id zero, which no method can have —
            /// `MESSAGE_ID_START` is 102.
            Cancelled,
        }

        impl WireFormat for Done {
            fn byte_size(&self) -> u32 {
                match self {
                    #(#byte_sizes,)*
                    Done::Cancelled => 1,
                }
            }

            fn encode<W: std::io::Write>(
                &self,
                writer: &mut W,
            ) -> std::io::Result<()> {
                match self {
                    #(#encodes,)*
                    Done::Cancelled => WireFormat::encode(&0u8, writer),
                }
            }

            fn decode<R: std::io::Read>(
                reader: &mut R,
            ) -> std::io::Result<Self> {
                let method: u8 = WireFormat::decode(reader)?;
                match method {
                    0u8 => Ok(Done::Cancelled),
                    #(#decodes)*
                    other => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("terminator for an unknown method: {}", other),
                    )),
                }
            }
        }
    }
}
