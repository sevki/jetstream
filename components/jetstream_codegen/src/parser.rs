// r[impl jetstream.codegen.parser]

use std::collections::HashMap;

use typeshare_core::rust_types::*;

use crate::attributes::has_skip_attr;

/// Parse a syn::Type into a typeshare-core RustType.
///
/// r[jetstream.codegen.type-map]
/// Unlike typeshare-core's built-in parser, this handles all wire format types
/// including u64, i64, u128, i128, usize, and isize.
pub fn parse_rust_type(ty: &syn::Type) -> RustType {
    match ty {
        syn::Type::Path(type_path) => {
            let segment = match type_path.path.segments.last() {
                Some(s) => s,
                None => return RustType::Simple { id: "Unknown".into() },
            };
            let ident = segment.ident.to_string();
            let generic_args: Vec<RustType> = match &segment.arguments {
                syn::PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .filter_map(|arg| {
                        if let syn::GenericArgument::Type(ty) = arg {
                            Some(parse_rust_type(ty))
                        } else {
                            None
                        }
                    })
                    .collect(),
                _ => vec![],
            };

            match ident.as_str() {
                "u8" => RustType::Special(SpecialRustType::U8),
                "u16" => RustType::Special(SpecialRustType::U16),
                "u32" => RustType::Special(SpecialRustType::U32),
                "u64" => RustType::Special(SpecialRustType::U64),
                "i8" => RustType::Special(SpecialRustType::I8),
                "i16" => RustType::Special(SpecialRustType::I16),
                "i32" => RustType::Special(SpecialRustType::I32),
                "i64" => RustType::Special(SpecialRustType::I64),
                "usize" => RustType::Special(SpecialRustType::USize),
                "isize" => RustType::Special(SpecialRustType::ISize),
                "f32" => RustType::Special(SpecialRustType::F32),
                "f64" => RustType::Special(SpecialRustType::F64),
                "bool" => RustType::Special(SpecialRustType::Bool),
                "String" | "str" => RustType::Special(SpecialRustType::String),
                "Vec" => {
                    let inner = generic_args
                        .into_iter()
                        .next()
                        .unwrap_or(RustType::Special(SpecialRustType::U8));
                    RustType::Special(SpecialRustType::Vec(Box::new(inner)))
                }
                "Option" => {
                    let inner = generic_args
                        .into_iter()
                        .next()
                        .unwrap_or(RustType::Special(SpecialRustType::Unit));
                    RustType::Special(SpecialRustType::Option(Box::new(inner)))
                }
                "HashMap" | "BTreeMap" => {
                    let mut args = generic_args.into_iter();
                    let key = args
                        .next()
                        .unwrap_or(RustType::Special(SpecialRustType::String));
                    let val = args
                        .next()
                        .unwrap_or(RustType::Special(SpecialRustType::String));
                    RustType::Special(SpecialRustType::HashMap(Box::new(key), Box::new(val)))
                }
                "Box" | "Arc" | "Rc" => generic_args
                    .into_iter()
                    .next()
                    .unwrap_or(RustType::Special(SpecialRustType::Unit)),
                _ => {
                    if generic_args.is_empty() {
                        RustType::Simple { id: ident }
                    } else {
                        RustType::Generic {
                            id: ident,
                            parameters: generic_args,
                        }
                    }
                }
            }
        }
        syn::Type::Reference(reference) => parse_rust_type(&reference.elem),
        syn::Type::Tuple(tuple) if tuple.elems.is_empty() => {
            RustType::Special(SpecialRustType::Unit)
        }
        _ => RustType::Simple {
            id: "Unknown".into(),
        },
    }
}

/// Parse a syn::DeriveInput into a typeshare-core RustItem.
pub fn parse_derive_input(input: &syn::DeriveInput) -> Option<RustItem> {
    let name = input.ident.to_string();
    let generic_types: Vec<String> = input
        .generics
        .type_params()
        .map(|tp| tp.ident.to_string())
        .collect();

    match &input.data {
        syn::Data::Struct(data) => {
            let fields = parse_fields(&data.fields);
            Some(RustItem::Struct(RustStruct {
                id: Id {
                    original: name.clone(),
                    renamed: name,
                    serde_rename: false,
                },
                generic_types,
                fields,
                comments: vec![],
                decorators: Default::default(),
                is_redacted: false,
            }))
        }
        syn::Data::Enum(data) => {
            let variants: Vec<RustEnumVariant> = data
                .variants
                .iter()
                .map(|v| {
                    let shared = RustEnumVariantShared {
                        id: Id {
                            original: v.ident.to_string(),
                            renamed: v.ident.to_string(),
                            serde_rename: false,
                        },
                        comments: vec![],
                    };
                    match &v.fields {
                        syn::Fields::Unit => RustEnumVariant::Unit(shared),
                        syn::Fields::Unnamed(fields) => {
                            if fields.unnamed.len() == 1 {
                                let f = fields.unnamed.first().unwrap();
                                RustEnumVariant::Tuple {
                                    ty: parse_rust_type(&f.ty),
                                    shared,
                                }
                            } else {
                                // Multiple unnamed fields — treat as anonymous struct
                                // with generated field names
                                let rust_fields: Vec<RustField> = fields
                                    .unnamed
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, f)| !has_skip_attr(f))
                                    .map(|(i, f)| RustField {
                                        id: Id {
                                            original: format!("field_{i}"),
                                            renamed: format!("field_{i}"),
                                            serde_rename: false,
                                        },
                                        ty: parse_rust_type(&f.ty),
                                        comments: vec![],
                                        has_default: false,
                                        decorators: HashMap::new(),
                                    })
                                    .collect();
                                RustEnumVariant::AnonymousStruct {
                                    fields: rust_fields,
                                    shared,
                                }
                            }
                        }
                        syn::Fields::Named(fields) => {
                            let rust_fields = fields
                                .named
                                .iter()
                                .filter(|f| !has_skip_attr(f))
                                .map(|f| {
                                    let field_name =
                                        f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                                    RustField {
                                        id: Id {
                                            original: field_name.clone(),
                                            renamed: field_name,
                                            serde_rename: false,
                                        },
                                        ty: parse_rust_type(&f.ty),
                                        comments: vec![],
                                        has_default: false,
                                        decorators: HashMap::new(),
                                    }
                                })
                                .collect();
                            RustEnumVariant::AnonymousStruct {
                                fields: rust_fields,
                                shared,
                            }
                        }
                    }
                })
                .collect();

            let is_all_unit = variants
                .iter()
                .all(|v| matches!(v, RustEnumVariant::Unit(_)));

            let shared = RustEnumShared {
                id: Id {
                    original: name.clone(),
                    renamed: name,
                    serde_rename: false,
                },
                generic_types,
                comments: vec![],
                variants,
                decorators: Default::default(),
                is_recursive: false,
                is_redacted: false,
            };

            if is_all_unit {
                Some(RustItem::Enum(RustEnum::Unit(shared)))
            } else {
                Some(RustItem::Enum(RustEnum::Algebraic {
                    tag_key: "type".into(),
                    content_key: "content".into(),
                    shared,
                }))
            }
        }
        syn::Data::Union(_) => None,
    }
}

fn parse_fields(fields: &syn::Fields) -> Vec<RustField> {
    match fields {
        syn::Fields::Named(named) => named
            .named
            .iter()
            .filter(|f| !has_skip_attr(f))
            .map(|f| {
                let field_name = f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                RustField {
                    id: Id {
                        original: field_name.clone(),
                        renamed: field_name,
                        serde_rename: false,
                    },
                    ty: parse_rust_type(&f.ty),
                    comments: vec![],
                    has_default: false,
                    decorators: HashMap::new(),
                }
            })
            .collect(),
        syn::Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .filter(|(_, f)| !has_skip_attr(f))
            .map(|(i, f)| RustField {
                id: Id {
                    original: format!("field_{i}"),
                    renamed: format!("field_{i}"),
                    serde_rename: false,
                },
                ty: parse_rust_type(&f.ty),
                comments: vec![],
                has_default: false,
                decorators: HashMap::new(),
            })
            .collect(),
        syn::Fields::Unit => vec![],
    }
}

/// Check if a syn::DeriveInput has `#[derive(JetStreamWireFormat)]`.
fn has_wireformat_derive(item: &syn::Item) -> bool {
    let attrs = match item {
        syn::Item::Struct(s) => &s.attrs,
        syn::Item::Enum(e) => &e.attrs,
        _ => return false,
    };
    attrs.iter().any(|attr| {
        if attr.path().is_ident("derive") {
            if let Ok(nested) =
                attr.parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
            {
                return nested.iter().any(|path| {
                    path.segments
                        .last()
                        .is_some_and(|seg| seg.ident == "JetStreamWireFormat")
                });
            }
        }
        false
    })
}

/// Parse a whole .rs file and extract all `#[derive(JetStreamWireFormat)]` types.
pub fn parse_file(source: &str) -> Vec<RustItem> {
    let file = match syn::parse_file(source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut items = Vec::new();
    for item in &file.items {
        if !has_wireformat_derive(item) {
            continue;
        }
        let derive_input: syn::DeriveInput = match item {
            syn::Item::Struct(s) => syn::DeriveInput {
                attrs: s.attrs.clone(),
                vis: s.vis.clone(),
                ident: s.ident.clone(),
                generics: s.generics.clone(),
                data: syn::Data::Struct(syn::DataStruct {
                    struct_token: s.struct_token,
                    fields: s.fields.clone(),
                    semi_token: s.semi_token,
                }),
            },
            syn::Item::Enum(e) => syn::DeriveInput {
                attrs: e.attrs.clone(),
                vis: e.vis.clone(),
                ident: e.ident.clone(),
                generics: e.generics.clone(),
                data: syn::Data::Enum(syn::DataEnum {
                    enum_token: e.enum_token,
                    brace_token: e.brace_token,
                    variants: e.variants.clone(),
                }),
            },
            _ => continue,
        };

        if let Some(rust_item) = parse_derive_input(&derive_input) {
            items.push(rust_item);
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_struct() {
        let source = r#"
            #[derive(JetStreamWireFormat)]
            struct Point {
                x: u32,
                y: u32,
            }
        "#;
        let items = parse_file(source);
        assert_eq!(items.len(), 1);
        match &items[0] {
            RustItem::Struct(s) => {
                assert_eq!(s.id.original, "Point");
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].id.original, "x");
                assert_eq!(s.fields[1].id.original, "y");
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
            #[derive(JetStreamWireFormat)]
            enum Color {
                Red,
                Green,
                Blue,
            }
        "#;
        let items = parse_file(source);
        assert_eq!(items.len(), 1);
        match &items[0] {
            RustItem::Enum(RustEnum::Unit(shared)) => {
                assert_eq!(shared.id.original, "Color");
                assert_eq!(shared.variants.len(), 3);
            }
            _ => panic!("expected unit enum"),
        }
    }

    #[test]
    fn test_parse_algebraic_enum() {
        let source = r#"
            #[derive(JetStreamWireFormat)]
            enum Shape {
                Circle(u32),
                Rectangle { width: u32, height: u32 },
            }
        "#;
        let items = parse_file(source);
        assert_eq!(items.len(), 1);
        match &items[0] {
            RustItem::Enum(RustEnum::Algebraic { shared, .. }) => {
                assert_eq!(shared.id.original, "Shape");
                assert_eq!(shared.variants.len(), 2);
            }
            _ => panic!("expected algebraic enum"),
        }
    }

    #[test]
    fn test_parse_rust_type_primitives() {
        let ty: syn::Type = syn::parse_str("u64").unwrap();
        assert_eq!(parse_rust_type(&ty), RustType::Special(SpecialRustType::U64));

        let ty: syn::Type = syn::parse_str("i64").unwrap();
        assert_eq!(parse_rust_type(&ty), RustType::Special(SpecialRustType::I64));

        let ty: syn::Type = syn::parse_str("Vec<String>").unwrap();
        assert!(matches!(parse_rust_type(&ty), RustType::Special(SpecialRustType::Vec(_))));

        let ty: syn::Type = syn::parse_str("Option<u32>").unwrap();
        assert!(matches!(parse_rust_type(&ty), RustType::Special(SpecialRustType::Option(_))));

        let ty: syn::Type = syn::parse_str("Box<u32>").unwrap();
        assert_eq!(parse_rust_type(&ty), RustType::Special(SpecialRustType::U32));
    }
}
