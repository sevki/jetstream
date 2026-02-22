use syn::parse_quote;
use syn::DeriveInput;

use crate::wireformat::wire_format_inner;

#[test]
fn test_with_option() {
    let input: DeriveInput = parse_quote! {
        struct ItemWithWith {
            a: u8,
            #[jetstream(with(CustomCodec))]
            custom_encoded: Vec<u8>,
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
        impl WireFormat for ItemWithWith {
            fn byte_size(&self) -> u32 {
                0 + WireFormat::byte_size(&self.a)
                    + CustomCodec::byte_size(&self.custom_encoded)
                    + WireFormat::byte_size(&self.c)
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                WireFormat::encode(&self.a, _writer)?;
                CustomCodec::encode(&self.custom_encoded, _writer)?;
                WireFormat::encode(&self.c, _writer)?;
                Ok(())
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                let a = WireFormat::decode(_reader)?;
                let custom_encoded = CustomCodec::decode(_reader)?;
                let c = WireFormat::decode(_reader)?;
                Ok(ItemWithWith {
                    a: a,
                    custom_encoded: custom_encoded,
                    c: c,
                })
            }
        }
    };
    ");
}

#[test]
fn test_specific_encode_decode_options() {
    let input: DeriveInput = parse_quote! {
        struct ItemWithSpecificEncodeDecode {
            a: u8,
            #[jetstream(with_encode(CustomEncoder), with_decode(CustomDecoder), with_byte_size(CustomSizer))]
            custom_field: Vec<u8>,
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
        impl WireFormat for ItemWithSpecificEncodeDecode {
            fn byte_size(&self) -> u32 {
                0 + WireFormat::byte_size(&self.a) + CustomSizer(&self.custom_field)
                    + WireFormat::byte_size(&self.c)
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                WireFormat::encode(&self.a, _writer)?;
                CustomEncoder(&self.custom_field, _writer)?;
                WireFormat::encode(&self.c, _writer)?;
                Ok(())
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                let a = WireFormat::decode(_reader)?;
                let custom_field = CustomDecoder(_reader)?;
                let c = WireFormat::decode(_reader)?;
                Ok(ItemWithSpecificEncodeDecode {
                    a: a,
                    custom_field: custom_field,
                    c: c,
                })
            }
        }
    };
    ");
}

#[test]
fn test_from_into_as_options() {
    let input: DeriveInput = parse_quote! {
        struct ItemWithFromIntoAs {
            a: u8,
            #[jetstream(into(into_wire_format))]
            into_field: NonWireType,
            #[jetstream(from(from_wire_format))]
            from_field: NonWireType,
            #[jetstream(as(as_wire_format))]
            as_field: NonWireType,
            z: u32,
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
        impl WireFormat for ItemWithFromIntoAs {
            fn byte_size(&self) -> u32 {
                0 + WireFormat::byte_size(&self.a) + WireFormat::byte_size(&self.into_field)
                    + WireFormat::byte_size(&self.from_field)
                    + WireFormat::byte_size(&self.as_field) + WireFormat::byte_size(&self.z)
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                WireFormat::encode(&self.a, _writer)?;
                WireFormat::encode(&(into_wire_format(&self.into_field)), _writer)?;
                WireFormat::encode(&self.from_field, _writer)?;
                WireFormat::encode(&as_wire_format(&self.as_field), _writer)?;
                WireFormat::encode(&self.z, _writer)?;
                Ok(())
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                let a = WireFormat::decode(_reader)?;
                let into_field = WireFormat::decode(_reader)?;
                let from_field = from_wire_format(WireFormat::decode(_reader)?);
                let as_field = WireFormat::decode(_reader)?;
                let z = WireFormat::decode(_reader)?;
                Ok(ItemWithFromIntoAs {
                    a: a,
                    into_field: into_field,
                    from_field: from_field,
                    as_field: as_field,
                    z: z,
                })
            }
        }
    };
    ");
}

#[test]
fn test_options_with_enum() {
    let input: DeriveInput = parse_quote! {
        enum EnumWithOptions {
            A(u8),
            B {
                #[jetstream(with(CustomCodec))]
                value: Vec<u8>
            },
            C(#[jetstream(from(from_wire_format))] NonWireType),
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
        impl WireFormat for EnumWithOptions {
            fn byte_size(&self) -> u32 {
                match self {
                    Self::A(ref __0) => 1 + WireFormat::byte_size(__0),
                    Self::B { ref value } => 1 + CustomCodec::byte_size(value),
                    Self::C(ref __0) => 1 + WireFormat::byte_size(__0),
                }
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                match self {
                    Self::A(ref __0) => {
                        WireFormat::encode(&(0u8), _writer)?;
                        WireFormat::encode(__0, _writer)?;
                    }
                    Self::B { ref value } => {
                        WireFormat::encode(&(1u8), _writer)?;
                        CustomCodec::encode(value, _writer)?;
                    }
                    Self::C(ref __0) => {
                        WireFormat::encode(&(2u8), _writer)?;
                        WireFormat::encode(__0, _writer)?;
                    }
                }
                Ok(())
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                let variant_index: u8 = WireFormat::decode(_reader)?;
                match variant_index {
                    0u8 => {
                        let __0 = WireFormat::decode(_reader)?;
                        Ok(Self::A(__0))
                    }
                    1u8 => {
                        let value = CustomCodec::decode(_reader)?;
                        Ok(Self::B { value })
                    }
                    2u8 => {
                        let __0 = from_wire_format(WireFormat::decode(_reader)?);
                        Ok(Self::C(__0))
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
