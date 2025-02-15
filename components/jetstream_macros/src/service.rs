use {
    proc_macro2::{Literal, TokenStream},
    quote::{format_ident, quote, ToTokens},
    syn::{Ident, ItemTrait, TraitItem},
};

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
        let converter = convert_case::Converter::new().to_case(convert_case::Case::Title);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    #[allow(dead_code)]
    fn to_upper_case(&self) -> Self {
        let converter = convert_case::Converter::new().to_case(convert_case::Case::Upper);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    fn to_screaming_snake_case(&self) -> Self {
        let converter = convert_case::Converter::new().to_case(convert_case::Case::ScreamingSnake);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    fn to_pascal_case(&self) -> Self {
        let converter = convert_case::Converter::new().to_case(convert_case::Case::Pascal);
        let converted = converter.convert(self.0.to_string());
        IdentCased(Ident::new(&converted, self.0.span()))
    }
    #[allow(dead_code)]
    fn to_upper_flat(&self) -> Self {
        let converter = convert_case::Converter::new().to_case(convert_case::Case::UpperFlat);
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

fn generate_tframe(tmsgs: &[(Ident, proc_macro2::TokenStream)]) -> proc_macro2::TokenStream {
    generate_frame(Direction::Tx, tmsgs)
}

fn generate_rframe(rmsgs: &[(Ident, proc_macro2::TokenStream)]) -> proc_macro2::TokenStream {
    generate_frame(Direction::Rx, rmsgs)
}

fn generate_msg_id(index: usize, method_name: &Ident) -> proc_macro2::TokenStream {
    let upper_cased_method_name = method_name.to_string().to_uppercase();
    let tmsg_const_name = Ident::new(&format!("T{}", upper_cased_method_name), method_name.span());
    let rmsg_const_name = Ident::new(&format!("R{}", upper_cased_method_name), method_name.span());
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
    let inputs = method_sig.inputs.iter().map(|arg| {
        match arg {
            syn::FnArg::Typed(pat) => {
                let name = pat.pat.clone();
                let ty = pat.ty.clone();
                quote! {
                    pub #name: #ty,
                }
            }
            syn::FnArg::Receiver(_) => quote! {},
        }
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
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(success_type)) =
                                    args.args.first()
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
                _ => {
                    quote! {
                        #[allow(non_camel_case_types)]
                        #[derive(Debug, JetStreamWireFormat)]
                        pub struct #return_struct_ident(pub #ty);
                    }
                }
            }
        }
        syn::ReturnType::Default => {
            quote! {
               #[allow(non_camel_case_types)]
               #[derive(Debug, JetStreamWireFormat)]
               pub struct #return_struct_ident;
            }
        }
    }
}

fn generate_match_arms(
    tmsgs: impl Iterator<Item = (Ident, proc_macro2::TokenStream)>,
) -> impl Iterator<Item = proc_macro2::TokenStream> {
    tmsgs.map(|(ident, _)| {
        let name: IdentCased = (&ident).into();
        let variant_name: Ident = name.remove_prefix().to_pascal_case().into();
        quote! {
            Tmessage::#variant_name(msg)
        }
    })
}
fn handle_receiver(recv: &syn::Receiver) -> proc_macro2::TokenStream {
    let mutability = &recv.mutability;
    let reference = &recv.reference;

    match (reference, mutability) {
        (Some(_), Some(_)) => quote! { &mut self.inner, },
        (Some(_), None) => quote! { &self.inner, },
        (None, _) => quote! { self.inner, },
    }
}
pub(crate) fn service_impl(item: ItemTrait, is_async_trait: bool) -> TokenStream {
    let trait_name = &item.ident;
    let trait_items = &item.items;
    let vis = &item.vis;

    // Generate message structs and enum variants
    // let mut message_structs = Vec::new();
    let mut tmsgs = Vec::new();
    let mut rmsgs = Vec::new();
    let mut msg_ids = Vec::new();
    let service_name = format_ident!("{}Service", trait_name);
    let channel_name = format_ident!("{}Channel", trait_name);
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
    let tag_name = format_ident!("{}_TAG", trait_name.to_string().to_uppercase());

    let mut server_calls = vec![];

    {
        let with_calls = item.items.iter().enumerate().map(|(index, item)| {
            if let TraitItem::Fn(method) = item {
                let method_name = &method.sig.ident;

                let request_struct_ident =
                    Ident::new(&format!("T{}", method_name), method_name.span());
                let return_struct_ident =
                    Ident::new(&format!("R{}", method_name), method_name.span());
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

                tmsgs.insert(
                    index,
                    (request_struct_ident.clone(), request_struct.clone()),
                );
                rmsgs.insert(index, (return_struct_ident.clone(), return_struct.clone()));
            }
        });
        calls.extend(with_calls);
    }
    let mut client_calls = vec![];
    {
        item.items.iter().enumerate().for_each(|(index,item)|{
            let TraitItem::Fn(method) = item else {return;};
            let method_name = &method.sig.ident;
            let has_req = method.sig.inputs.iter().count() > 1;
            let is_async = method.sig.asyncness.is_some();
            let maybe_await = if is_async { quote! { .await } } else { quote! {} };

            let request_struct_ident = tmsgs.get(index).unwrap().0.clone();
            let return_struct_ident = rmsgs.get(index).unwrap().0.clone();
            let new = if has_req {
                let spread_req = method.sig.inputs.iter().map(|arg| match arg {
                    syn::FnArg::Typed(pat) => {
                        let name = pat.pat.clone();
                        quote! { req.#name, }
                    }
                    syn::FnArg::Receiver(_) => quote! {},
                });
                quote! {
                    fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> impl ::core::future::Future<
                        Output = Result<#return_struct_ident, Error,
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
                    fn #method_name(&mut self, tag: u16, req: #request_struct_ident) -> impl ::core::future::Future<
                        Output = Result<#return_struct_ident, Error,
                    > + Send + Sync {
                        Box::pin(async move {
                            #return_struct_ident(tag, #trait_name::#method_name(&mut self.inner)#maybe_await)
                        })
                    }
                    }
                };
            server_calls.extend(new);
        });
    }
    {
        item.items.iter().enumerate().for_each(|(index, item)| {
            let TraitItem::Fn(method) = item else {
                return;
            };
            let method_name = &method.sig.ident;
            let variant_name: Ident = IdentCased(method_name.clone()).to_pascal_case().into();
            let retn = &method.sig.output;
            let is_async = method.sig.asyncness.is_some();
            let maybe_async = if is_async {
                quote! { async }
            } else {
                quote! {}
            };
            let request_struct_ident = tmsgs.get(index).unwrap().0.clone();
            let inputs = method.sig.inputs.iter().map(|arg| {
                match arg {
                    syn::FnArg::Typed(pat) => {
                        let name = pat.pat.clone();
                        let ty = pat.ty.clone();
                        quote! {
                             #name: #ty,
                        }
                    }
                    syn::FnArg::Receiver(_) => quote! {},
                }
            });
            let args = method.sig.inputs.iter().map(|arg| {
                match arg {
                    syn::FnArg::Typed(pat) => {
                        let name = pat.pat.clone();
                        quote! {
                             #name,
                        }
                    }
                    syn::FnArg::Receiver(_) => quote! {},
                }
            });
            let new = quote! {
                #maybe_async fn #method_name(&mut self, #(#inputs)*)  #retn {
                    let tag =#tag_name.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let req = Tmessage::#variant_name(#request_struct_ident {
                        #(
                            #args
                        )*
                    });
                    let tframe= Frame::from((tag, req));
                    let rframe = self.rpc(tframe).await?;
                    let rmsg = rframe.msg;
                    match rmsg {
                        Rmessage::#variant_name(msg) => Ok(msg.0),
                        _ => Err(Error::InvalidResponse),
                    }
                }
            };

            client_calls.extend(new);
        });
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
    let proto_mod = format_ident!("{}_protocol", trait_name.to_string().to_lowercase());

    let match_arms = generate_match_arms(tmsgs.clone().into_iter());
    let match_arm_bodies: Vec<proc_macro2::TokenStream> = item
        .items
        .clone()
        .iter()
        .map(|item| {
            match item {
                TraitItem::Fn(method) => {
                    let method_name = &method.sig.ident;
                    let name: IdentCased = method_name.into();
                    let variant_name: Ident = name.to_pascal_case().into();
                    let return_struct_ident =
                        Ident::new(&format!("R{}", method_name), method_name.span());
                    let variables_spead = method.sig.inputs.iter().map(|arg| {
                        match arg {
                            syn::FnArg::Typed(pat) => {
                                let name = pat.pat.clone();
                                quote! { msg.#name, }
                            }
                            syn::FnArg::Receiver(recv) => handle_receiver(recv),
                        }
                    });
                    quote! {
                         {
                            let msg = #trait_name::#method_name(
                                #(
                                    #variables_spead
                                )*
                            ).await?;
                            let ret = #return_struct_ident(msg);
                            Ok(Rmessage::#variant_name(ret))
                        }
                    }
                }
                _ => quote! {},
            }
        })
        .collect();
    let matches = std::iter::zip(match_arms, match_arm_bodies.iter()).map(|(arm, body)| {
        quote! {
            #arm => #body
        }
    });

    let trait_attribute = if is_async_trait {
        quote! { #[jetstream::prelude::async_trait] }
    } else {
        quote! { #[jetstream::prelude::trait_variant::make(Send + Sync)] }
    };
    quote! {
        #vis mod #proto_mod{
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

            #rmessage

            #[derive(Clone)]
            pub struct #service_name<T: #trait_name> {
                pub inner: T,
            }

            impl<T> Protocol for #service_name<T>
            where
                T: #trait_name+ Send + Sync + Sized
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
                        let res: Result<<Self as Protocol>::Response, Self::Error> =match req {
                                #(
                                    #matches
                                )*
                        };
                        let rframe: Frame<<Self as Protocol>::Response> = Frame::from((frame.tag, res?));
                        Ok(rframe)
                    })
                }
            }
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

        #trait_attribute
        #vis trait #trait_name {
            #(#trait_items)*
        }
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use {super::*, syn::parse_quote};

    fn run_test_with_filters<F>(test_fn: F)
    where
        F: FnOnce() + panic::UnwindSafe,
    {
        let filters = vec![
            // Filter for protocol version strings
            (
                r"dev\.branch\.jetstream\.proto/\w+/\d+\.\d+\.\d+-[a-f0-9]{8}",
                "dev.branch.jetstream.proto/NAME/VERSION-HASH",
            ),
            // Filter for digest strings
            (r"[a-f0-9]{64}", "DIGEST_HASH"),
        ];

        insta::with_settings!({
            filters => filters,
        }, {
            test_fn();
        });
    }

    #[test]
    fn test_simple_service() {
        let input: ItemTrait = parse_quote! {
            pub trait Echo {
                async fn ping(&self) -> Result<(), std::io::Error>;
            }
        };
        let output = service_impl(input, false);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        run_test_with_filters(|| {
            insta::assert_snapshot!(output_str, @r###"
            pub mod echo_protocol {
                use jetstream::prelude::*;
                use std::io::{self, Read, Write};
                use std::mem;
                use super::Echo;
                const MESSAGE_ID_START: u8 = 101;
                pub const PROTOCOL_VERSION: &str = "dev.branch.jetstream.proto/NAME/VERSION-HASH";
                const DIGEST: &str = "DIGEST_HASH";
                pub const TPING: u8 = MESSAGE_ID_START + 0u8;
                pub const RPING: u8 = MESSAGE_ID_START + 0u8 + 1;
                #[allow(non_camel_case_types)]
                #[derive(Debug, JetStreamWireFormat)]
                pub struct Tping {}
                #[allow(non_camel_case_types)]
                #[derive(Debug, JetStreamWireFormat)]
                pub struct Rping(pub ());
                #[derive(Debug)]
                #[repr(u8)]
                pub enum Tmessage {
                    Ping(Tping) = TPING,
                }
                impl Framer for Tmessage {
                    fn byte_size(&self) -> u32 {
                        match &self {
                            Tmessage::Ping(msg) => msg.byte_size(),
                        }
                    }
                    fn message_type(&self) -> u8 {
                        unsafe { *<*const _>::from(self).cast::<u8>() }
                    }
                    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                        match &self {
                            Tmessage::Ping(msg) => msg.encode(writer)?,
                        }
                        Ok(())
                    }
                    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Tmessage> {
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
                #[derive(Debug)]
                #[repr(u8)]
                pub enum Rmessage {
                    Ping(Rping) = RPING,
                }
                impl Framer for Rmessage {
                    fn byte_size(&self) -> u32 {
                        match &self {
                            Rmessage::Ping(msg) => msg.byte_size(),
                        }
                    }
                    fn message_type(&self) -> u8 {
                        unsafe { *<*const _>::from(self).cast::<u8>() }
                    }
                    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                        match &self {
                            Rmessage::Ping(msg) => msg.encode(writer)?,
                        }
                        Ok(())
                    }
                    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Rmessage> {
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
                #[derive(Clone)]
                pub struct EchoService<T: Echo> {
                    pub inner: T,
                }
                impl<T> Protocol for EchoService<T>
                where
                    T: Echo + Send + Sync + Sized,
                {
                    type Request = Tmessage;
                    type Response = Rmessage;
                    type Error = Error;
                    const VERSION: &'static str = PROTOCOL_VERSION;
                    fn rpc(
                        &mut self,
                        frame: Frame<<Self as Protocol>::Request>,
                    ) -> impl ::core::future::Future<
                        Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
                    > + Send + Sync {
                        Box::pin(async move {
                            let req: <Self as Protocol>::Request = frame.msg;
                            let res: Result<<Self as Protocol>::Response, Self::Error> = match req {
                                Tmessage::Ping(msg) => {
                                    let msg = Echo::ping(&self.inner).await?;
                                    let ret = Rping(msg);
                                    Ok(Rmessage::Ping(ret))
                                }
                            };
                            let rframe: Frame<<Self as Protocol>::Response> = Frame::from((
                                frame.tag,
                                res?,
                            ));
                            Ok(rframe)
                        })
                    }
                }
                pub struct EchoChannel<'a> {
                    pub inner: Box<&'a mut dyn ClientTransport<Self>>,
                }
                impl<'a> Protocol for EchoChannel<'a> {
                    type Request = Tmessage;
                    type Response = Rmessage;
                    type Error = Error;
                    const VERSION: &'static str = PROTOCOL_VERSION;
                    fn rpc(
                        &mut self,
                        frame: Frame<<Self as Protocol>::Request>,
                    ) -> impl ::core::future::Future<
                        Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
                    > + Send + Sync {
                        use futures::{SinkExt, StreamExt};
                        Box::pin(async move {
                            self.inner.send(frame).await?;
                            let frame = self.inner.next().await.unwrap()?;
                            Ok(frame)
                        })
                    }
                }
                lazy_static::lazy_static! {
                    static ref ECHO_TAG : std::sync::atomic::AtomicU16 =
                    std::sync::atomic::AtomicU16::new(0);
                }
                impl<'a> Echo for EchoChannel<'a> {
                    async fn ping(&mut self) -> Result<(), std::io::Error> {
                        let tag = ECHO_TAG.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let req = Tmessage::Ping(Tping {});
                        let tframe = Frame::from((tag, req));
                        let rframe = self.rpc(tframe).await?;
                        let rmsg = rframe.msg;
                        match rmsg {
                            Rmessage::Ping(msg) => Ok(msg.0),
                            _ => Err(Error::InvalidResponse),
                        }
                    }
                }
            }
            #[jetstream::prelude::trait_variant::make(Send+Sync)]
            pub trait Echo {
                async fn ping(&self) -> Result<(), std::io::Error>;
            }
            "###)
        })
    }

    #[test]
    fn test_service_with_args() {
        let input: ItemTrait = parse_quote! {
            pub trait Echo {
                async fn ping(&self, message: String) -> Result<String, std::io::Error>;
            }
        };
        let output = service_impl(input, false);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        run_test_with_filters(|| {
            insta::assert_snapshot!(output_str, @r###"
            pub mod echo_protocol {
                use jetstream::prelude::*;
                use std::io::{self, Read, Write};
                use std::mem;
                use super::Echo;
                const MESSAGE_ID_START: u8 = 101;
                pub const PROTOCOL_VERSION: &str = "dev.branch.jetstream.proto/NAME/VERSION-HASH";
                const DIGEST: &str = "DIGEST_HASH";
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
                #[repr(u8)]
                pub enum Tmessage {
                    Ping(Tping) = TPING,
                }
                impl Framer for Tmessage {
                    fn byte_size(&self) -> u32 {
                        match &self {
                            Tmessage::Ping(msg) => msg.byte_size(),
                        }
                    }
                    fn message_type(&self) -> u8 {
                        unsafe { *<*const _>::from(self).cast::<u8>() }
                    }
                    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                        match &self {
                            Tmessage::Ping(msg) => msg.encode(writer)?,
                        }
                        Ok(())
                    }
                    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Tmessage> {
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
                #[derive(Debug)]
                #[repr(u8)]
                pub enum Rmessage {
                    Ping(Rping) = RPING,
                }
                impl Framer for Rmessage {
                    fn byte_size(&self) -> u32 {
                        match &self {
                            Rmessage::Ping(msg) => msg.byte_size(),
                        }
                    }
                    fn message_type(&self) -> u8 {
                        unsafe { *<*const _>::from(self).cast::<u8>() }
                    }
                    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                        match &self {
                            Rmessage::Ping(msg) => msg.encode(writer)?,
                        }
                        Ok(())
                    }
                    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Rmessage> {
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
                #[derive(Clone)]
                pub struct EchoService<T: Echo> {
                    pub inner: T,
                }
                impl<T> Protocol for EchoService<T>
                where
                    T: Echo + Send + Sync + Sized,
                {
                    type Request = Tmessage;
                    type Response = Rmessage;
                    type Error = Error;
                    const VERSION: &'static str = PROTOCOL_VERSION;
                    fn rpc(
                        &mut self,
                        frame: Frame<<Self as Protocol>::Request>,
                    ) -> impl ::core::future::Future<
                        Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
                    > + Send + Sync {
                        Box::pin(async move {
                            let req: <Self as Protocol>::Request = frame.msg;
                            let res: Result<<Self as Protocol>::Response, Self::Error> = match req {
                                Tmessage::Ping(msg) => {
                                    let msg = Echo::ping(&self.inner, msg.message).await?;
                                    let ret = Rping(msg);
                                    Ok(Rmessage::Ping(ret))
                                }
                            };
                            let rframe: Frame<<Self as Protocol>::Response> = Frame::from((
                                frame.tag,
                                res?,
                            ));
                            Ok(rframe)
                        })
                    }
                }
                pub struct EchoChannel<'a> {
                    pub inner: Box<&'a mut dyn ClientTransport<Self>>,
                }
                impl<'a> Protocol for EchoChannel<'a> {
                    type Request = Tmessage;
                    type Response = Rmessage;
                    type Error = Error;
                    const VERSION: &'static str = PROTOCOL_VERSION;
                    fn rpc(
                        &mut self,
                        frame: Frame<<Self as Protocol>::Request>,
                    ) -> impl ::core::future::Future<
                        Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
                    > + Send + Sync {
                        use futures::{SinkExt, StreamExt};
                        Box::pin(async move {
                            self.inner.send(frame).await?;
                            let frame = self.inner.next().await.unwrap()?;
                            Ok(frame)
                        })
                    }
                }
                lazy_static::lazy_static! {
                    static ref ECHO_TAG : std::sync::atomic::AtomicU16 =
                    std::sync::atomic::AtomicU16::new(0);
                }
                impl<'a> Echo for EchoChannel<'a> {
                    async fn ping(&mut self, message: String) -> Result<String, std::io::Error> {
                        let tag = ECHO_TAG.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let req = Tmessage::Ping(Tping { message });
                        let tframe = Frame::from((tag, req));
                        let rframe = self.rpc(tframe).await?;
                        let rmsg = rframe.msg;
                        match rmsg {
                            Rmessage::Ping(msg) => Ok(msg.0),
                            _ => Err(Error::InvalidResponse),
                        }
                    }
                }
            }
            #[jetstream::prelude::trait_variant::make(Send+Sync)]
            pub trait Echo {
                async fn ping(&self, message: String) -> Result<String, std::io::Error>;
            }
            "###)
        })
    }

    #[test]
    fn test_async_trait_service_with_args() {
        let input: ItemTrait = parse_quote! {
            pub trait Echo {
                async fn ping(&mut self, message: String) -> Result<String, std::io::Error>;
            }
        };
        let output = service_impl(input, true);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        run_test_with_filters(|| {
            insta::assert_snapshot!(output_str, @r###"
            pub mod echo_protocol {
                use jetstream::prelude::*;
                use std::io::{self, Read, Write};
                use std::mem;
                use super::Echo;
                const MESSAGE_ID_START: u8 = 101;
                pub const PROTOCOL_VERSION: &str = "dev.branch.jetstream.proto/NAME/VERSION-HASH";
                const DIGEST: &str = "DIGEST_HASH";
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
                #[repr(u8)]
                pub enum Tmessage {
                    Ping(Tping) = TPING,
                }
                impl Framer for Tmessage {
                    fn byte_size(&self) -> u32 {
                        match &self {
                            Tmessage::Ping(msg) => msg.byte_size(),
                        }
                    }
                    fn message_type(&self) -> u8 {
                        unsafe { *<*const _>::from(self).cast::<u8>() }
                    }
                    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                        match &self {
                            Tmessage::Ping(msg) => msg.encode(writer)?,
                        }
                        Ok(())
                    }
                    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Tmessage> {
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
                #[derive(Debug)]
                #[repr(u8)]
                pub enum Rmessage {
                    Ping(Rping) = RPING,
                }
                impl Framer for Rmessage {
                    fn byte_size(&self) -> u32 {
                        match &self {
                            Rmessage::Ping(msg) => msg.byte_size(),
                        }
                    }
                    fn message_type(&self) -> u8 {
                        unsafe { *<*const _>::from(self).cast::<u8>() }
                    }
                    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
                        match &self {
                            Rmessage::Ping(msg) => msg.encode(writer)?,
                        }
                        Ok(())
                    }
                    fn decode<R: Read>(reader: &mut R, ty: u8) -> io::Result<Rmessage> {
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
                #[derive(Clone)]
                pub struct EchoService<T: Echo> {
                    pub inner: T,
                }
                impl<T> Protocol for EchoService<T>
                where
                    T: Echo + Send + Sync + Sized,
                {
                    type Request = Tmessage;
                    type Response = Rmessage;
                    type Error = Error;
                    const VERSION: &'static str = PROTOCOL_VERSION;
                    fn rpc(
                        &mut self,
                        frame: Frame<<Self as Protocol>::Request>,
                    ) -> impl ::core::future::Future<
                        Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
                    > + Send + Sync {
                        Box::pin(async move {
                            let req: <Self as Protocol>::Request = frame.msg;
                            let res: Result<<Self as Protocol>::Response, Self::Error> = match req {
                                Tmessage::Ping(msg) => {
                                    let msg = Echo::ping(&mut self.inner, msg.message).await?;
                                    let ret = Rping(msg);
                                    Ok(Rmessage::Ping(ret))
                                }
                            };
                            let rframe: Frame<<Self as Protocol>::Response> = Frame::from((
                                frame.tag,
                                res?,
                            ));
                            Ok(rframe)
                        })
                    }
                }
                pub struct EchoChannel<'a> {
                    pub inner: Box<&'a mut dyn ClientTransport<Self>>,
                }
                impl<'a> Protocol for EchoChannel<'a> {
                    type Request = Tmessage;
                    type Response = Rmessage;
                    type Error = Error;
                    const VERSION: &'static str = PROTOCOL_VERSION;
                    fn rpc(
                        &mut self,
                        frame: Frame<<Self as Protocol>::Request>,
                    ) -> impl ::core::future::Future<
                        Output = Result<Frame<<Self as Protocol>::Response>, Self::Error>,
                    > + Send + Sync {
                        use futures::{SinkExt, StreamExt};
                        Box::pin(async move {
                            self.inner.send(frame).await?;
                            let frame = self.inner.next().await.unwrap()?;
                            Ok(frame)
                        })
                    }
                }
                lazy_static::lazy_static! {
                    static ref ECHO_TAG : std::sync::atomic::AtomicU16 =
                    std::sync::atomic::AtomicU16::new(0);
                }
                impl<'a> Echo for EchoChannel<'a> {
                    async fn ping(&mut self, message: String) -> Result<String, std::io::Error> {
                        let tag = ECHO_TAG.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let req = Tmessage::Ping(Tping { message });
                        let tframe = Frame::from((tag, req));
                        let rframe = self.rpc(tframe).await?;
                        let rmsg = rframe.msg;
                        match rmsg {
                            Rmessage::Ping(msg) => Ok(msg.0),
                            _ => Err(Error::InvalidResponse),
                        }
                    }
                }
            }
            #[jetstream::prelude::async_trait]
            pub trait Echo {
                async fn ping(&mut self, message: String) -> Result<String, std::io::Error>;
            }
            "###)
        })
    }
}
