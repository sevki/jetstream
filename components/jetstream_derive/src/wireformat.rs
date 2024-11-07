use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DeriveInput, Fields, Ident};

pub(crate) fn wire_format_inner(input: DeriveInput) -> TokenStream {
    if !input.generics.params.is_empty() {
        return quote! {
            compile_error!("derive(JetStreamWireFormat) does not support generic parameters");
        };
    }

    let container = input.ident;

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
        }
    }
}

// Generate code that recursively calls byte_size on every field in the struct.
fn byte_size_sum(data: &Data) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let fields = fields.named.iter().map(|f| {
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
            let fields = unnamed.unnamed.iter().enumerate().map(|(i, _f)| {
                let index = syn::Index::from(i);
                quote! {
                    WireFormat::byte_size(&self.#index)
                }
            });

            quote! {
                0 #(+ #fields)*
            }
        } else {
            unimplemented!("byte_size_sum for {:?}", data.struct_token.span);
        }
    } else {
        unimplemented!("byte_size_sum for ");
    }
}

// Generate code that recursively calls encode on every field in the struct.
fn encode_wire_format(data: &Data) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let fields = fields.named.iter().map(|f| {
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
            let fields = unnamed.unnamed.iter().enumerate().map(|(i, _f)| {
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
    } else {
        unimplemented!();
    }
}

// Generate code that recursively calls decode on every field in the struct.
fn decode_wire_format(data: &Data, container: &Ident) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let values = fields.named.iter().map(|f| {
                let field = &f.ident;
                let span = field.span();
                quote_spanned! {span=>
                    let #field = WireFormat::decode(_reader)?;
                }
            });

            let members = fields.named.iter().map(|f| {
                let field = &f.ident;
                quote! {
                    #field: #field,
                }
            });

            quote! {
                #(#values)*

                Ok(#container {
                    #(#members)*
                })
            }
        } else if let Fields::Unnamed(unnamed) = &data.fields {
            let values = unnamed.unnamed.iter().enumerate().map(|(i, _f)| {
                let index = syn::Index::from(i);
                // create a new ident that s __{index}
                let ident = Ident::new(
                    &format!("__{}", index.index),
                    Span::call_site(),
                );
                quote! {
                    let #ident = WireFormat::decode(_reader)?;
                }
            });

            let members = unnamed.unnamed.iter().enumerate().map(|(i, _f)| {
                let index = syn::Index::from(i);
                let ident = Ident::new(
                    &format!("__{}", index.index),
                    Span::call_site(),
                );
                quote! {
                    #ident
                }
            });

            quote! {
                #(#values)*

                Ok(#container(
                    #(#members,)*
                ))
            }
        } else {
            unimplemented!();
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

        let expected = quote! {
            mod wire_format_niijima_先輩 {
                extern crate std;
                use self::std::io;
                use self::std::result::Result::Ok;

                use super::Niijima_先輩;

                use jetstream_wireformat::WireFormat;

                impl WireFormat for Niijima_先輩 {
                    fn byte_size(&self) -> u32 {
                        0
                        + WireFormat::byte_size(&self.a)
                        + WireFormat::byte_size(&self.b)
                        + WireFormat::byte_size(&self.c)
                        + WireFormat::byte_size(&self.d)
                        + WireFormat::byte_size(&self.e)
                        + WireFormat::byte_size(&self.f)
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
        };

        assert_eq!(wire_format_inner(input).to_string(), expected.to_string(),);
    }

    #[test]
    fn end_to_end_unnamed() {
        let input: DeriveInput = parse_quote! {
            struct Niijima_先輩(u8, u16, u32, u64, String, Vec<String>, Nested);
        };

        let expected = quote! {
            mod wire_format_niijima_先輩 {
                extern crate std;
                use self::std::io;
                use self::std::result::Result::Ok;

                use super::Niijima_先輩;

                use jetstream_wireformat::WireFormat;

                impl WireFormat for Niijima_先輩 {
                    fn byte_size(&self) -> u32 {
                        0
                        + WireFormat::byte_size(&self.0)
                        + WireFormat::byte_size(&self.1)
                        + WireFormat::byte_size(&self.2)
                        + WireFormat::byte_size(&self.3)
                        + WireFormat::byte_size(&self.4)
                        + WireFormat::byte_size(&self.5)
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
                        let __1= WireFormat::decode(_reader)?;
                        let __2 = WireFormat::decode(_reader)?;
                        let __3 = WireFormat::decode(_reader)?;
                        let __4 = WireFormat::decode(_reader)?;
                        let __5 = WireFormat::decode(_reader)?;
                        let __6 = WireFormat::decode(_reader)?;
                        Ok(Niijima_先輩(
                            __0,
                            __1,
                            __2,
                            __3,
                            __4,
                            __5,
                            __6,
                        ))
                    }
                }
            }
        };

        assert_eq!(wire_format_inner(input).to_string(), expected.to_string(),);
    }
}
