use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::utils::case_conversion::IdentCased;

pub fn generate_tframe(
    tmsgs: &[(Ident, TokenStream)],
    has_subscriptions: bool,
) -> TokenStream {
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

    // r[impl jetstream.version.framer.tmessage]
    // Add version variant for TVERSION handling
    let version_variant = quote! {
        Version(jetstream::prelude::Tversion) = TVERSION,
    };

    let version_byte_size = quote! {
        #enum_name::Version(v) => v.byte_size()
    };

    let version_message_type = quote! {
        #enum_name::Version(_) => TVERSION
    };

    let version_encode = quote! {
        #enum_name::Version(v) => v.encode(writer)?,
    };

    let version_decode = quote! {
        TVERSION => Ok(#enum_name::Version(WireFormat::decode(reader)?)),
    };

    // r[impl jetstream.subscription.dispatch.declared]
    // Cancellation is an ordinary request under a fresh tag, naming its
    // target in the payload. A protocol with no subscriptions has
    // nothing to cancel and emits none of this.
    let (
        cancel_variant,
        cancel_byte_size,
        cancel_message_type,
        cancel_encode,
        cancel_decode,
    ) = if has_subscriptions {
        (
            quote! {
                Cancel(jetstream::prelude::subscription::Tcancel) = TCANCEL,
            },
            quote! { #enum_name::Cancel(c) => c.byte_size(), },
            quote! { #enum_name::Cancel(_) => TCANCEL, },
            quote! { #enum_name::Cancel(c) => c.encode(writer)?, },
            quote! {
                TCANCEL => Ok(#enum_name::Cancel(WireFormat::decode(reader)?)),
            },
        )
    } else {
        (quote! {}, quote! {}, quote! {}, quote! {}, quote! {})
    };

    quote! {
        #[derive(Debug)]
        #[repr(u8)]
        pub enum #enum_name {
            #( #msg_variants )*
            #version_variant
            #cancel_variant
        }

        impl Framer for #enum_name {
            fn byte_size(&self) -> u32 {
                match &self {
                    #(
                        #cloned_byte_sizes,
                     )*
                    #version_byte_size,
                    #cancel_byte_size
                }
            }

            fn message_type(&self) -> u8 {
                match self {
                    #(
                        #message_type_match_arms,
                     )*
                    #version_message_type,
                    #cancel_message_type
                }
            }

            fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                match &self {
                    #(
                        #encode_match_arms
                     )*
                    #version_encode
                    #cancel_encode
                }
                Ok(())
            }

            fn decode<R: std::io::Read>(reader: &mut R, ty: u8) -> std::io::Result<#enum_name> {
                match ty {
                    #(
                        #decode_bodies
                     )*
                    #version_decode
                    #cancel_decode
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown message type: {}", ty),
                    )),
                }
            }
        }
    }
}

pub fn generate_rframe(
    rmsgs: &[(Ident, TokenStream)],
    has_subscriptions: bool,
) -> TokenStream {
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

    // r[impl jetstream.version.framer.rmessage]
    // Add version variant for RVERSION handling
    let rversion_variant = quote! {
        Version(jetstream::prelude::Rversion) = RVERSION,
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

    let rversion_byte_size = quote! {
        #enum_name::Version(v) => v.byte_size()
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

    let rversion_match_arm = quote! {
        #enum_name::Version(v)
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

    let rversion_decode = quote! {
        RVERSION => Ok(#enum_name::Version(WireFormat::decode(reader)?)),
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

    let rversion_encode = quote! {
        #rversion_match_arm => v.encode(writer)?,
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

    let rversion_message_type = quote! {
        #enum_name::Version(_) => RVERSION
    };

    // r[impl jetstream.subscription.termination]
    // r[impl jetstream.subscription.cancel]
    // The terminator and the acknowledgement, both on global ids so a
    // streaming method costs no per-method id.
    let (
        stream_variants,
        stream_byte_size,
        stream_message_type,
        stream_encode,
        stream_decode,
    ) = if has_subscriptions {
        (
            quote! {
                Done(Done) = RDONE,
                CancelAck(jetstream::prelude::subscription::Rcancel) = RCANCEL,
            },
            quote! {
                #enum_name::Done(d) => d.byte_size(),
                #enum_name::CancelAck(a) => a.byte_size(),
            },
            quote! {
                #enum_name::Done(_) => RDONE,
                #enum_name::CancelAck(_) => RCANCEL,
            },
            quote! {
                #enum_name::Done(d) => d.encode(writer)?,
                #enum_name::CancelAck(a) => a.encode(writer)?,
            },
            quote! {
                RDONE => Ok(#enum_name::Done(WireFormat::decode(reader)?)),
                RCANCEL => Ok(#enum_name::CancelAck(WireFormat::decode(reader)?)),
            },
        )
    } else {
        (quote! {}, quote! {}, quote! {}, quote! {}, quote! {})
    };

    quote! {
        #[derive(Debug)]
        #[repr(u8)]
        pub enum #enum_name {
            #( #msg_variants )*
            #error_variant
            #rversion_variant
            #stream_variants
        }

        impl Framer for #enum_name {

            fn byte_size(&self) -> u32 {
                match &self {
                    #(
                        #cloned_byte_sizes,
                     )*
                    #error_byte_size,
                    #rversion_byte_size,
                    #stream_byte_size
                }
            }

            fn message_type(&self) -> u8 {
                match self {
                    #(
                        #message_type_match_arms,
                     )*
                    #error_message_type,
                    #rversion_message_type,
                    #stream_message_type
                }
            }

            fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
                match &self {
                    #(
                        #encode_match_arms
                     )*
                    #error_encode
                    #rversion_encode
                    #stream_encode
                }
                Ok(())
            }

            fn decode<R: std::io::Read>(reader: &mut R, ty: u8) -> std::io::Result<#enum_name> {
                match ty {
                    #(
                        #decode_bodies
                     )*
                    #error_decode
                    #rversion_decode
                    #stream_decode
                    _ => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown message type: {}", ty),
                    )),
                }
            }
        }
    }
}
