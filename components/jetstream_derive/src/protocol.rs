extern crate proc_macro;

use self::proc_macro::TokenStream;
use quote::quote;
use sha256::digest;
use std::iter;
use syn::{parse_str, Ident, Item, ItemMod, ItemTrait, TraitItem};

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
        #[derive(JetStreamWireFormat)]
        pub struct #request_struct_ident {
            pub tag: u16,
            #(#inputs)*
        }
    }
}

fn generate_return_struct(
    return_struct_ident: &Ident,
    method_sig: &syn::Signature,
) -> proc_macro2::TokenStream {
    match &method_sig.output {
        syn::ReturnType::Type(_, ty) => quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, JetStreamWireFormat)]
            pub struct #return_struct_ident(pub u16, pub #ty );
        },
        syn::ReturnType::Default => quote! {
           #[allow(non_camel_case_types)]
           #[derive(Debug, JetStreamWireFormat)]
           pub struct #return_struct_ident(pub u16);
        },
    }
}

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
    msgs: Vec<(Ident, proc_macro2::TokenStream)>,
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
        pub enum #enum_name {
            #( #msg_variants )*
        }

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
                (mem::size_of::<u32>() + mem::size_of::<u8>() + mem::size_of::<u16>())
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
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
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
                    _ => Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("unknown message type: {}", ty),
                    )),
                }
            }
        }
    }
}

fn generate_tframe(
    tmsgs: Vec<(Ident, proc_macro2::TokenStream)>,
) -> proc_macro2::TokenStream {
    generate_frame(Direction::Tx, tmsgs)
}

fn generate_rframe(
    rmsgs: Vec<(Ident, proc_macro2::TokenStream)>,
) -> proc_macro2::TokenStream {
    generate_frame(Direction::Rx, rmsgs)
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

pub fn generate_jetstream_prococol(
    item: &mut ItemTrait,
    digest: String,
) -> proc_macro2::TokenStream {
    let trait_name: &Ident = &item.ident;
    let original_trait_name = trait_name.clone();
    let mut trait_ident: IdentCased = trait_name.into();
    trait_ident = trait_ident.to_pascale_case();
    let trait_name: Ident = trait_ident.into();
    // rename the trait to have a prefix
    let proto_name =
        Ident::new(&format!("{}Protocol", trait_name), trait_name.span());
    let server_name =
        Ident::new(&format!("{}Server", trait_name), trait_name.span());
    let _client_name =
        Ident::new(&format!("{}Client", trait_name), trait_name.span());
    // rename the trait to have a prefix
    let trait_name =
        Ident::new(&format!("{}Service", trait_name), trait_name.span());

    let mut tmsgs = vec![];
    let mut rmsgs = vec![];
    let mut calls = vec![];
    let mut msg_ids = vec![];
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
                        async fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> #return_struct_ident {
                            #return_struct_ident(tag, #original_trait_name::#method_name(&mut self.inner,
                                #(#spread_req)*
                            )#maybe_await)
                        }
                    }
                } else {
                    quote! {
                        #[inline]
                        async fn #method_name(&mut self,tag: u16, req: #request_struct_ident)-> #return_struct_ident {
                            #return_struct_ident(tag, #original_trait_name::#method_name(&mut self.inner)#maybe_await)
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
                        async fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> #return_struct_ident;
                    }
                } else {
                    quote! {
                        async fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> #return_struct_ident;
                    }
                }
            }
            _ => quote! { #item },
        });

        signatures.extend(with_signatures);

        let match_arms = generate_match_arms(tmsgs.clone().into_iter());
        let match_arm_bodies: Vec<proc_macro2::TokenStream> = item.items.clone().iter().map(|item| match item {
            TraitItem::Fn(method) => {
                let method_name = &method.sig.ident;
                let name: IdentCased = method_name.into();
                let variant_name: Ident = name.to_pascale_case().into();
                quote! {
                     {
                        let msg = #trait_name::#method_name(self,req.tag, msg).await;
                        Ok(Rframe{
                            tag: req.tag,
                            msg: Rmessage::#variant_name(msg)
                        })
                    }
                }
            }
            _ => quote! {},
        }).collect();
        let matches = std::iter::zip(match_arms, match_arm_bodies.iter()).map(
            |(arm, body)| {
                quote! {
                    #arm => #body
                }
            },
        );
        server_calls.extend(iter::once(quote! {
            #[inline]
            async fn rpc(&mut self, req:Self::Request) -> Result<Self::Response,Box<dyn Error + Send + Sync>> {
                match req.msg {
                    #(
                        #matches
                     )*
                }
            }

        }));
    }
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

    let tframe = generate_tframe(tmsgs.clone());
    let rframe = generate_rframe(rmsgs.clone());

    #[allow(clippy::to_string_in_format_args)]
    let protocol_version = format!("pub const PROTOCOL_VERSION: &str = \"dev.branch.jetstream.proto/{}/{}.{}.{}-{}\";",
        original_trait_name.to_string().to_lowercase(),
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH"),
        digest[0..8].to_string()
    );

    // make a const with the digest
    let digest = format!("pub const DIGEST: &str = \"{}\";", digest);
    let digest = parse_str::<Item>(digest.as_str()).unwrap();
    let protocol_version =
        parse_str::<Item>(protocol_version.as_str()).unwrap();
    quote!(
        const MESSAGE_ID_START: u8 = 101;

        #digest

        #protocol_version

        #(
            #msg_ids
        )*

        #(
            #tmsg_definitions
        )*

        #(
            #rmsg_definitions
        )*

        #tframe

        impl Message for Tframe {}

        #rframe

        impl Message for Rframe {}

        pub struct #proto_name;
        impl JetStreamProtocol for #proto_name {
            type Request = Tframe;
            type Response = Rframe;
        }

        #[async_trait::async_trait]
        pub trait #trait_name
            where
             Self: Sized + Send + Sync,
         {
            type Protocol: JetStreamProtocol;
            #(
                #signatures
            )*
        }
        pub struct #server_name<T: #original_trait_name+Send+Sync +?Sized> {
            inner: T
        }
        impl<T: #original_trait_name+Send + Sync + Sized> #server_name<T>
            where Self: Send + Sync + Sized
        {
            pub fn new(inner: T) -> Self {
                Self { inner }
            }
        }
        impl<T:#original_trait_name+Send + Sync + Sized> JetStreamProtocol for #server_name<T> {
            type Request = Tframe;
            type Response = Rframe;
        }
        #[async_trait::async_trait]
        impl<T: #original_trait_name+Send + Sync + Sized> JetStreamAsyncService for #server_name<T>
            where Self: Send + Sync + Sized
        {
            #(
                #server_calls
            )*
        }
        #[async_trait::async_trait]
        impl<T: #original_trait_name+Send + Sync + Sized> #trait_name for #server_name<T>
            where Self: Send + Sync + Sized
         {
            type Protocol = #proto_name;

            #(
                #calls
            )*
        }
    )
}

pub fn protocol_inner(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_mod: ItemMod = parse_macro_input!(item as ItemMod);
    let module_name = item_mod.ident.clone();

    // get all the original items in the module
    let original_mod = item_mod.clone();
    let original_content =
        original_mod.content.as_ref().expect("module content");
    let original_items = &original_content.1;
    let p = quote! { #original_mod }.to_string();
    let dig: String = digest(p);

    let transformed_items = item_mod
        .content
        .as_mut()
        .expect("module content")
        .1
        .iter_mut()
        .map(|trait_item| match trait_item {
            Item::Trait(trait_item) => {
                generate_jetstream_prococol(trait_item, dig.clone())
            }
            _ => quote! {},
        });
    // Construct the final output TokenStream
    let visibility = &item_mod.vis;

    TokenStream::from(quote! {
        #visibility mod #module_name {
            pub use async_trait::async_trait;
            use std::io;
            pub use jetstream::{Message, WireFormat, JetStreamWireFormat, wire_format_extensions::AsyncWireFormatExt, JetStreamProtocol, service::{JetStreamAsyncService}};
            pub use std::mem;
            pub use std::io::{Read, Write, ErrorKind};
            pub use std::future::Future;
            pub use std::pin::Pin;
            pub use std::error::Error;
            pub use super::*;

            #(#transformed_items)*

            #(#original_items)*
        }
    })
}
