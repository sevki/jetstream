use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::utils::case_conversion::IdentCased;

pub fn generate_tframe(tmsgs: &[(Ident, TokenStream)]) -> TokenStream {
    let enum_name = quote! { Tmessage };

    let msg_variants = tmsgs.iter().map(|(ident, _p)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        let constant_name: Ident = name.to_screaming_snake_case().into();
        quote! {
            #variant_name(#ident) = #constant_name,
        }
    });

    let cloned_byte_sizes = tmsgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        quote! {
            #enum_name::#variant_name(msg) => msg.byte_size()
        }
    });

    let match_arms = tmsgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        quote! {
            #enum_name::#variant_name(msg)
        }
    });

    let decode_bodies = tmsgs.iter().map(|(ident, _)| {
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

    let message_type_match_arms = tmsgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        let const_name: Ident = name.to_screaming_snake_case().into();
        quote! {
            #enum_name::#variant_name(_) => #const_name
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
                match self {
                    #(
                        #message_type_match_arms,
                     )*
                }
            }

            fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                match &self {
                    #(
                        #encode_match_arms
                     )*
                }
                Ok(())
            }

            fn decode<R: std::io::Read>(reader: &mut R, ty: u8) -> std::io::Result<#enum_name> {
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

pub fn generate_rframe(rmsgs: &[(Ident, TokenStream)]) -> TokenStream {
    let enum_name = quote! { Rmessage };

    // Generate regular message variants
    let msg_variants = rmsgs.iter().map(|(ident, _p)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        let constant_name: Ident = name.to_screaming_snake_case().into();
        quote! {
            #variant_name(#ident) = #constant_name,
        }
    });

    // r[impl jetstream.error-message-frame]
    // Add error variant for RERROR handling - this is the error message type
    // for serializing errors across requests
    let error_variant = quote! {
        Error(jetstream::prelude::Error) = RERROR,
    };

    let cloned_byte_sizes = rmsgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        quote! {
            #enum_name::#variant_name(msg) => msg.byte_size()
        }
    });

    // Add error byte size handling
    let error_byte_size = quote! {
        #enum_name::Error(err) => err.byte_size()
    };

    let match_arms = rmsgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        quote! {
            #enum_name::#variant_name(msg)
        }
    });

    let error_match_arm = quote! {
        #enum_name::Error(err)
    };

    let decode_bodies = rmsgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        let const_name: Ident = name.to_screaming_snake_case().into();
        quote! {
            #const_name => Ok(#enum_name::#variant_name(WireFormat::decode(reader)?)),
        }
    });

    // Add RERROR decode handling
    let error_decode = quote! {
        RERROR => Ok(#enum_name::Error(WireFormat::decode(reader)?)),
    };

    let encode_match_arms = match_arms.clone().map(|arm| {
        quote! {
            #arm => msg.encode(writer)?,
        }
    });

    // Add error encode handling
    let error_encode = quote! {
        #error_match_arm => err.encode(writer)?,
    };

    let message_type_match_arms = rmsgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        let const_name: Ident = name.to_screaming_snake_case().into();
        quote! {
            #enum_name::#variant_name(_) => #const_name
        }
    });

    // Add error message type handling
    let error_message_type = quote! {
        #enum_name::Error(_) => RERROR
    };

    quote! {
        #[derive(Debug)]
        #[repr(u8)]
        pub enum #enum_name {
            #( #msg_variants )*
            #error_variant
        }

        impl Framer for #enum_name {

            fn byte_size(&self) -> u32 {
                match &self {
                    #(
                        #cloned_byte_sizes,
                     )*
                    #error_byte_size,
                }
            }

            fn message_type(&self) -> u8 {
                match self {
                    #(
                        #message_type_match_arms,
                     )*
                    #error_message_type,
                }
            }

            fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                match &self {
                    #(
                        #encode_match_arms
                     )*
                    #error_encode
                }
                Ok(())
            }

            fn decode<R: std::io::Read>(reader: &mut R, ty: u8) -> std::io::Result<#enum_name> {
                match ty {
                    #(
                        #decode_bodies
                     )*
                    #error_decode
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown message type: {}", ty),
                    )),
                }
            }
        }
    }
}
