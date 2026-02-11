use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Generics, WherePredicate, Type, TypeParamBound, TraitBound, Path, PathSegment, GenericParam, TypeParam, punctuated::Punctuated};

use jetstream_codegen::attributes::extract_jetstream_type;
use super::codegen::{byte_size_sum, decode_wire_format, encode_wire_format};

// Add WireFormat bounds to generic type parameters
fn add_wireformat_bounds(
    generics: &Generics,
    predicates: &mut Punctuated<WherePredicate, syn::token::Comma>,
) {
    for param in &generics.params {
        if let GenericParam::Type(TypeParam { ident, .. }) = param {
            let ty = Type::Path(syn::TypePath {
                qself: None,
                path: Path {
                    leading_colon: None,
                    segments: {
                        let mut segments = Punctuated::new();
                        segments.push(PathSegment {
                            ident: ident.clone(),
                            arguments: syn::PathArguments::None,
                        });
                        segments
                    },
                },
            });

            // Create the WireFormat trait bound
            let trait_path = syn::parse_str::<Path>("jetstream_wireformat::WireFormat").unwrap();
            let trait_bound = TypeParamBound::Trait(TraitBound {
                paren_token: None,
                modifier: syn::TraitBoundModifier::None,
                lifetimes: None,
                path: trait_path,
            });

            // Create the where predicate: T: WireFormat
            let mut bounds = Punctuated::new();
            bounds.push(trait_bound);
            
            let predicate = WherePredicate::Type(syn::PredicateType {
                lifetimes: None,
                bounded_ty: ty,
                colon_token: syn::token::Colon::default(),
                bounds,
            });
            
            predicates.push(predicate);
        }
    }
}

pub fn wire_format_inner(input: DeriveInput) -> TokenStream {
    let jetstream_type = extract_jetstream_type(&input);
    let container = input.ident.clone();
    
    // Extract generics information
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    
    // Create a where clause for WireFormat bounds on generic types
    let where_clause = match where_clause {
        Some(where_clause) => {
            let mut predicates = where_clause.predicates.clone();
            add_wireformat_bounds(&generics, &mut predicates);
            Some(syn::WhereClause {
                where_token: syn::token::Where::default(),
                predicates,
            })
        }
        None if !generics.params.is_empty() => {
            let mut predicates = syn::punctuated::Punctuated::new();
            add_wireformat_bounds(&generics, &mut predicates);
            Some(syn::WhereClause {
                where_token: syn::token::Where::default(),
                predicates,
            })
        }
        None => None,
    };
    
    let where_clause_tokens = where_clause.map_or_else(|| quote! {}, |wc| quote! { #wc });

    // Generate message type implementation
    let message_impl = if let Some(msg_type) = jetstream_type {
        quote! {
           impl #impl_generics jetstream_wireformat::Message for #container #ty_generics #where_clause_tokens {
               const MESSAGE_TYPE: u8 = #msg_type;
           }
        }
    } else {
        quote! {}
    };

    let byte_size_impl = byte_size_sum(&input.data);
    let encode_impl = encode_wire_format(&input.data);
    let decode_impl = decode_wire_format(&input.data, &container);
    
    // Use const block for hygiene
    quote! {
        const _: () = {
            extern crate std;
            use std::io;
            use std::result::Result::Ok;
            use jetstream_wireformat::WireFormat;

            impl #impl_generics WireFormat for #container #ty_generics #where_clause_tokens {
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
        };
    }
}