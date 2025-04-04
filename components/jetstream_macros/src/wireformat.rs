use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DeriveInput, Fields, Ident, Meta};

fn has_skip_attr(field: &syn::Field) -> bool {
    field.attrs.iter().any(|attr| {
        if attr.path().is_ident("jetstream") {
            if let Ok(()) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    Ok(())
                } else {
                    Err(meta.error("expected `skip`"))
                }
            }) {
                return true;
            }
        }
        false
    })
}

fn extract_jetstream_type(input: &DeriveInput) -> Option<Ident> {
    for attr in &input.attrs {
        if attr.path().is_ident("jetstream_type") {
            if let Ok(Meta::Path(path)) = attr.parse_args() {
                if let Some(ident) = path.get_ident() {
                    return Some(ident.clone());
                }
            }
        }
    }
    None
}

pub(crate) fn wire_format_inner(input: DeriveInput) -> TokenStream {
    if !input.generics.params.is_empty() {
        return quote! {
            compile_error!("derive(JetStreamWireFormat) does not support generic parameters");
        };
    }
    let jetstream_type = extract_jetstream_type(&input);
    let container = input.ident;

    // Generate message type implementation
    let message_impl = if let Some(msg_type) = jetstream_type {
        quote! {
           impl jetstream_wireformat::Message for #container {
               const MESSAGE_TYPE: u8 = super::#msg_type;
           }
        }
    } else {
        quote! {}
    };

    let byte_size_impl = byte_size_sum(&input.data);
    let encode_impl = encode_wire_format(&input.data);
    let decode_impl = decode_wire_format(&input.data, &container);

    let scope = format!("wire_format_{}", container).to_lowercase();
    let scope = Ident::new(&scope, Span::call_site());
    quote! {
        mod #scope {
            extern crate std;
            use self::std::io;
            use self::std::result::Result::Ok;
            use super::#container;
            use jetstream_wireformat::WireFormat;

            impl WireFormat for #container {
                fn byte_size(&self) -> u32 {
                    #byte_size_impl
                }

                fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                    #encode_impl
                }

                fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                    #decode_impl
                }
            }
            #message_impl
        }
    }
}

fn byte_size_sum(data: &Data) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let fields =
                fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                    let field = &f.ident;
                    let span = field.span();
                    quote_spanned! {span=>
                        WireFormat::byte_size(&self.#field)
                    }
                });

            quote! {
                0 #(+ #fields)*
            }
        } else if let Fields::Unnamed(unnamed) = &data.fields {
            let fields = unnamed
                .unnamed
                .iter()
                .enumerate()
                .filter(|(_, f)| !has_skip_attr(f))
                .map(|(i, _f)| {
                    let index = syn::Index::from(i);
                    quote! {
                        WireFormat::byte_size(&self.#index)
                    }
                });

            quote! {
                0 #(+ #fields)*
            }
        } else {
            unimplemented!();
        }
    } else if let Data::Enum(ref data) = *data {
        let variants = data.variants.iter().map(|variant| {
            let variant_ident = &variant.ident;
            match &variant.fields {
                Fields::Named(fields) => {
                    let field_idents = fields
                        .named
                        .iter()
                        .filter(|f| !has_skip_attr(f))
                        .map(|f| &f.ident)
                        .collect::<Vec<_>>();
                    quote! {
                        Self::#variant_ident { #(ref #field_idents),* } => {
                            1 #(+ WireFormat::byte_size(#field_idents))*
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let refs = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| !has_skip_attr(f))
                        .map(|(i, _)| format!("__{}", i))
                        .map(|name| Ident::new(&name, Span::call_site()))
                        .collect::<Vec<_>>();
                    quote! {
                        Self::#variant_ident(#(ref #refs),*) => {
                            1 #(+ WireFormat::byte_size(#refs))*
                        }
                    }
                }
                Fields::Unit => {
                    quote! {
                        Self::#variant_ident => 1
                    }
                }
            }
        });

        quote! {
            match self {
                #(#variants),*
            }
        }
    } else {
        unimplemented!();
    }
}

fn encode_wire_format(data: &Data) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let fields =
                fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                    let field = &f.ident;
                    let span = field.span();
                    quote_spanned! {span=>
                        WireFormat::encode(&self.#field, _writer)?;
                    }
                });

            quote! {
                #(#fields)*
                Ok(())
            }
        } else if let Fields::Unnamed(unnamed) = &data.fields {
            let fields = unnamed
                .unnamed
                .iter()
                .enumerate()
                .filter(|(_, f)| !has_skip_attr(f))
                .map(|(i, _f)| {
                    let index = syn::Index::from(i);
                    quote! {
                        WireFormat::encode(&self.#index, _writer)?;
                    }
                });

            quote! {
                #(#fields)*
                Ok(())
            }
        } else {
            unimplemented!();
        }
    } else if let Data::Enum(ref data) = *data {
        let variants =
            data.variants.iter().enumerate().map(|(idx, variant)| {
                let variant_ident = &variant.ident;
                let idx = idx as u8;

                match &variant.fields {
                    Fields::Named(ref fields) => {
                        let field_idents = fields
                            .named
                            .iter()
                            .filter(|f| !has_skip_attr(f))
                            .map(|f| &f.ident)
                            .collect::<Vec<_>>();

                        quote! {
                            Self::#variant_ident { #(ref #field_idents),* } => {
                                WireFormat::encode(&(#idx), _writer)?;
                                #(WireFormat::encode(#field_idents, _writer)?;)*
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let field_refs = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .filter(|(_, f)| !has_skip_attr(f))
                            .map(|(i, _)| format!("__{}", i))
                            .map(|name| Ident::new(&name, Span::call_site()))
                            .collect::<Vec<_>>();
                        quote! {
                            Self::#variant_ident(#(ref #field_refs),*) => {
                                WireFormat::encode(&(#idx), _writer)?;
                                #(WireFormat::encode(#field_refs, _writer)?;)*
                            }
                        }
                    }
                    Fields::Unit => {
                        quote! {
                            Self::#variant_ident => {
                                WireFormat::encode(&(#idx), _writer)?;
                            }
                        }
                    }
                }
            });

        quote! {
            match self {
                #(#variants),*
            }
            Ok(())
        }
    } else {
        unimplemented!();
    }
}

fn decode_wire_format(data: &Data, container: &Ident) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let all_fields = fields.named.iter().collect::<Vec<_>>();
            let non_skipped_values =
                fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                    let field = &f.ident;
                    let span = field.span();
                    quote_spanned! {span=>
                        let #field = WireFormat::decode(_reader)?;
                    }
                });

            let members = all_fields.iter().map(|f| {
                let field = &f.ident;
                if has_skip_attr(f) {
                    quote! {
                        #field: Default::default(),
                    }
                } else {
                    quote! {
                        #field: #field,
                    }
                }
            });

            quote! {
                #(#non_skipped_values)*
                Ok(#container {
                    #(#members)*
                })
            }
        } else if let Fields::Unnamed(unnamed) = &data.fields {
            let all_fields = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| (i, has_skip_attr(f)))
                .collect::<Vec<_>>();

            let non_skipped_values = unnamed
                .unnamed
                .iter()
                .enumerate()
                .filter(|(_, f)| !has_skip_attr(f))
                .map(|(i, _f)| {
                    let ident =
                        Ident::new(&format!("__{}", i), Span::call_site());
                    quote! {
                        let #ident = WireFormat::decode(_reader)?;
                    }
                });

            let members = all_fields.iter().map(|(i, is_skipped)| {
                let ident = if *is_skipped {
                    quote! { Default::default() }
                } else {
                    let ident =
                        Ident::new(&format!("__{}", i), Span::call_site());
                    quote! { #ident }
                };
                quote! { #ident }
            });

            quote! {
                #(#non_skipped_values)*
                Ok(#container(
                    #(#members,)*
                ))
            }
        } else {
            unimplemented!();
        }
    } else if let Data::Enum(ref data) = *data {
        let mut variant_matches = data
            .variants
            .iter()
            .enumerate()
            .map(|(idx, variant)| {
                let variant_ident = &variant.ident;
                let idx = idx as u8;

                match &variant.fields {
                    Fields::Named(ref fields) => {
                        let field_decodes =
                            fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                                let field_ident = &f.ident;
                                quote! { let #field_ident = WireFormat::decode(_reader)?; }
                            });
                        let field_names = fields.named.iter().map(|f| {
                            let field_ident = &f.ident;
                            if has_skip_attr(f) {
                                quote! { #field_ident: Default::default() }
                            } else {
                                // Just use the field name directly for the shorthand syntax
                                quote! { #field_ident }
                            }
                        });

                        quote! {
                            #idx => {
                                #(#field_decodes)*
                                Ok(Self::#variant_ident { #(#field_names),* })
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let field_decodes = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .filter(|(_, f)| !has_skip_attr(f))
                            .map(|(i, _)| {
                                let field_name = Ident::new(&format!("__{}", i), Span::call_site());
                                quote! { let #field_name = WireFormat::decode(_reader)?; }
                            });
                        let field_names = fields.unnamed.iter().enumerate().map(|(i, f)| {
                            if has_skip_attr(f) {
                                quote! { Default::default() }
                            } else {
                                let field_name = Ident::new(&format!("__{}", i), Span::call_site());
                                quote! { #field_name }
                            }
                        });

                        quote! {
                            #idx => {
                                #(#field_decodes)*
                                Ok(Self::#variant_ident(#(#field_names),*))
                            }
                        }
                    }
                    Fields::Unit => {
                        quote! {
                            #idx => Ok(Self::#variant_ident)
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        variant_matches.push(quote! {
              _ => Err(::std::io::Error::new(::std::io::ErrorKind::InvalidData, "invalid variant index"))
          });

        quote! {
            let variant_index: u8 = WireFormat::decode(_reader)?;
            match variant_index {
                #(#variant_matches),*
            }
        }
    } else {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    extern crate pretty_assertions;
    use syn::parse_quote;

    use self::pretty_assertions::assert_eq;
    use super::*;

    #[test]
    fn byte_size() {
        let input: DeriveInput = parse_quote! {
            struct Item {
                ident: u32,
                with_underscores: String,
                other: u8,
            }
        };

        let expected = quote! {
            0
                + WireFormat::byte_size(&self.ident)
                + WireFormat::byte_size(&self.with_underscores)
                + WireFormat::byte_size(&self.other)
        };

        assert_eq!(
            byte_size_sum(&input.data).to_string(),
            expected.to_string()
        );
    }

    #[test]
    fn encode() {
        let input: DeriveInput = parse_quote! {
            struct Item {
                ident: u32,
                with_underscores: String,
                other: u8,
            }
        };

        let expected = quote! {
            WireFormat::encode(&self.ident, _writer)?;
            WireFormat::encode(&self.with_underscores, _writer)?;
            WireFormat::encode(&self.other, _writer)?;
            Ok(())
        };

        assert_eq!(
            encode_wire_format(&input.data).to_string(),
            expected.to_string(),
        );
    }

    #[test]
    fn decode() {
        let input: DeriveInput = parse_quote! {
            struct Item {
                ident: u32,
                with_underscores: String,
                other: u8,
            }
        };

        let container = Ident::new("Item", Span::call_site());
        let expected = quote! {
            let ident = WireFormat::decode(_reader)?;
            let with_underscores = WireFormat::decode(_reader)?;
            let other = WireFormat::decode(_reader)?;
            Ok(Item {
                ident: ident,
                with_underscores: with_underscores,
                other: other,
            })
        };

        assert_eq!(
            decode_wire_format(&input.data, &container).to_string(),
            expected.to_string(),
        );
    }

    #[test]
    fn end_to_end() {
        let input: DeriveInput = parse_quote! {
            struct Niijima_先輩 {
                a: u8,
                b: u16,
                c: u32,
                d: u64,
                e: String,
                f: Vec<String>,
                g: Nested,
            }
        };
        let output = wire_format_inner(input);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        insta::assert_snapshot!(output_str, @r###"
        mod wire_format_niijima_先輩 {
            extern crate std;
            use self::std::io;
            use self::std::result::Result::Ok;
            use super::Niijima_先輩;
            use jetstream_wireformat::WireFormat;
            impl WireFormat for Niijima_先輩 {
                fn byte_size(&self) -> u32 {
                    0 + WireFormat::byte_size(&self.a) + WireFormat::byte_size(&self.b)
                        + WireFormat::byte_size(&self.c) + WireFormat::byte_size(&self.d)
                        + WireFormat::byte_size(&self.e) + WireFormat::byte_size(&self.f)
                        + WireFormat::byte_size(&self.g)
                }
                fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                    WireFormat::encode(&self.a, _writer)?;
                    WireFormat::encode(&self.b, _writer)?;
                    WireFormat::encode(&self.c, _writer)?;
                    WireFormat::encode(&self.d, _writer)?;
                    WireFormat::encode(&self.e, _writer)?;
                    WireFormat::encode(&self.f, _writer)?;
                    WireFormat::encode(&self.g, _writer)?;
                    Ok(())
                }
                fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                    let a = WireFormat::decode(_reader)?;
                    let b = WireFormat::decode(_reader)?;
                    let c = WireFormat::decode(_reader)?;
                    let d = WireFormat::decode(_reader)?;
                    let e = WireFormat::decode(_reader)?;
                    let f = WireFormat::decode(_reader)?;
                    let g = WireFormat::decode(_reader)?;
                    Ok(Niijima_先輩 {
                        a: a,
                        b: b,
                        c: c,
                        d: d,
                        e: e,
                        f: f,
                        g: g,
                    })
                }
            }
        }
        "###);
    }

    #[test]
    fn end_to_end_unnamed() {
        let input: DeriveInput = parse_quote! {
            struct Niijima_先輩(u8, u16, u32, u64, String, Vec<String>, Nested);
        };

        let output = wire_format_inner(input);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        insta::assert_snapshot!(output_str, @r###"
        mod wire_format_niijima_先輩 {
            extern crate std;
            use self::std::io;
            use self::std::result::Result::Ok;
            use super::Niijima_先輩;
            use jetstream_wireformat::WireFormat;
            impl WireFormat for Niijima_先輩 {
                fn byte_size(&self) -> u32 {
                    0 + WireFormat::byte_size(&self.0) + WireFormat::byte_size(&self.1)
                        + WireFormat::byte_size(&self.2) + WireFormat::byte_size(&self.3)
                        + WireFormat::byte_size(&self.4) + WireFormat::byte_size(&self.5)
                        + WireFormat::byte_size(&self.6)
                }
                fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                    WireFormat::encode(&self.0, _writer)?;
                    WireFormat::encode(&self.1, _writer)?;
                    WireFormat::encode(&self.2, _writer)?;
                    WireFormat::encode(&self.3, _writer)?;
                    WireFormat::encode(&self.4, _writer)?;
                    WireFormat::encode(&self.5, _writer)?;
                    WireFormat::encode(&self.6, _writer)?;
                    Ok(())
                }
                fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                    let __0 = WireFormat::decode(_reader)?;
                    let __1 = WireFormat::decode(_reader)?;
                    let __2 = WireFormat::decode(_reader)?;
                    let __3 = WireFormat::decode(_reader)?;
                    let __4 = WireFormat::decode(_reader)?;
                    let __5 = WireFormat::decode(_reader)?;
                    let __6 = WireFormat::decode(_reader)?;
                    Ok(Niijima_先輩(__0, __1, __2, __3, __4, __5, __6))
                }
            }
        }
        "###);
    }

    #[test]
    fn enum_byte_size() {
        let input: DeriveInput = parse_quote! {
            enum Message {
                Ping,
                Text { content: String },
                Binary(Vec<u8>),
            }
        };

        let expected = quote! {
            match self {
                Self::Ping => 1,
                Self::Text { ref content } => { 1 + WireFormat::byte_size(content) },
                Self::Binary(ref __0) => { 1 + WireFormat::byte_size(__0) }
            }
        };

        assert_eq!(
            byte_size_sum(&input.data).to_string(),
            expected.to_string()
        );
    }

    #[test]
    fn enum_encode() {
        let input: DeriveInput = parse_quote! {
            enum Message {
                Ping,
                Text { content: String },
                Binary(Vec<u8>),
            }
        };

        let expected = quote! {
            match self {
                Self::Ping => {
                    WireFormat::encode(&(0u8), _writer)?;
                },
                Self::Text { ref content } => {
                    WireFormat::encode(&(1u8), _writer)?;
                    WireFormat::encode(content, _writer)?;
                },
                Self::Binary(ref __0) => {
                    WireFormat::encode(&(2u8), _writer)?;
                    WireFormat::encode(__0, _writer)?;
                }
            }
            Ok(())
        };

        assert_eq!(
            encode_wire_format(&input.data).to_string(),
            expected.to_string()
        );
    }

    #[test]
    fn enum_decode() {
        let input: DeriveInput = parse_quote! {
            enum Message {
                Ping,
                Text { content: String },
                Binary(Vec<u8>),
            }
        };

        let container = Ident::new("Message", Span::call_site());
        let expected = quote! {
            let variant_index: u8 = WireFormat::decode(_reader)?;
            match variant_index {
                0u8 => Ok(Self::Ping) ,
                1u8 => {
                    let content = WireFormat::decode(_reader)?;
                    Ok(Self::Text { content })
                },
                2u8 => {
                    let __0 = WireFormat::decode(_reader)?;
                    Ok(Self::Binary(__0))
                },
                _ => Err(::std::io::Error::new(::std::io::ErrorKind::InvalidData, "invalid variant index"))
            }
        };

        assert_eq!(
            decode_wire_format(&input.data, &container).to_string(),
            expected.to_string()
        );
    }

    #[test]
    fn enum_end_to_end() {
        let input: DeriveInput = parse_quote! {
            enum Message {
                Ping,
                Text { content: String },
                Binary(Vec<u8>),
            }
        };
        let output = wire_format_inner(input);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        insta::assert_snapshot!(output_str, @r###"
        mod wire_format_message {
            extern crate std;
            use self::std::io;
            use self::std::result::Result::Ok;
            use super::Message;
            use jetstream_wireformat::WireFormat;
            impl WireFormat for Message {
                fn byte_size(&self) -> u32 {
                    match self {
                        Self::Ping => 1,
                        Self::Text { ref content } => 1 + WireFormat::byte_size(content),
                        Self::Binary(ref __0) => 1 + WireFormat::byte_size(__0),
                    }
                }
                fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                    match self {
                        Self::Ping => {
                            WireFormat::encode(&(0u8), _writer)?;
                        }
                        Self::Text { ref content } => {
                            WireFormat::encode(&(1u8), _writer)?;
                            WireFormat::encode(content, _writer)?;
                        }
                        Self::Binary(ref __0) => {
                            WireFormat::encode(&(2u8), _writer)?;
                            WireFormat::encode(__0, _writer)?;
                        }
                    }
                    Ok(())
                }
                fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                    let variant_index: u8 = WireFormat::decode(_reader)?;
                    match variant_index {
                        0u8 => Ok(Self::Ping),
                        1u8 => {
                            let content = WireFormat::decode(_reader)?;
                            Ok(Self::Text { content })
                        }
                        2u8 => {
                            let __0 = WireFormat::decode(_reader)?;
                            Ok(Self::Binary(__0))
                        }
                        _ => {
                            Err(
                                ::std::io::Error::new(
                                    ::std::io::ErrorKind::InvalidData,
                                    "invalid variant index",
                                ),
                            )
                        }
                    }
                }
            }
        }
        "###);
    }
    #[test]
    fn test_struct_skip_field() {
        let input: DeriveInput = parse_quote! {
            struct Item {
                ident: u32,
                #[jetstream(skip)]
                skipped: String,
                other: u8,
            }
        };

        // Test byte_size
        let expected_size = quote! {
            0
                + WireFormat::byte_size(&self.ident)
                + WireFormat::byte_size(&self.other)
        };

        assert_eq!(
            byte_size_sum(&input.data).to_string(),
            expected_size.to_string()
        );

        // Test encode
        let expected_encode = quote! {
            WireFormat::encode(&self.ident, _writer)?;
            WireFormat::encode(&self.other, _writer)?;
            Ok(())
        };

        assert_eq!(
            encode_wire_format(&input.data).to_string(),
            expected_encode.to_string()
        );

        // Test decode
        let container = Ident::new("Item", Span::call_site());
        let expected_decode = quote! {
            let ident = WireFormat::decode(_reader)?;
            let other = WireFormat::decode(_reader)?;
            Ok(Item {
                ident: ident,
                skipped: Default::default(),
                other: other,
            })
        };

        assert_eq!(
            decode_wire_format(&input.data, &container).to_string(),
            expected_decode.to_string()
        );
    }

    #[test]
    fn test_tuple_struct_skip_field() {
        let input: DeriveInput = parse_quote! {
            struct Item(u32, #[jetstream(skip)] String, u8);
        };

        // Test byte_size
        let expected_size = quote! {
            0
                + WireFormat::byte_size(&self.0)
                + WireFormat::byte_size(&self.2)
        };

        assert_eq!(
            byte_size_sum(&input.data).to_string(),
            expected_size.to_string()
        );

        // Test encode
        let expected_encode = quote! {
            WireFormat::encode(&self.0, _writer)?;
            WireFormat::encode(&self.2, _writer)?;
            Ok(())
        };

        assert_eq!(
            encode_wire_format(&input.data).to_string(),
            expected_encode.to_string()
        );

        // Test decode
        let container = Ident::new("Item", Span::call_site());
        let expected_decode = quote! {
            let __0 = WireFormat::decode(_reader)?;
            let __2 = WireFormat::decode(_reader)?;
            Ok(Item(__0, Default::default(), __2,))
        };

        assert_eq!(
            decode_wire_format(&input.data, &container).to_string(),
            expected_decode.to_string()
        );
    }

    #[test]
    fn test_enum_skip_field() {
        let input: DeriveInput = parse_quote! {
            enum Message {
                Ping,
                Text {
                    content: String,
                    #[jetstream(skip)]
                    metadata: Vec<u8>
                },
                Binary(Vec<u8>, #[jetstream(skip)] String),
            }
        };

        // Test byte_size
        let expected_size = quote! {
            match self {
                Self::Ping => 1,
                Self::Text { ref content } => { 1 + WireFormat::byte_size(content) },
                Self::Binary(ref __0) => { 1 + WireFormat::byte_size(__0) }
            }
        };

        assert_eq!(
            byte_size_sum(&input.data).to_string(),
            expected_size.to_string()
        );

        // Test encode
        let expected_encode = quote! {
            match self {
                Self::Ping => {
                    WireFormat::encode(&(0u8), _writer)?;
                },
                Self::Text { ref content } => {
                    WireFormat::encode(&(1u8), _writer)?;
                    WireFormat::encode(content, _writer)?;
                },
                Self::Binary(ref __0) => {
                    WireFormat::encode(&(2u8), _writer)?;
                    WireFormat::encode(__0, _writer)?;
                }
            }
            Ok(())
        };

        assert_eq!(
            encode_wire_format(&input.data).to_string(),
            expected_encode.to_string()
        );

        // Test decode
        let container = Ident::new("Message", Span::call_site());
        let expected_decode = quote! {
            let variant_index: u8 = WireFormat::decode(_reader)?;
            match variant_index {
                0u8 => Ok(Self::Ping),
                1u8 => {
                    let content = WireFormat::decode(_reader)?;
                    Ok(Self::Text { content, metadata: Default::default() })
                },
                2u8 => {
                    let __0 = WireFormat::decode(_reader)?;
                    Ok(Self::Binary(__0, Default::default()))
                },
                _ => Err(::std::io::Error::new(::std::io::ErrorKind::InvalidData, "invalid variant index"))
            }
        };

        assert_eq!(
            decode_wire_format(&input.data, &container).to_string(),
            expected_decode.to_string()
        );
    }

    #[test]
    fn test_end_to_end_with_skip() {
        let input: DeriveInput = parse_quote! {
            struct Item {
                a: u8,
                #[jetstream(skip)]
                skip_this: String,
                b: u16,
                #[jetstream(skip)]
                also_skip: Vec<u8>,
                c: u32,
            }
        };

        let output = wire_format_inner(input);
        let syntax_tree: syn::File = syn::parse2(output).unwrap();
        let output_str = prettyplease::unparse(&syntax_tree);
        insta::assert_snapshot!(output_str, @r###"
        mod wire_format_item {
            extern crate std;
            use self::std::io;
            use self::std::result::Result::Ok;
            use super::Item;
            use jetstream_wireformat::WireFormat;
            impl WireFormat for Item {
                fn byte_size(&self) -> u32 {
                    0 + WireFormat::byte_size(&self.a) + WireFormat::byte_size(&self.b)
                        + WireFormat::byte_size(&self.c)
                }
                fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                    WireFormat::encode(&self.a, _writer)?;
                    WireFormat::encode(&self.b, _writer)?;
                    WireFormat::encode(&self.c, _writer)?;
                    Ok(())
                }
                fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                    let a = WireFormat::decode(_reader)?;
                    let b = WireFormat::decode(_reader)?;
                    let c = WireFormat::decode(_reader)?;
                    Ok(Item {
                        a: a,
                        skip_this: Default::default(),
                        b: b,
                        also_skip: Default::default(),
                        c: c,
                    })
                }
            }
        }
        "###);
    }
}
