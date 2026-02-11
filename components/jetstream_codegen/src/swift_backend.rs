// r[impl jetstream.codegen.swift]

use std::fmt::Write;

use convert_case::{Case, Casing};
use typeshare_core::rust_types::*;

/// Configuration for Swift code generation.
pub struct SwiftConfig {
    /// Module name for the WireFormat import.
    pub wireformat_module: String,
}

impl Default for SwiftConfig {
    fn default() -> Self {
        Self {
            wireformat_module: "JetStreamWireFormat".into(),
        }
    }
}

/// r[jetstream.codegen.swift.struct]
/// Generate a Swift struct conforming to WireFormat from a RustStruct.
pub fn generate_swift_struct(s: &RustStruct, _config: &SwiftConfig) -> String {
    let mut out = String::new();
    let name = &s.id.original;

    writeln!(out, "public struct {name}: WireFormat {{").unwrap();

    // Properties
    for field in &s.fields {
        let field_name = field.id.renamed.to_case(Case::Camel);
        let swift_type = rust_type_to_swift(&field.ty);
        writeln!(out, "    public var {field_name}: {swift_type}").unwrap();
    }
    writeln!(out).unwrap();

    // byteSize
    writeln!(out, "    public func byteSize() -> UInt32 {{").unwrap();
    if s.fields.is_empty() {
        writeln!(out, "        return 0").unwrap();
    } else {
        let parts: Vec<String> = s
            .fields
            .iter()
            .map(|f| {
                let field_name = f.id.renamed.to_case(Case::Camel);
                format!("{field_name}.byteSize()")
            })
            .collect();
        writeln!(out, "        return {}", parts.join(" + ")).unwrap();
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // encode
    writeln!(
        out,
        "    public func encode(writer: inout BinaryWriter) throws {{"
    )
    .unwrap();
    for field in &s.fields {
        let field_name = field.id.renamed.to_case(Case::Camel);
        writeln!(out, "        try {field_name}.encode(writer: &writer)").unwrap();
    }
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // decode
    writeln!(
        out,
        "    public static func decode(reader: inout BinaryReader) throws -> {name} {{"
    )
    .unwrap();
    for field in &s.fields {
        let field_name = field.id.renamed.to_case(Case::Camel);
        let swift_type = rust_type_to_swift(&field.ty);
        writeln!(
            out,
            "        let {field_name} = try {swift_type}.decode(reader: &reader)"
        )
        .unwrap();
    }
    let field_list: Vec<String> = s
        .fields
        .iter()
        .map(|f| {
            let field_name = f.id.renamed.to_case(Case::Camel);
            format!("{field_name}: {field_name}")
        })
        .collect();
    writeln!(
        out,
        "        return {name}({})",
        field_list.join(", ")
    )
    .unwrap();
    writeln!(out, "    }}").unwrap();

    writeln!(out, "}}").unwrap();

    out
}

/// r[jetstream.codegen.swift.enum]
/// Generate a Swift enum conforming to WireFormat from a RustEnum.
pub fn generate_swift_enum(e: &RustEnum, _config: &SwiftConfig) -> String {
    let mut out = String::new();
    let shared = e.shared();
    let name = &shared.id.original;

    writeln!(out, "public enum {name}: WireFormat {{").unwrap();

    // Cases
    for v in &shared.variants {
        match v {
            RustEnumVariant::Unit(shared) => {
                let case_name = shared.id.original.to_case(Case::Camel);
                writeln!(out, "    case {case_name}").unwrap();
            }
            RustEnumVariant::Tuple { ty, shared } => {
                let case_name = shared.id.original.to_case(Case::Camel);
                let swift_type = rust_type_to_swift(ty);
                writeln!(out, "    case {case_name}({swift_type})").unwrap();
            }
            RustEnumVariant::AnonymousStruct { fields, shared } => {
                let case_name = shared.id.original.to_case(Case::Camel);
                let params: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let field_name = f.id.renamed.to_case(Case::Camel);
                        let swift_type = rust_type_to_swift(&f.ty);
                        format!("{field_name}: {swift_type}")
                    })
                    .collect();
                writeln!(out, "    case {case_name}({})", params.join(", ")).unwrap();
            }
        }
    }
    writeln!(out).unwrap();

    // byteSize
    writeln!(out, "    public func byteSize() -> UInt32 {{").unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for v in &shared.variants {
        match v {
            RustEnumVariant::Unit(shared) => {
                let case_name = shared.id.original.to_case(Case::Camel);
                writeln!(out, "        case .{case_name}: return 1").unwrap();
            }
            RustEnumVariant::Tuple { shared, .. } => {
                let case_name = shared.id.original.to_case(Case::Camel);
                writeln!(
                    out,
                    "        case .{case_name}(let v): return 1 + v.byteSize()"
                )
                .unwrap();
            }
            RustEnumVariant::AnonymousStruct { fields, shared } => {
                let case_name = shared.id.original.to_case(Case::Camel);
                let bindings: Vec<String> = fields
                    .iter()
                    .map(|f| format!("let {}", f.id.renamed.to_case(Case::Camel)))
                    .collect();
                let sizes: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        format!("{}.byteSize()", f.id.renamed.to_case(Case::Camel))
                    })
                    .collect();
                writeln!(
                    out,
                    "        case .{case_name}({}): return 1 + {}",
                    bindings.join(", "),
                    sizes.join(" + ")
                )
                .unwrap();
            }
        }
    }
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // encode
    writeln!(
        out,
        "    public func encode(writer: inout BinaryWriter) throws {{"
    )
    .unwrap();
    writeln!(out, "        switch self {{").unwrap();
    for (idx, v) in shared.variants.iter().enumerate() {
        match v {
            RustEnumVariant::Unit(shared) => {
                let case_name = shared.id.original.to_case(Case::Camel);
                writeln!(out, "        case .{case_name}:").unwrap();
                writeln!(out, "            writer.writeU8({idx})").unwrap();
            }
            RustEnumVariant::Tuple { shared, .. } => {
                let case_name = shared.id.original.to_case(Case::Camel);
                writeln!(out, "        case .{case_name}(let v):").unwrap();
                writeln!(out, "            writer.writeU8({idx})").unwrap();
                writeln!(out, "            try v.encode(writer: &writer)").unwrap();
            }
            RustEnumVariant::AnonymousStruct { fields, shared } => {
                let case_name = shared.id.original.to_case(Case::Camel);
                let bindings: Vec<String> = fields
                    .iter()
                    .map(|f| format!("let {}", f.id.renamed.to_case(Case::Camel)))
                    .collect();
                writeln!(
                    out,
                    "        case .{case_name}({}):",
                    bindings.join(", ")
                )
                .unwrap();
                writeln!(out, "            writer.writeU8({idx})").unwrap();
                for f in fields {
                    let field_name = f.id.renamed.to_case(Case::Camel);
                    writeln!(
                        out,
                        "            try {field_name}.encode(writer: &writer)"
                    )
                    .unwrap();
                }
            }
        }
    }
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();

    // decode
    writeln!(
        out,
        "    public static func decode(reader: inout BinaryReader) throws -> {name} {{"
    )
    .unwrap();
    writeln!(out, "        let variant = try reader.readU8()").unwrap();
    writeln!(out, "        switch variant {{").unwrap();
    for (idx, v) in shared.variants.iter().enumerate() {
        match v {
            RustEnumVariant::Unit(shared) => {
                let case_name = shared.id.original.to_case(Case::Camel);
                writeln!(out, "        case {idx}: return .{case_name}").unwrap();
            }
            RustEnumVariant::Tuple { ty, shared } => {
                let case_name = shared.id.original.to_case(Case::Camel);
                let swift_type = rust_type_to_swift(ty);
                writeln!(
                    out,
                    "        case {idx}: return .{case_name}(try {swift_type}.decode(reader: &reader))"
                )
                .unwrap();
            }
            RustEnumVariant::AnonymousStruct { fields, shared } => {
                let case_name = shared.id.original.to_case(Case::Camel);
                writeln!(out, "        case {idx}:").unwrap();
                for f in fields {
                    let field_name = f.id.renamed.to_case(Case::Camel);
                    let swift_type = rust_type_to_swift(&f.ty);
                    writeln!(
                        out,
                        "            let {field_name} = try {swift_type}.decode(reader: &reader)"
                    )
                    .unwrap();
                }
                let args: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let field_name = f.id.renamed.to_case(Case::Camel);
                        format!("{field_name}: {field_name}")
                    })
                    .collect();
                writeln!(
                    out,
                    "            return .{case_name}({})",
                    args.join(", ")
                )
                .unwrap();
            }
        }
    }
    writeln!(
        out,
        "        default: throw WireFormatError.invalidEnumVariant(variant)"
    )
    .unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();

    writeln!(out, "}}").unwrap();

    out
}

/// Generate a complete Swift file from a list of RustItems.
pub fn generate_swift_file(items: &[RustItem], config: &SwiftConfig) -> String {
    let mut out = String::new();

    writeln!(out, "import Foundation").unwrap();
    writeln!(out, "import {}", config.wireformat_module).unwrap();
    writeln!(out).unwrap();

    for item in items {
        match item {
            RustItem::Struct(s) => {
                write!(out, "{}", generate_swift_struct(s, config)).unwrap();
                writeln!(out).unwrap();
            }
            RustItem::Enum(e) => {
                write!(out, "{}", generate_swift_enum(e, config)).unwrap();
                writeln!(out).unwrap();
            }
            _ => {}
        }
    }

    out
}

/// Map a RustType to a Swift type string.
pub fn rust_type_to_swift(ty: &RustType) -> String {
    match ty {
        RustType::Special(special) => match special {
            SpecialRustType::U8 => "UInt8".into(),
            SpecialRustType::U16 => "UInt16".into(),
            SpecialRustType::U32 => "UInt32".into(),
            SpecialRustType::U64 | SpecialRustType::USize => "UInt64".into(),
            SpecialRustType::I8 => "Int8".into(),
            SpecialRustType::I16 => "Int16".into(),
            SpecialRustType::I32 => "Int32".into(),
            SpecialRustType::I64 | SpecialRustType::ISize => "Int64".into(),
            SpecialRustType::F32 => "Float".into(),
            SpecialRustType::F64 => "Double".into(),
            SpecialRustType::Bool => "Bool".into(),
            SpecialRustType::String | SpecialRustType::Char => "String".into(),
            SpecialRustType::Unit => "Void".into(),
            SpecialRustType::Vec(inner) | SpecialRustType::Array(inner, _) | SpecialRustType::Slice(inner) => {
                format!("[{}]", rust_type_to_swift(inner))
            }
            SpecialRustType::Option(inner) => {
                format!("{}?", rust_type_to_swift(inner))
            }
            SpecialRustType::HashMap(k, v) => {
                format!("[{}: {}]", rust_type_to_swift(k), rust_type_to_swift(v))
            }
            _ => "Any".into(),
        },
        RustType::Simple { id } => id.clone(),
        RustType::Generic { id, parameters } => {
            let params: Vec<String> = parameters.iter().map(rust_type_to_swift).collect();
            format!("{}<{}>", id, params.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;

    #[test]
    fn test_generate_swift_struct() {
        let source = r#"
            #[derive(JetStreamWireFormat)]
            struct Point {
                x: u32,
                y: u32,
            }
        "#;
        let items = parse_file(source);
        let config = SwiftConfig::default();
        let swift = generate_swift_struct(
            match &items[0] {
                RustItem::Struct(s) => s,
                _ => panic!("expected struct"),
            },
            &config,
        );
        assert!(swift.contains("public struct Point: WireFormat"));
        assert!(swift.contains("public var x: UInt32"));
        assert!(swift.contains("public var y: UInt32"));
        assert!(swift.contains("x.byteSize() + y.byteSize()"));
        assert!(swift.contains("try x.encode(writer: &writer)"));
        assert!(swift.contains("let x = try UInt32.decode(reader: &reader)"));
    }

    #[test]
    fn test_generate_swift_enum() {
        let source = r#"
            #[derive(JetStreamWireFormat)]
            enum Color {
                Red,
                Green,
                Blue,
            }
        "#;
        let items = parse_file(source);
        let config = SwiftConfig::default();
        let swift = generate_swift_enum(
            match &items[0] {
                RustItem::Enum(e) => e,
                _ => panic!("expected enum"),
            },
            &config,
        );
        assert!(swift.contains("public enum Color: WireFormat"));
        assert!(swift.contains("case red"));
        assert!(swift.contains("case green"));
        assert!(swift.contains("case blue"));
    }
}
