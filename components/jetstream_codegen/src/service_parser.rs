// r[impl jetstream.codegen.service]

use quote::ToTokens;
use typeshare_core::rust_types::RustType;

use crate::parser::parse_rust_type;

/// Parsed service definition from a `#[service]` trait.
#[derive(Debug, Clone)]
pub struct ServiceDef {
    pub name: String,
    pub methods: Vec<MethodDef>,
    pub version: String,
    pub digest: String,
}

/// A single RPC method in a service.
#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub return_type: Option<RustType>,
    pub request_id: u8,
    pub response_id: u8,
}

/// A single parameter of an RPC method.
#[derive(Debug, Clone)]
pub struct ParamDef {
    pub name: String,
    pub ty: RustType,
}

const MESSAGE_ID_START: u8 = 102;

/// Parse a `syn::ItemTrait` into a `ServiceDef`.
///
/// The trait should be annotated with `#[service]` and contain async methods.
/// Context parameters are filtered out (they are not part of the wire protocol).
pub fn parse_service_trait(item: &syn::ItemTrait, digest: &str) -> ServiceDef {
    let name = item.ident.to_string();

    let mut methods = Vec::new();

    for (index, trait_item) in item.items.iter().enumerate() {
        if let syn::TraitItem::Fn(method) = trait_item {
            let method_name = method.sig.ident.to_string();

            let params: Vec<ParamDef> = method
                .sig
                .inputs
                .iter()
                .filter_map(|arg| {
                    if let syn::FnArg::Typed(pat) = arg {
                        let param_name = match pat.pat.as_ref() {
                            syn::Pat::Ident(ident) => ident.ident.to_string(),
                            _ => return None,
                        };

                        // Skip Context type
                        if let syn::Type::Path(type_path) = pat.ty.as_ref() {
                            if let Some(seg) = type_path.path.segments.last() {
                                if seg.ident == "Context" {
                                    return None;
                                }
                            }
                        }

                        Some(ParamDef {
                            name: param_name,
                            ty: parse_rust_type(&pat.ty),
                        })
                    } else {
                        None
                    }
                })
                .collect();

            let return_type = match &method.sig.output {
                syn::ReturnType::Type(_, ty) => {
                    // Unwrap Result<T, E> → T
                    if let syn::Type::Path(type_path) = ty.as_ref() {
                        if let Some(seg) = type_path.path.segments.last() {
                            if seg.ident == "Result" {
                                if let syn::PathArguments::AngleBracketed(
                                    args,
                                ) = &seg.arguments
                                {
                                    if let Some(syn::GenericArgument::Type(
                                        success_type,
                                    )) = args.args.first()
                                    {
                                        return_type_from_syn(success_type)
                                    } else {
                                        None
                                    }
                                } else {
                                    Some(parse_rust_type(ty))
                                }
                            } else {
                                Some(parse_rust_type(ty))
                            }
                        } else {
                            Some(parse_rust_type(ty))
                        }
                    } else {
                        Some(parse_rust_type(ty))
                    }
                }
                syn::ReturnType::Default => None,
            };

            let offset = 2 * index as u8;
            methods.push(MethodDef {
                name: method_name,
                params,
                return_type,
                request_id: MESSAGE_ID_START + offset,
                response_id: MESSAGE_ID_START + offset + 1,
            });
        }
    }

    ServiceDef {
        name,
        methods,
        version: String::new(),
        digest: digest.to_string(),
    }
}

fn return_type_from_syn(ty: &syn::Type) -> Option<RustType> {
    // Check for () / unit
    if let syn::Type::Tuple(tuple) = ty {
        if tuple.elems.is_empty() {
            return None;
        }
    }
    Some(parse_rust_type(ty))
}

/// Parse service traits from a whole .rs file.
pub fn parse_services_from_file(
    source: &str,
    version: &str,
) -> Vec<ServiceDef> {
    let file = match syn::parse_file(source) {
        Ok(f) => f,
        Err(_) => return vec![],
    };

    let mut services = Vec::new();
    for item in &file.items {
        if let syn::Item::Trait(trait_item) = item {
            // Check if it has a #[service] attribute
            let has_service = trait_item
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("service"));
            if has_service {
                let digest_prefix =
                    &sha256::digest(trait_item.to_token_stream().to_string())
                        [0..8];
                let mut svc = parse_service_trait(trait_item, digest_prefix);
                svc.version = version.to_string();
                services.push(svc);
            }
        }
    }
    services
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_service() {
        let source = r#"
            #[service]
            trait Echo {
                async fn echo(&self, msg: String) -> Result<String, Error>;
                async fn add(&self, a: u32, b: u32) -> Result<u32, Error>;
            }
        "#;
        let services = parse_services_from_file(source, "1.0.0");
        assert_eq!(services.len(), 1);
        let svc = &services[0];
        assert_eq!(svc.name, "Echo");
        assert_eq!(svc.methods.len(), 2);

        assert_eq!(svc.methods[0].name, "echo");
        assert_eq!(svc.methods[0].params.len(), 1);
        assert_eq!(svc.methods[0].params[0].name, "msg");
        assert_eq!(svc.methods[0].request_id, 101);
        assert_eq!(svc.methods[0].response_id, 102);

        assert_eq!(svc.methods[1].name, "add");
        assert_eq!(svc.methods[1].params.len(), 2);
        assert_eq!(svc.methods[1].request_id, 103);
        assert_eq!(svc.methods[1].response_id, 104);
    }
}
