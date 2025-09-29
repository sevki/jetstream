use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::utils::case_conversion::IdentCased;

pub enum Direction {
    Rx,
    Tx,
}

pub fn generate_frame(
    direction: Direction,
    msgs: &[(Ident, TokenStream)],
) -> TokenStream {
    let enum_name = match direction {
        Direction::Rx => quote! { Rmessage },
        Direction::Tx => quote! { Tmessage },
    };

    let msg_variants = msgs.iter().map(|(ident, _p)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        let constant_name: Ident = name.to_screaming_snake_case().into();
        quote! {
            #variant_name(#ident) = #constant_name,
        }
    });
    
    let cloned_byte_sizes = msgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        quote! {
            #enum_name::#variant_name(msg) => msg.byte_size()
        }
    });

    let match_arms = msgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        quote! {
            #enum_name::#variant_name(msg)
        }
    });

    let decode_bodies = msgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        let const_name: Ident = name.to_screaming_snake_case().into();
        quote! {
            #const_name => Ok(#enum_name::#variant_name(WireFormat::decode(reader)?)),
        }
    });

    let encode_match_arms = match_arms.clone().map(|arm| {
        quote! {
            #arm => msg.encode(writer)?,
        }
    });

    quote! {
        #[derive(Debug)]
        #[repr(u8)]
        pub enum #enum_name {
            #( #msg_variants )*
        }

        impl Framer for #enum_name {
            fn byte_size(&self) -> u32 {
                match &self {
                    #(
                        #cloned_byte_sizes,
                     )*
                }
            }

            fn message_type(&self) -> u8 {
                // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
                // between `repr(C)` structs, each of which has the `u8` discriminant as its first
                // field, so we can read the discriminant without offsetting the pointer.
                unsafe { *<*const _>::from(self).cast::<u8>() }
            }

            fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                match &self {
                    #(
                        #encode_match_arms
                     )*
                }
                Ok(())
            }

            fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<#enum_name> {
                match ty {
                    #(
                        #decode_bodies
                     )*
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown message type: {}", ty),
                    )),
                }
            }
        }
    }
}

pub fn generate_tframe(tmsgs: &[(Ident, TokenStream)]) -> TokenStream {
    generate_frame(Direction::Tx, tmsgs)
}

pub fn generate_rframe(rmsgs: &[(Ident, TokenStream)]) -> TokenStream {
    generate_frame(Direction::Rx, rmsgs)
}