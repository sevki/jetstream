use syn::parse_quote;
use syn::DeriveInput;

use crate::wireformat::wire_format_inner;

#[test]
fn test_generics_support() {
    let input: DeriveInput = parse_quote! {
        struct GenericItem<T, U> {
            field_t: T,
            field_u: U,
            field_int: u32,
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
        impl<T, U> WireFormat for GenericItem<T, U>
        where
            T: jetstream_wireformat::WireFormat,
            U: jetstream_wireformat::WireFormat,
        {
            fn byte_size(&self) -> u32 {
                0 + WireFormat::byte_size(&self.field_t)
                    + WireFormat::byte_size(&self.field_u)
                    + WireFormat::byte_size(&self.field_int)
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                WireFormat::encode(&self.field_t, _writer)?;
                WireFormat::encode(&self.field_u, _writer)?;
                WireFormat::encode(&self.field_int, _writer)?;
                Ok(())
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                let field_t = WireFormat::decode(_reader)?;
                let field_u = WireFormat::decode(_reader)?;
                let field_int = WireFormat::decode(_reader)?;
                Ok(GenericItem {
                    field_t: field_t,
                    field_u: field_u,
                    field_int: field_int,
                })
            }
        }
    };
    ");
}

#[test]
fn test_generic_enum() {
    let input: DeriveInput = parse_quote! {
        enum GenericEnum<T, U> {
            VariantT(T),
            VariantU { value: U },
            VariantNone,
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
        impl<T, U> WireFormat for GenericEnum<T, U>
        where
            T: jetstream_wireformat::WireFormat,
            U: jetstream_wireformat::WireFormat,
        {
            fn byte_size(&self) -> u32 {
                match self {
                    Self::VariantT(ref __0) => 1 + WireFormat::byte_size(__0),
                    Self::VariantU { ref value } => 1 + WireFormat::byte_size(value),
                    Self::VariantNone => 1,
                }
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                match self {
                    Self::VariantT(ref __0) => {
                        WireFormat::encode(&(0u8), _writer)?;
                        WireFormat::encode(__0, _writer)?;
                    }
                    Self::VariantU { ref value } => {
                        WireFormat::encode(&(1u8), _writer)?;
                        WireFormat::encode(value, _writer)?;
                    }
                    Self::VariantNone => {
                        WireFormat::encode(&(2u8), _writer)?;
                    }
                }
                Ok(())
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                let variant_index: u8 = WireFormat::decode(_reader)?;
                match variant_index {
                    0u8 => {
                        let __0 = WireFormat::decode(_reader)?;
                        Ok(Self::VariantT(__0))
                    }
                    1u8 => {
                        let value = WireFormat::decode(_reader)?;
                        Ok(Self::VariantU { value })
                    }
                    2u8 => Ok(Self::VariantNone),
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
