use std::iter;

use proc_macro2::Literal;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{Ident, ItemTrait, TraitItem};

struct IdentCased(Ident);

impl From<&Ident> for IdentCased {
    fn from(ident: &Ident) -> Self {
        IdentCased(ident.clone())
    }
}

impl IdentCased {
    fn remove_prefix(&self) -> Self {
        let s = self.0.to_string();
        IdentCased(Ident::new(&s[1..], self.0.span()))
    }
    #[allow(dead_code)]
    fn to_title_case(&self) -> Self {
        let converter =
            convert_case::Converter::new().to_case(convert_case::Case::Title);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    #[allow(dead_code)]
    fn to_upper_case(&self) -> Self {
        let converter =
            convert_case::Converter::new().to_case(convert_case::Case::Upper);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    fn to_screaming_snake_case(&self) -> Self {
        let converter = convert_case::Converter::new()
            .to_case(convert_case::Case::ScreamingSnake);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    fn to_pascale_case(&self) -> Self {
        let converter =
            convert_case::Converter::new().to_case(convert_case::Case::Pascal);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    #[allow(dead_code)]
    fn to_upper_flat(&self) -> Self {
        let converter = convert_case::Converter::new()
            .to_case(convert_case::Case::UpperFlat);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    #[allow(dead_code)]
    fn remove_whitespace(&self) -> Self {
        let s = self.0.to_string().split_whitespace().collect::<String>();
        IdentCased(Ident::new(&s, self.0.span()))
    }
}

impl From<IdentCased> for Ident {
    fn from(ident: IdentCased) -> Self {
        ident.0
    }
}

enum Direction {
    Rx,
    Tx,
}

fn generate_frame(
    direction: Direction,
    msgs: &[(Ident, proc_macro2::TokenStream)],
) -> proc_macro2::TokenStream {
    let enum_name = match direction {
        Direction::Rx => quote! { Rmessage },
        Direction::Tx => quote! { Tmessage },
    };
    let frame_name = match direction {
        Direction::Rx => quote! { Rframe },
        Direction::Tx => quote! { Tframe },
    };

    let msg_variants = msgs.iter().map(|(ident, _p)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascale_case().into();
        quote! {
            #variant_name(#ident),
        }
    });
    let cloned_byte_sizes = msgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascale_case().into();
        quote! {
            #enum_name::#variant_name(msg) => msg.byte_size()
        }
    });

    let match_arms = msgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascale_case().into();
        quote! {
            #enum_name::#variant_name(msg)
        }
    });

    let encode_bodies = msgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.to_screaming_snake_case().into();

        let new_ident = Ident::new(&format!("{}", variant_name), ident.span());
        quote! {
            #new_ident,
        }
    });

    let decode_bodies = msgs.iter().map(|(ident, _)| {
        let name: IdentCased = ident.into();
        let variant_name: Ident = name.remove_prefix().to_pascale_case().into();

        let const_name: Ident = name.to_screaming_snake_case().into();
        quote!{
                #const_name => Ok(#enum_name::#variant_name(WireFormat::decode(reader)?)),
        }
    });

    let type_match_arms =
        std::iter::zip(match_arms.clone(), encode_bodies.clone()).map(
            |(arm, body)| {
                quote! {
                    #arm => #body
                }
            },
        );

    let encode_match_arms = match_arms.clone().map(|arm| {
        quote! {
            #arm => msg.encode(writer)?,
        }
    });

    quote! {
        #[derive(Debug)]
        pub enum #enum_name {
            #( #msg_variants )*
        }
        #[derive(Debug)]
        pub struct #frame_name {
            pub tag: u16,
            pub msg: #enum_name,
        }
        impl WireFormat for #frame_name {
            fn byte_size(&self) -> u32 {
                let msg = &self.msg;
                let msg_size = match msg {
                    #(
                        #cloned_byte_sizes,
                     )*
                };
                // size + type + tag + message size
                (std::mem::size_of::<u32>() + std::mem::size_of::<u8>() + std::mem::size_of::<u16>())
                    as u32
                    + msg_size
            }

            fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                self.byte_size().encode(writer)?;

                let ty = match &self.msg {
                    #(
                        #type_match_arms
                     )*
                };

                ty.encode(writer)?;

                self.tag.encode(writer)?;

                match &self.msg {
                    #(
                        #encode_match_arms
                     )*
                }

                Ok(())
            }

            fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                let byte_size = u32::decode(reader)?;

                // byte_size includes the size of byte_size so remove that from the
                // expected length of the message.  Also make sure that byte_size is at least
                // that long to begin with.
                if byte_size < mem::size_of::<u32>() as u32 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("byte_size(= {}) is less than 4 bytes", byte_size),
                    ));
                } else {
                    let byte_size = byte_size -
                    (mem::size_of::<u32>() + mem::size_of::<u8>() + mem::size_of::<u16>()) as u32;
                }

                let ty = u8::decode(reader)?;

                let tag: u16 = u16::decode(reader)?;
                let reader = &mut reader.take((byte_size) as u64);

                let msg: #enum_name = Self::decode_message(reader, ty)?;

                Ok(#frame_name { tag, msg })
            }
        }
        impl #frame_name {
            fn decode_message<R: Read>(reader: &mut R, ty: u8) -> io::Result<#enum_name> {
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

fn generate_tframe(
    tmsgs: &[(Ident, proc_macro2::TokenStream)],
) -> proc_macro2::TokenStream {
    generate_frame(Direction::Tx, tmsgs)
}

fn generate_rframe(
    rmsgs: &[(Ident, proc_macro2::TokenStream)],
) -> proc_macro2::TokenStream {
    generate_frame(Direction::Rx, rmsgs)
}

fn generate_msg_id(
    index: usize,
    method_name: &Ident,
) -> proc_macro2::TokenStream {
    let upper_cased_method_name = method_name.to_string().to_uppercase();
    let tmsg_const_name = Ident::new(
        &format!("T{}", upper_cased_method_name),
        method_name.span(),
    );
    let rmsg_const_name = Ident::new(
        &format!("R{}", upper_cased_method_name),
        method_name.span(),
    );
    let offset = 2 * index as u8;

    quote! {
        pub const #tmsg_const_name: u8 = MESSAGE_ID_START + #offset;
        pub const #rmsg_const_name: u8 = MESSAGE_ID_START + #offset + 1;
    }
}

fn generate_input_struct(
    request_struct_ident: &Ident,
    method_sig: &syn::Signature,
) -> proc_macro2::TokenStream {
    let inputs = method_sig.inputs.iter().map(|arg| match arg {
        syn::FnArg::Typed(pat) => {
            let name = pat.pat.clone();
            let ty = pat.ty.clone();
            quote! {
                pub #name: #ty,
            }
        }
        syn::FnArg::Receiver(_) => quote! {},
    });

    quote! {
        #[allow(non_camel_case_types)]
        #[derive(Debug, JetStreamWireFormat)]
        pub struct #request_struct_ident {
            #(#inputs)*
        }
    }
}
fn generate_return_struct(
    return_struct_ident: &Ident,
    method_sig: &syn::Signature,
) -> proc_macro2::TokenStream {
    match &method_sig.output {
        syn::ReturnType::Type(_, ty) => {
            match &**ty {
                syn::Type::Path(type_path) => {
                    // Check if it's a Result type
                    if let Some(segment) = type_path.path.segments.last() {
                        if segment.ident == "Result" {
                            // Extract the success type from Result<T, E>
                            if let syn::PathArguments::AngleBracketed(args) =
                                &segment.arguments
                            {
                                if let Some(syn::GenericArgument::Type(
                                    success_type,
                                )) = args.args.first()
                                {
                                    return quote! {
                                        #[allow(non_camel_case_types)]
                                        #[derive(Debug, JetStreamWireFormat)]
                                        pub struct #return_struct_ident(pub #success_type);
                                    };
                                }
                            }
                        }
                    }
                    // If not a Result or couldn't extract type, use the whole type
                    quote! {
                        #[allow(non_camel_case_types)]
                        #[derive(Debug, JetStreamWireFormat)]
                        pub struct #return_struct_ident(pub #ty);
                    }
                }
                // Handle other return type variants if needed
                _ => quote! {
                    #[allow(non_camel_case_types)]
                    #[derive(Debug, JetStreamWireFormat)]
                    pub struct #return_struct_ident(pub #ty);
                },
            }
        }
        syn::ReturnType::Default => quote! {
           #[allow(non_camel_case_types)]
           #[derive(Debug, JetStreamWireFormat)]
           pub struct #return_struct_ident;
        },
    }
}

fn generate_match_arms(
    tmsgs: impl Iterator<Item = (Ident, proc_macro2::TokenStream)>,
) -> impl Iterator<Item = proc_macro2::TokenStream> {
    tmsgs.map(|(ident, _)| {
        let name: IdentCased = (&ident).into();
        let variant_name: Ident = name.remove_prefix().to_pascale_case().into();
        quote! {
            Tmessage::#variant_name(msg)
        }
    })
}

pub(crate) fn service_impl(item: ItemTrait) -> TokenStream {
    let trait_name = &item.ident;
    let trait_items = &item.items;
    let vis = &item.vis;

    // Generate message structs and enum variants
    // let mut message_structs = Vec::new();
    let mut tmsgs = Vec::new();
    let mut rmsgs = Vec::new();
    let mut msg_ids = Vec::new();
    let protocol_name = format_ident!("{}Protocol", trait_name);
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
    let mut calls = vec![];
    let mut signatures = vec![];
    let mut server_calls = vec![];
    {
        let with_calls = item
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| match item {
            TraitItem::Fn(method) => {
                let method_name = &method.sig.ident;

                let request_struct_ident = Ident::new(
                    &format!("T{}", method_name),
                    method_name.span(),
                );
                let return_struct_ident = Ident::new(
                    &format!("R{}", method_name),
                    method_name.span(),
                );
                let _output_type = match &method.sig.output {
                    syn::ReturnType::Type(_, ty) => quote! { #ty },
                    syn::ReturnType::Default => quote! { () },
                };
                let msg_id = generate_msg_id(index, method_name);
                msg_ids.push(msg_id);
                let request_struct =
                    generate_input_struct(&request_struct_ident.clone(), &method.sig);
                let return_struct =
                    generate_return_struct(&return_struct_ident.clone(), &method.sig);


                tmsgs.push((request_struct_ident.clone(),request_struct.clone()));
                rmsgs.push((return_struct_ident.clone(),return_struct.clone()));
                let has_req = method.sig.inputs.iter().count() > 1;
                let is_async = method.sig.asyncness.is_some();
                let maybe_await = if is_async { quote! { .await } } else { quote! {} };
                if has_req {
                    let spread_req = method.sig.inputs.iter().map(|arg| match arg {
                        syn::FnArg::Typed(pat) => {
                            let name = pat.pat.clone();
                            quote! { req.#name, }
                        }
                        syn::FnArg::Receiver(_) => quote! {},
                    });
                    quote! {
                        #[inline]
                        fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> impl ::core::future::Future<
                            Output = Result<#return_struct_ident, Box<dyn Error + Send + Sync>>,
                        > + Send + Sync {
                            Box::pin(async move {
                                #return_struct_ident(tag, #trait_name::#method_name(&mut self.inner,
                                    #(#spread_req)*
                                )#maybe_await)
                            })
                        }
                    }
                } else {
                    quote! {
                        #[inline]
                        fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> impl ::core::future::Future<
                            Output = Result<#return_struct_ident, Box<dyn Error + Send + Sync>>,
                        > + Send + Sync {
                            Box::pin(async move {
                                #return_struct_ident(tag, #trait_name::#method_name(&mut self.inner)#maybe_await)
                            })
                        }
                    }
                }

            }
            _ => quote! { #item },
        });
        calls.extend(with_calls);
        let with_signatures = item.items.iter().enumerate().map(|(index, item)| match item {
            TraitItem::Fn(method) => {
                let method_name = &method.sig.ident;
                let request_struct_ident = Ident::new(
                    &format!("T{}", method_name),
                    method_name.span(),
                );
                let return_struct_ident = Ident::new(
                    &format!("R{}", method_name),
                    method_name.span(),
                );
                let _output_type = match &method.sig.output {
                    syn::ReturnType::Type(_, ty) => quote! { #ty },
                    syn::ReturnType::Default => quote! { () },
                };
                let _msg_id = generate_msg_id(index, method_name);
                // msg_ids.push(msg_id);
                let _request_struct =
                    generate_input_struct(&request_struct_ident.clone(), &method.sig);
                let _return_struct =
                    generate_return_struct(&return_struct_ident.clone(), &method.sig);
                // tmsgs.push((request_struct_ident.clone(),request_struct.clone()));
                //        rmsgs.push((return_struct_ident.clone(),return_struct.clone()));
                let has_req = method.sig.inputs.iter().count() > 1;
                let is_async = method.sig.asyncness.is_some();
                let _maybe_await = if is_async { quote! { .await } } else { quote! {} };
                if has_req {
                    let _spread_req = method.sig.inputs.iter().map(|arg| match arg {
                        syn::FnArg::Typed(pat) => {
                            let name = pat.pat.clone();
                            quote! { req.#name, }
                        }
                        syn::FnArg::Receiver(_) => quote! {},
                    });
                    quote! {
                        async fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> Result<#return_struct_ident,Box<dyn Error + Send + Sync>>;
                    }
                } else {
                    quote! {
                        async fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> Result<#return_struct_ident,Box<dyn Error + Send + Sync>>;
                    }
                }
            }
            _ => quote! { #item },
        });

        signatures.extend(with_signatures);

        let match_arms = generate_match_arms(tmsgs.clone().into_iter());
        let match_arm_bodies: Vec<proc_macro2::TokenStream> = item
            .items
            .clone()
            .iter()
            .map(|item| match item {
                TraitItem::Fn(method) => {
                    let method_name = &method.sig.ident;
                    let name: IdentCased = method_name.into();
                    let variant_name: Ident = name.to_pascale_case().into();
                    let return_struct_ident = Ident::new(
                        &format!("R{}", method_name),
                        method_name.span(),
                    );
                    let variables_spead =
                        method.sig.inputs.iter().map(|arg| match arg {
                            syn::FnArg::Typed(pat) => {
                                let name = pat.pat.clone();
                                quote! { msg.#name, }
                            }
                            syn::FnArg::Receiver(_) => quote! {&self.inner,},
                        });
                    quote! {
                         {
                            let msg = #trait_name::#method_name(
                                #(
                                    #variables_spead
                                )*
                            ).await?;
                            let ret = #return_struct_ident(msg);
                            Ok(Rframe{
                                tag: req.tag,
                                msg: Rmessage::#variant_name(ret)
                            })
                        }
                    }
                }
                _ => quote! {},
            })
            .collect();
        let matches = std::iter::zip(match_arms, match_arm_bodies.iter()).map(
            |(arm, body)| {
                quote! {
                    #arm => #body
                }
            },
        );
        server_calls.extend(iter::once(quote! {
            #[inline]
            fn rpc(&mut self, req:Self::Request) -> impl ::core::future::Future<
                Output = Result<Self::Response, Error>,
            > + Send + Sync {
                Box::pin(async move {match req.msg {
                    #(
                        #matches
                     )*
                }})
            }

        }));
    }

    // make a const with the digest
    let digest = Literal::string(digest.as_str());
    let tmsg_definitions = tmsgs.iter().map(|(_ident, def)| {
        quote! {
            #def
        }
    });

    let rmsg_definitions = rmsgs.iter().map(|(_ident, def)| {
        quote! {
            #def
        }
    });
    let tmessage = generate_tframe(&tmsgs);
    let rmessage = generate_rframe(&rmsgs);
    let proto_mod =
        format_ident!("{}_protocol", trait_name.to_string().to_lowercase());
    quote! {
        pub use trait_variant;


        mod #proto_mod{
            use jetstream::prelude::*;
            use std::io::{self,Read,Write};
            use std::mem;
            use super::#trait_name;
            const MESSAGE_ID_START: u8 = 101;
            pub const PROTOCOL_VERSION: &str = #protocol_version;
            const DIGEST: &str = #digest;

            #(#msg_ids)*

            #(#tmsg_definitions)*

            #(#rmsg_definitions)*

            #tmessage

            impl Message for Tframe{}

            #rmessage

            impl Message for Rframe{}
            #[derive(Clone)]
            pub struct #protocol_name<T: #trait_name>
                where
                    T: #trait_name+ Send + Sync + Sized
            {
                inner: T,
            }
            impl<T: #trait_name> #protocol_name<T>
                where
                    T: #trait_name+ Send + Sync + Sized
            {
                pub fn new(inner: T) -> Self {
                    Self { inner }
                }
            }
            impl<T> Protocol for #protocol_name<T>
            where
                T: #trait_name+ Send + Sync + Sized
            {
                type Request = Tframe;
                type Response = Rframe;
            }
            impl<T> Service for #protocol_name<T>
            where
                T: #trait_name+ Send + Sync + Sized
            {
                #(#server_calls)*
            }
        }

        #[jetstream::prelude::trait_variant::make(Send + Sync)]
        #vis trait #trait_name {
            #(#trait_items)*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_service() {
        let input: ItemTrait = parse_quote! {
            pub trait Echo {
                async fn ping(&self) -> Result<(), std::io::Error>;
            }
        };
        let output = service_impl(input);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        insta::assert_snapshot!(output_str, @r###"
        pub use trait_variant;
        mod echo_protocol {
            use jetstream::prelude::*;
            use std::io::{self, Read, Write};
            use std::mem;
            use super::Echo;
            const MESSAGE_ID_START: u8 = 101;
            const PROTOCOL_VERSION: &str = "dev.branch.jetstream.proto/echo/5.3.0-8d935c22";
            const DIGEST: &str = "8d935c22bef12403928f57643f3e513d37adf7e1b300719f6d3f62e50c4ad006";
            pub const TPING: u8 = MESSAGE_ID_START + 0u8;
            pub const RPING: u8 = MESSAGE_ID_START + 0u8 + 1;
            #[allow(non_camel_case_types)]
            #[derive(Debug, JetStreamWireFormat)]
            pub struct Tping {}
            #[allow(non_camel_case_types)]
            #[derive(Debug, JetStreamWireFormat)]
            pub struct Rping(pub ());
            #[derive(Debug)]
            pub enum Tmessage {
                Ping(Tping),
            }
            #[derive(Debug)]
            pub struct Tframe {
                pub tag: u16,
                pub msg: Tmessage,
            }
            impl WireFormat for Tframe {
                fn byte_size(&self) -> u32 {
                    let msg = &self.msg;
                    let msg_size = match msg {
                        Tmessage::Ping(msg) => msg.byte_size(),
                    };
                    (std::mem::size_of::<u32>() + std::mem::size_of::<u8>()
                        + std::mem::size_of::<u16>()) as u32 + msg_size
                }
                fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                    self.byte_size().encode(writer)?;
                    let ty = match &self.msg {
                        Tmessage::Ping(msg) => TPING,
                    };
                    ty.encode(writer)?;
                    self.tag.encode(writer)?;
                    match &self.msg {
                        Tmessage::Ping(msg) => msg.encode(writer)?,
                    }
                    Ok(())
                }
                fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                    let byte_size = u32::decode(reader)?;
                    if byte_size < mem::size_of::<u32>() as u32 {
                        return Err(
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("byte_size(= {}) is less than 4 bytes", byte_size),
                            ),
                        );
                    } else {
                        let byte_size = byte_size
                            - (mem::size_of::<u32>() + mem::size_of::<u8>()
                                + mem::size_of::<u16>()) as u32;
                    }
                    let ty = u8::decode(reader)?;
                    let tag: u16 = u16::decode(reader)?;
                    let reader = &mut reader.take((byte_size) as u64);
                    let msg: Tmessage = Self::decode_message(reader, ty)?;
                    Ok(Tframe { tag, msg })
                }
            }
            impl Tframe {
                fn decode_message<R: Read>(reader: &mut R, ty: u8) -> io::Result<Tmessage> {
                    match ty {
                        TPING => Ok(Tmessage::Ping(WireFormat::decode(reader)?)),
                        _ => {
                            Err(
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("unknown message type: {}", ty),
                                ),
                            )
                        }
                    }
                }
            }
            impl Message for Tframe {}
            #[derive(Debug)]
            pub enum Rmessage {
                Ping(Rping),
            }
            #[derive(Debug)]
            pub struct Rframe {
                pub tag: u16,
                pub msg: Rmessage,
            }
            impl WireFormat for Rframe {
                fn byte_size(&self) -> u32 {
                    let msg = &self.msg;
                    let msg_size = match msg {
                        Rmessage::Ping(msg) => msg.byte_size(),
                    };
                    (std::mem::size_of::<u32>() + std::mem::size_of::<u8>()
                        + std::mem::size_of::<u16>()) as u32 + msg_size
                }
                fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                    self.byte_size().encode(writer)?;
                    let ty = match &self.msg {
                        Rmessage::Ping(msg) => RPING,
                    };
                    ty.encode(writer)?;
                    self.tag.encode(writer)?;
                    match &self.msg {
                        Rmessage::Ping(msg) => msg.encode(writer)?,
                    }
                    Ok(())
                }
                fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                    let byte_size = u32::decode(reader)?;
                    if byte_size < mem::size_of::<u32>() as u32 {
                        return Err(
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("byte_size(= {}) is less than 4 bytes", byte_size),
                            ),
                        );
                    } else {
                        let byte_size = byte_size
                            - (mem::size_of::<u32>() + mem::size_of::<u8>()
                                + mem::size_of::<u16>()) as u32;
                    }
                    let ty = u8::decode(reader)?;
                    let tag: u16 = u16::decode(reader)?;
                    let reader = &mut reader.take((byte_size) as u64);
                    let msg: Rmessage = Self::decode_message(reader, ty)?;
                    Ok(Rframe { tag, msg })
                }
            }
            impl Rframe {
                fn decode_message<R: Read>(reader: &mut R, ty: u8) -> io::Result<Rmessage> {
                    match ty {
                        RPING => Ok(Rmessage::Ping(WireFormat::decode(reader)?)),
                        _ => {
                            Err(
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("unknown message type: {}", ty),
                                ),
                            )
                        }
                    }
                }
            }
            impl Message for Rframe {}
            #[derive(Clone)]
            pub struct EchoProtocol<T: Echo>
            where
                T: Echo + Send + Sync + Sized,
            {
                inner: T,
            }
            impl<T: Echo> EchoProtocol<T>
            where
                T: Echo + Send + Sync + Sized,
            {
                pub fn new(inner: T) -> Self {
                    Self { inner }
                }
            }
            impl<T> Protocol for EchoProtocol<T>
            where
                T: Echo + Send + Sync + Sized,
            {
                type Request = Tframe;
                type Response = Rframe;
            }
            impl<T> Service for EchoProtocol<T>
            where
                T: Echo + Send + Sync + Sized,
            {
                #[inline]
                fn rpc(
                    &mut self,
                    req: Self::Request,
                ) -> impl ::core::future::Future<
                    Output = Result<Self::Response, Error>,
                > + Send + Sync {
                    Box::pin(async move {
                        match req.msg {
                            Tmessage::Ping(msg) => {
                                let msg = Echo::ping(&self.inner).await?;
                                let ret = Rping(msg);
                                Ok(Rframe {
                                    tag: req.tag,
                                    msg: Rmessage::Ping(ret),
                                })
                            }
                        }
                    })
                }
            }
        }
        #[trait_variant::make(Send+Sync)]
        pub trait Echo {
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
        "###)
    }

    #[test]
    fn test_service_with_args() {
        let input: ItemTrait = parse_quote! {
            pub trait Echo {
                async fn ping(&self, message: String) -> Result<String, std::io::Error>;
            }
        };
        let output = service_impl(input);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        insta::assert_snapshot!(output_str, @r###"
        pub use trait_variant;
        mod echo_protocol {
            use jetstream::prelude::*;
            use std::io::{self, Read, Write};
            use std::mem;
            use super::Echo;
            const MESSAGE_ID_START: u8 = 101;
            const PROTOCOL_VERSION: &str = "dev.branch.jetstream.proto/echo/5.3.0-423bf765";
            const DIGEST: &str = "423bf765c290b0887475654092b49358060592507ba9f4fbafaab630ab27add4";
            pub const TPING: u8 = MESSAGE_ID_START + 0u8;
            pub const RPING: u8 = MESSAGE_ID_START + 0u8 + 1;
            #[allow(non_camel_case_types)]
            #[derive(Debug, JetStreamWireFormat)]
            pub struct Tping {
                pub message: String,
            }
            #[allow(non_camel_case_types)]
            #[derive(Debug, JetStreamWireFormat)]
            pub struct Rping(pub String);
            #[derive(Debug)]
            pub enum Tmessage {
                Ping(Tping),
            }
            #[derive(Debug)]
            pub struct Tframe {
                pub tag: u16,
                pub msg: Tmessage,
            }
            impl WireFormat for Tframe {
                fn byte_size(&self) -> u32 {
                    let msg = &self.msg;
                    let msg_size = match msg {
                        Tmessage::Ping(msg) => msg.byte_size(),
                    };
                    (std::mem::size_of::<u32>() + std::mem::size_of::<u8>()
                        + std::mem::size_of::<u16>()) as u32 + msg_size
                }
                fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                    self.byte_size().encode(writer)?;
                    let ty = match &self.msg {
                        Tmessage::Ping(msg) => TPING,
                    };
                    ty.encode(writer)?;
                    self.tag.encode(writer)?;
                    match &self.msg {
                        Tmessage::Ping(msg) => msg.encode(writer)?,
                    }
                    Ok(())
                }
                fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                    let byte_size = u32::decode(reader)?;
                    if byte_size < mem::size_of::<u32>() as u32 {
                        return Err(
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("byte_size(= {}) is less than 4 bytes", byte_size),
                            ),
                        );
                    } else {
                        let byte_size = byte_size
                            - (mem::size_of::<u32>() + mem::size_of::<u8>()
                                + mem::size_of::<u16>()) as u32;
                    }
                    let ty = u8::decode(reader)?;
                    let tag: u16 = u16::decode(reader)?;
                    let reader = &mut reader.take((byte_size) as u64);
                    let msg: Tmessage = Self::decode_message(reader, ty)?;
                    Ok(Tframe { tag, msg })
                }
            }
            impl Tframe {
                fn decode_message<R: Read>(reader: &mut R, ty: u8) -> io::Result<Tmessage> {
                    match ty {
                        TPING => Ok(Tmessage::Ping(WireFormat::decode(reader)?)),
                        _ => {
                            Err(
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("unknown message type: {}", ty),
                                ),
                            )
                        }
                    }
                }
            }
            impl Message for Tframe {}
            #[derive(Debug)]
            pub enum Rmessage {
                Ping(Rping),
            }
            #[derive(Debug)]
            pub struct Rframe {
                pub tag: u16,
                pub msg: Rmessage,
            }
            impl WireFormat for Rframe {
                fn byte_size(&self) -> u32 {
                    let msg = &self.msg;
                    let msg_size = match msg {
                        Rmessage::Ping(msg) => msg.byte_size(),
                    };
                    (std::mem::size_of::<u32>() + std::mem::size_of::<u8>()
                        + std::mem::size_of::<u16>()) as u32 + msg_size
                }
                fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                    self.byte_size().encode(writer)?;
                    let ty = match &self.msg {
                        Rmessage::Ping(msg) => RPING,
                    };
                    ty.encode(writer)?;
                    self.tag.encode(writer)?;
                    match &self.msg {
                        Rmessage::Ping(msg) => msg.encode(writer)?,
                    }
                    Ok(())
                }
                fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
                    let byte_size = u32::decode(reader)?;
                    if byte_size < mem::size_of::<u32>() as u32 {
                        return Err(
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("byte_size(= {}) is less than 4 bytes", byte_size),
                            ),
                        );
                    } else {
                        let byte_size = byte_size
                            - (mem::size_of::<u32>() + mem::size_of::<u8>()
                                + mem::size_of::<u16>()) as u32;
                    }
                    let ty = u8::decode(reader)?;
                    let tag: u16 = u16::decode(reader)?;
                    let reader = &mut reader.take((byte_size) as u64);
                    let msg: Rmessage = Self::decode_message(reader, ty)?;
                    Ok(Rframe { tag, msg })
                }
            }
            impl Rframe {
                fn decode_message<R: Read>(reader: &mut R, ty: u8) -> io::Result<Rmessage> {
                    match ty {
                        RPING => Ok(Rmessage::Ping(WireFormat::decode(reader)?)),
                        _ => {
                            Err(
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("unknown message type: {}", ty),
                                ),
                            )
                        }
                    }
                }
            }
            impl Message for Rframe {}
            #[derive(Clone)]
            pub struct EchoProtocol<T: Echo>
            where
                T: Echo + Send + Sync + Sized,
            {
                inner: T,
            }
            impl<T: Echo> EchoProtocol<T>
            where
                T: Echo + Send + Sync + Sized,
            {
                pub fn new(inner: T) -> Self {
                    Self { inner }
                }
            }
            impl<T> Protocol for EchoProtocol<T>
            where
                T: Echo + Send + Sync + Sized,
            {
                type Request = Tframe;
                type Response = Rframe;
            }
            impl<T> Service for EchoProtocol<T>
            where
                T: Echo + Send + Sync + Sized,
            {
                #[inline]
                fn rpc(
                    &mut self,
                    req: Self::Request,
                ) -> impl ::core::future::Future<
                    Output = Result<Self::Response, Error>,
                > + Send + Sync {
                    Box::pin(async move {
                        match req.msg {
                            Tmessage::Ping(msg) => {
                                let msg = Echo::ping(&self.inner, msg.message).await?;
                                let ret = Rping(msg);
                                Ok(Rframe {
                                    tag: req.tag,
                                    msg: Rmessage::Ping(ret),
                                })
                            }
                        }
                    })
                }
            }
        }
        #[trait_variant::make(Send+Sync)]
        pub trait Echo {
            async fn ping(&self, message: String) -> Result<String, std::io::Error>;
        }
        "###)
    }
}
