// r[impl jetstream.codegen.ts]

use std::fmt::Write;

use convert_case::{Case, Casing};
use typeshare_core::rust_types::*;

/// Configuration for TypeScript code generation.
pub struct TsConfig {
    /// Import path for the wireformat module (e.g., "@sevki/jetstream-wireformat").
    pub import_path: String,
    /// Import path for the RPC module (e.g., "@sevki/jetstream-rpc").
    pub rpc_import_path: String,
}

impl Default for TsConfig {
    fn default() -> Self {
        Self {
            import_path: "@sevki/jetstream-wireformat".into(),
            rpc_import_path: "@sevki/jetstream-rpc".into(),
        }
    }
}

/// r[jetstream.codegen.ts.struct]
/// Generate a TypeScript interface + WireFormat codec for a RustStruct.
pub fn generate_ts_struct(s: &RustStruct, _config: &TsConfig) -> String {
    let mut out = String::new();
    let name = &s.id.original;

    // Interface
    writeln!(out, "export interface {name} {{").unwrap();
    for field in &s.fields {
        let field_name = field.id.renamed.to_case(Case::Camel);
        let ts_type = rust_type_to_ts(&field.ty);
        writeln!(out, "  {field_name}: {ts_type};").unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // Codec
    let codec_name = format!("{}Codec", name.to_case(Case::Camel));
    writeln!(out, "export const {codec_name}: WireFormat<{name}> = {{")
        .unwrap();

    // byteSize
    writeln!(out, "  byteSize(value: {name}): number {{").unwrap();
    if s.fields.is_empty() {
        writeln!(out, "    return 0;").unwrap();
    } else {
        let parts: Vec<String> = s
            .fields
            .iter()
            .map(|f| {
                let field_name = f.id.renamed.to_case(Case::Camel);
                let codec = rust_type_to_ts_codec(&f.ty);
                format!("{codec}.byteSize(value.{field_name})")
            })
            .collect();
        writeln!(out, "    return {};", parts.join(" + ")).unwrap();
    }
    writeln!(out, "  }},").unwrap();

    // encode
    writeln!(
        out,
        "  encode(value: {name}, writer: BinaryWriter): void {{"
    )
    .unwrap();
    for field in &s.fields {
        let field_name = field.id.renamed.to_case(Case::Camel);
        let codec = rust_type_to_ts_codec(&field.ty);
        writeln!(out, "    {codec}.encode(value.{field_name}, writer);")
            .unwrap();
    }
    writeln!(out, "  }},").unwrap();

    // decode
    writeln!(out, "  decode(reader: BinaryReader): {name} {{").unwrap();
    for field in &s.fields {
        let field_name = field.id.renamed.to_case(Case::Camel);
        let codec = rust_type_to_ts_codec(&field.ty);
        writeln!(out, "    const {field_name} = {codec}.decode(reader);")
            .unwrap();
    }
    let field_list: Vec<String> = s
        .fields
        .iter()
        .map(|f| f.id.renamed.to_case(Case::Camel))
        .collect();
    writeln!(out, "    return {{ {} }};", field_list.join(", ")).unwrap();
    writeln!(out, "  }},").unwrap();

    writeln!(out, "}};").unwrap();

    out
}

/// r[jetstream.codegen.ts.enum]
/// Generate a TypeScript discriminated union type + WireFormat codec for a RustEnum.
pub fn generate_ts_enum(e: &RustEnum, _config: &TsConfig) -> String {
    let mut out = String::new();
    let shared = e.shared();
    let name = &shared.id.original;

    // Type union
    let variants: Vec<String> = shared
        .variants
        .iter()
        .map(|v| match v {
            RustEnumVariant::Unit(shared) => {
                format!("{{ tag: '{}' }}", shared.id.original)
            }
            RustEnumVariant::Tuple { ty, shared } => {
                let ts_type = rust_type_to_ts(ty);
                format!(
                    "{{ tag: '{}'; value: {} }}",
                    shared.id.original, ts_type
                )
            }
            RustEnumVariant::AnonymousStruct { fields, shared } => {
                let mut parts = vec![format!("tag: '{}'", shared.id.original)];
                for f in fields {
                    let field_name = f.id.renamed.to_case(Case::Camel);
                    let ts_type = rust_type_to_ts(&f.ty);
                    parts.push(format!("{field_name}: {ts_type}"));
                }
                format!("{{ {} }}", parts.join("; "))
            }
        })
        .collect();

    writeln!(out, "export type {name} =").unwrap();
    for (i, v) in variants.iter().enumerate() {
        if i < variants.len() - 1 {
            writeln!(out, "  | {v}").unwrap();
        } else {
            writeln!(out, "  | {v};").unwrap();
        }
    }
    writeln!(out).unwrap();

    // Codec
    let codec_name = format!("{}Codec", name.to_case(Case::Camel));
    writeln!(out, "export const {codec_name}: WireFormat<{name}> = {{")
        .unwrap();

    // byteSize
    writeln!(out, "  byteSize(value: {name}): number {{").unwrap();
    writeln!(out, "    switch (value.tag) {{").unwrap();
    for v in &shared.variants {
        match v {
            RustEnumVariant::Unit(shared) => {
                writeln!(out, "      case '{}': return 1;", shared.id.original)
                    .unwrap();
            }
            RustEnumVariant::Tuple { ty, shared } => {
                let codec = rust_type_to_ts_codec(ty);
                writeln!(
                    out,
                    "      case '{}': return 1 + {codec}.byteSize(value.value);",
                    shared.id.original
                )
                .unwrap();
            }
            RustEnumVariant::AnonymousStruct { fields, shared } => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let field_name = f.id.renamed.to_case(Case::Camel);
                        let codec = rust_type_to_ts_codec(&f.ty);
                        format!("{codec}.byteSize(value.{field_name})")
                    })
                    .collect();
                if parts.is_empty() {
                    writeln!(
                        out,
                        "      case '{}': return 1;",
                        shared.id.original
                    )
                    .unwrap();
                } else {
                    writeln!(
                        out,
                        "      case '{}': return 1 + {};",
                        shared.id.original,
                        parts.join(" + ")
                    )
                    .unwrap();
                }
            }
        }
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    // encode
    writeln!(
        out,
        "  encode(value: {name}, writer: BinaryWriter): void {{"
    )
    .unwrap();
    writeln!(out, "    switch (value.tag) {{").unwrap();
    for (idx, v) in shared.variants.iter().enumerate() {
        match v {
            RustEnumVariant::Unit(shared) => {
                writeln!(
                    out,
                    "      case '{}': writer.writeU8({idx}); break;",
                    shared.id.original
                )
                .unwrap();
            }
            RustEnumVariant::Tuple { ty, shared } => {
                let codec = rust_type_to_ts_codec(ty);
                writeln!(
                    out,
                    "      case '{}': writer.writeU8({idx}); {codec}.encode(value.value, writer); break;",
                    shared.id.original
                )
                .unwrap();
            }
            RustEnumVariant::AnonymousStruct { fields, shared } => {
                let encodes: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let field_name = f.id.renamed.to_case(Case::Camel);
                        let codec = rust_type_to_ts_codec(&f.ty);
                        format!("{codec}.encode(value.{field_name}, writer);")
                    })
                    .collect();
                writeln!(
                    out,
                    "      case '{}': writer.writeU8({idx}); {} break;",
                    shared.id.original,
                    encodes.join(" ")
                )
                .unwrap();
            }
        }
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    // decode
    writeln!(out, "  decode(reader: BinaryReader): {name} {{").unwrap();
    writeln!(out, "    const tag = reader.readU8();").unwrap();
    writeln!(out, "    switch (tag) {{").unwrap();
    for (idx, v) in shared.variants.iter().enumerate() {
        match v {
            RustEnumVariant::Unit(shared) => {
                writeln!(
                    out,
                    "      case {idx}: return {{ tag: '{}' }};",
                    shared.id.original
                )
                .unwrap();
            }
            RustEnumVariant::Tuple { ty, shared } => {
                let codec = rust_type_to_ts_codec(ty);
                writeln!(
                    out,
                    "      case {idx}: return {{ tag: '{}', value: {codec}.decode(reader) }};",
                    shared.id.original
                )
                .unwrap();
            }
            RustEnumVariant::AnonymousStruct { fields, shared } => {
                write!(out, "      case {idx}: {{ ").unwrap();
                for f in fields {
                    let field_name = f.id.renamed.to_case(Case::Camel);
                    let codec = rust_type_to_ts_codec(&f.ty);
                    write!(
                        out,
                        "const {field_name} = {codec}.decode(reader); "
                    )
                    .unwrap();
                }
                let field_list: Vec<String> = fields
                    .iter()
                    .map(|f| f.id.renamed.to_case(Case::Camel))
                    .collect();
                writeln!(
                    out,
                    "return {{ tag: '{}', {} }}; }}",
                    shared.id.original,
                    field_list.join(", ")
                )
                .unwrap();
            }
        }
    }
    writeln!(
        out,
        "      default: throw new Error(`invalid variant index: ${{tag}}`);"
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "  }},").unwrap();

    writeln!(out, "}};").unwrap();

    out
}

/// Generate a complete TypeScript file from a list of RustItems.
pub fn generate_ts_file(items: &[RustItem], config: &TsConfig) -> String {
    let mut out = String::new();

    // Collect needed codecs
    let mut needed_codecs = std::collections::BTreeSet::new();
    for item in items {
        collect_needed_codecs(item, &mut needed_codecs);
    }

    // Imports — emit a single consolidated import from the barrel
    let mut value_imports = vec!["BinaryReader", "BinaryWriter"];
    for codec in &needed_codecs {
        value_imports.push(codec);
    }
    writeln!(
        out,
        "import {{ {} }} from '{}';",
        value_imports.join(", "),
        config.import_path
    )
    .unwrap();
    writeln!(
        out,
        "import type {{ WireFormat }} from '{}';",
        config.import_path
    )
    .unwrap();
    writeln!(out).unwrap();

    // Types
    for item in items {
        match item {
            RustItem::Struct(s) => {
                write!(out, "{}", generate_ts_struct(s, config)).unwrap();
                writeln!(out).unwrap();
            }
            RustItem::Enum(e) => {
                write!(out, "{}", generate_ts_enum(e, config)).unwrap();
                writeln!(out).unwrap();
            }
            _ => {}
        }
    }

    out
}

#[allow(clippy::needless_lifetimes)]
fn collect_needed_codecs<'a>(
    item: &RustItem,
    codecs: &mut std::collections::BTreeSet<&'a str>,
) {
    let types: Vec<&RustType> = match item {
        RustItem::Struct(s) => s.fields.iter().map(|f| &f.ty).collect(),
        RustItem::Enum(e) => e
            .shared()
            .variants
            .iter()
            .flat_map(|v| match v {
                RustEnumVariant::Unit(_) => vec![],
                RustEnumVariant::Tuple { ty, .. } => vec![ty],
                RustEnumVariant::AnonymousStruct { fields, .. } => {
                    fields.iter().map(|f| &f.ty).collect()
                }
            })
            .collect(),
        _ => vec![],
    };

    for ty in types {
        collect_codecs_for_type(ty, codecs);
    }
}
#[allow(clippy::needless_lifetimes)]
fn collect_codecs_for_type<'a>(
    ty: &RustType,
    codecs: &mut std::collections::BTreeSet<&'a str>,
) {
    if let RustType::Special(special) = ty {
        match special {
            SpecialRustType::U8 => {
                codecs.insert("u8Codec");
            }
            SpecialRustType::U16 => {
                codecs.insert("u16Codec");
            }
            SpecialRustType::U32 => {
                codecs.insert("u32Codec");
            }
            SpecialRustType::U64 | SpecialRustType::USize => {
                codecs.insert("u64Codec");
            }
            SpecialRustType::I8 => {
                codecs.insert("u8Codec");
            } // i8 uses u8Codec with cast
            SpecialRustType::I16 => {
                codecs.insert("i16Codec");
            }
            SpecialRustType::I32 => {
                codecs.insert("i32Codec");
            }
            SpecialRustType::I64 | SpecialRustType::ISize => {
                codecs.insert("i64Codec");
            }
            SpecialRustType::F32 => {
                codecs.insert("f32Codec");
            }
            SpecialRustType::F64 => {
                codecs.insert("f64Codec");
            }
            SpecialRustType::Bool => {
                codecs.insert("boolCodec");
            }
            SpecialRustType::String | SpecialRustType::Char => {
                codecs.insert("stringCodec");
            }
            SpecialRustType::Vec(inner) => {
                codecs.insert("vecCodec");
                collect_codecs_for_type(inner, codecs);
            }
            SpecialRustType::Option(inner) => {
                codecs.insert("optionCodec");
                collect_codecs_for_type(inner, codecs);
            }
            SpecialRustType::HashMap(k, v) => {
                codecs.insert("mapCodec");
                collect_codecs_for_type(k, codecs);
                collect_codecs_for_type(v, codecs);
            }
            _ => {}
        }
    }
}

/// Map a RustType to a TypeScript type string.
pub fn rust_type_to_ts(ty: &RustType) -> String {
    match ty {
        RustType::Special(special) => match special {
            SpecialRustType::U8
            | SpecialRustType::U16
            | SpecialRustType::U32 => "number".into(),
            SpecialRustType::U64 | SpecialRustType::USize => "bigint".into(),
            SpecialRustType::I8
            | SpecialRustType::I16
            | SpecialRustType::I32 => "number".into(),
            SpecialRustType::I64 | SpecialRustType::ISize => "bigint".into(),
            SpecialRustType::F32 | SpecialRustType::F64 => "number".into(),
            SpecialRustType::Bool => "boolean".into(),
            SpecialRustType::String | SpecialRustType::Char => "string".into(),
            SpecialRustType::Unit => "undefined".into(),
            SpecialRustType::Vec(inner) => {
                format!("{}[]", rust_type_to_ts(inner))
            }
            SpecialRustType::Array(inner, _) => {
                format!("{}[]", rust_type_to_ts(inner))
            }
            SpecialRustType::Slice(inner) => {
                format!("{}[]", rust_type_to_ts(inner))
            }
            SpecialRustType::Option(inner) => {
                format!("{} | null", rust_type_to_ts(inner))
            }
            SpecialRustType::HashMap(k, v) => {
                format!("Map<{}, {}>", rust_type_to_ts(k), rust_type_to_ts(v))
            }
            _ => "any".into(),
        },
        RustType::Simple { id } => id.clone(),
        RustType::Generic { id, parameters } => {
            let params: Vec<String> =
                parameters.iter().map(rust_type_to_ts).collect();
            format!("{}<{}>", id, params.join(", "))
        }
    }
}

/// Map a RustType to a TypeScript codec expression string.
pub fn rust_type_to_ts_codec(ty: &RustType) -> String {
    match ty {
        RustType::Special(special) => match special {
            SpecialRustType::U8 => "u8Codec".into(),
            SpecialRustType::U16 => "u16Codec".into(),
            SpecialRustType::U32 => "u32Codec".into(),
            SpecialRustType::U64 | SpecialRustType::USize => "u64Codec".into(),
            SpecialRustType::I8 => "u8Codec".into(), // i8 treated as u8 on wire
            SpecialRustType::I16 => "i16Codec".into(),
            SpecialRustType::I32 => "i32Codec".into(),
            SpecialRustType::I64 | SpecialRustType::ISize => "i64Codec".into(),
            SpecialRustType::F32 => "f32Codec".into(),
            SpecialRustType::F64 => "f64Codec".into(),
            SpecialRustType::Bool => "boolCodec".into(),
            SpecialRustType::String | SpecialRustType::Char => {
                "stringCodec".into()
            }
            SpecialRustType::Unit => "unitCodec".into(),
            SpecialRustType::Vec(inner) => {
                format!("vecCodec({})", rust_type_to_ts_codec(inner))
            }
            SpecialRustType::Array(inner, _)
            | SpecialRustType::Slice(inner) => {
                format!("vecCodec({})", rust_type_to_ts_codec(inner))
            }
            SpecialRustType::Option(inner) => {
                format!("optionCodec({})", rust_type_to_ts_codec(inner))
            }
            SpecialRustType::HashMap(k, v) => {
                format!(
                    "mapCodec({}, {}, (a, b) => a < b ? -1 : a > b ? 1 : 0)",
                    rust_type_to_ts_codec(k),
                    rust_type_to_ts_codec(v)
                )
            }
            _ => "unknownCodec".into(),
        },
        RustType::Simple { id } => format!("{}Codec", id.to_case(Case::Camel)),
        RustType::Generic { id, .. } => {
            format!("{}Codec", id.to_case(Case::Camel))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;

    #[test]
    fn test_generate_struct() {
        let source = r#"
            #[derive(JetStreamWireFormat)]
            struct Point {
                x: u32,
                y: u32,
            }
        "#;
        let items = parse_file(source);
        let config = TsConfig::default();
        let ts = generate_ts_struct(
            match &items[0] {
                RustItem::Struct(s) => s,
                _ => panic!("expected struct"),
            },
            &config,
        );
        assert!(ts.contains("export interface Point"));
        assert!(ts.contains("x: number"));
        assert!(ts.contains("y: number"));
        assert!(ts.contains("pointCodec"));
        assert!(ts.contains("u32Codec.byteSize"));
        assert!(ts.contains("u32Codec.encode"));
        assert!(ts.contains("u32Codec.decode"));
    }

    #[test]
    fn test_generate_enum() {
        let source = r#"
            #[derive(JetStreamWireFormat)]
            enum Color {
                Red,
                Green,
                Blue,
            }
        "#;
        let items = parse_file(source);
        let config = TsConfig::default();
        let ts = generate_ts_enum(
            match &items[0] {
                RustItem::Enum(e) => e,
                _ => panic!("expected enum"),
            },
            &config,
        );
        assert!(ts.contains("export type Color ="));
        assert!(ts.contains("tag: 'Red'"));
        assert!(ts.contains("tag: 'Green'"));
        assert!(ts.contains("tag: 'Blue'"));
        assert!(ts.contains("colorCodec"));
    }
}
