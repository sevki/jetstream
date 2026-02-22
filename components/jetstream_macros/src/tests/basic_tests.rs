use proc_macro2::Span;
use quote::quote;
use syn::{parse_quote, DeriveInput, Ident};
extern crate pretty_assertions;
use pretty_assertions::assert_eq;

use crate::wireformat::{
    byte_size_sum, decode_wire_format, encode_wire_format, wire_format_inner,
};

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

    assert_eq!(byte_size_sum(&input.data).to_string(), expected.to_string());
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
    insta::assert_snapshot!(output_str, @r"
    const _: () = {
        extern crate std;
        use std::io;
        use std::result::Result::Ok;
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
    };
    ");
}

#[test]
fn end_to_end_unnamed() {
    let input: DeriveInput = parse_quote! {
        struct Niijima_先輩(u8, u16, u32, u64, String, Vec<String>, Nested);
    };

    let output = wire_format_inner(input);
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    insta::assert_snapshot!(output_str, @r"
    const _: () = {
        extern crate std;
        use std::io;
        use std::result::Result::Ok;
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
    };
    ");
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

    assert_eq!(byte_size_sum(&input.data).to_string(), expected.to_string());
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
    insta::assert_snapshot!(output_str, @r#"
    const _: () = {
        extern crate std;
        use std::io;
        use std::result::Result::Ok;
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
    };
    "#);
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
    insta::assert_snapshot!(output_str, @r"
    const _: () = {
        extern crate std;
        use std::io;
        use std::result::Result::Ok;
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
    };
    ");
}
