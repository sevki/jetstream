use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, DeriveInput, Fields, Ident, Meta, GenericParam, TypeParam, 
    WherePredicate, Type, TypeParamBound, TraitBound, Path, PathSegment, Generics, punctuated::Punctuated};

pub(crate) fn has_skip_attr(field: &syn::Field) -> bool {
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

pub(crate) fn extract_jetstream_type(input: &DeriveInput) -> Option<Ident> {
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

// Add WireFormat bounds to generic type parameters
pub(crate) fn add_wireformat_bounds(
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

pub(crate) fn wire_format_inner(input: DeriveInput) -> TokenStream {
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

    // Previously we used a scope for the module, but now we use a const block
    // let scope = format!("wire_format_{}", container).to_lowercase();
    // let scope = Ident::new(&scope, Span::call_site());
    
    // Use the container directly (not through type alias) to properly handle generics
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub(crate) struct Options {
    /// #[jetstream(with(impl WireFormat))]
    with: Option<syn::Path>,
    /// #[jetstream(with_encode(impl WireFormat))]
    encode: Option<syn::Path>,
    /// #[jetstream(with_decode(impl WireFormat))]
    decode: Option<syn::Path>,
    /// #[jetstream(with_byte_size(FnOnce(As<T> -> u32)))]
    byte_size: Option<syn::Path>,
    /// #[jetstream(from(impl From<WireFormat>))]
    from: Option<syn::Path>,
    /// #[jetstream(into(impl Into<WireFormat>))]
    into: Option<syn::Path>,
    /// #[jetstream(as(impl As<WireFormat>))]
    as_: Option<syn::Path>,
}

pub(crate) fn extract_field_options(field: &syn::Field) -> Options {
    let mut options = Options {
        ..Default::default()
    };

    for attr in &field.attrs {
        if attr.path().is_ident("jetstream") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("with") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let path: syn::Path = content.parse()?;
                    options.with = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("with_encode") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let path: syn::Path = content.parse()?;
                    options.encode = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("with_decode") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let path: syn::Path = content.parse()?;
                    options.decode = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("with_byte_size") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let path: syn::Path = content.parse()?;
                    options.byte_size = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("from") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let path: syn::Path = content.parse()?;
                    options.from = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("into") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let path: syn::Path = content.parse()?;
                    options.into = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("as") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let path: syn::Path = content.parse()?;
                    options.as_ = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("skip") {
                    return Ok(());
                }
                Err(meta.error("unrecognized jetstream attribute"))
            })
            .ok();
        }
    }

    options
}

pub(crate) fn byte_size_sum(data: &Data) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let fields =
                fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                    let field = &f.ident;
                    let span = field.span();
                    let options = extract_field_options(f);

                    if let Some(byte_size_fn) = options.byte_size {
                        quote_spanned! {span=>
                            #byte_size_fn(&self.#field)
                        }
                    } else if let Some(with_fn) = options.with {
                        quote_spanned! {span=>
                            #with_fn::byte_size(&self.#field)
                        }
                    } else {
                        quote_spanned! {span=>
                            WireFormat::byte_size(&self.#field)
                        }
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
                .map(|(i, f)| {
                    let index = syn::Index::from(i);
                    let options = extract_field_options(f);

                    if let Some(byte_size_fn) = options.byte_size {
                        quote! {
                            #byte_size_fn(&self.#index)
                        }
                    } else if let Some(with_fn) = options.with {
                        quote! {
                            #with_fn::byte_size(&self.#index)
                        }
                    } else {
                        quote! {
                            WireFormat::byte_size(&self.#index)
                        }
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
                        .map(|f| (f, &f.ident))
                        .collect::<Vec<_>>();
                    
                    let size_calcs = field_idents.iter().map(|(f, ident)| {
                        let options = extract_field_options(f);
                        if let Some(byte_size_fn) = options.byte_size {
                            quote! { + #byte_size_fn(#ident) }
                        } else if let Some(with_fn) = options.with {
                            quote! { + #with_fn::byte_size(#ident) }
                        } else {
                            quote! { + WireFormat::byte_size(#ident) }
                        }
                    });
                    
                    let field_idents = field_idents.iter().map(|(_, ident)| ident);
                    
                    quote! {
                        Self::#variant_ident { #(ref #field_idents),* } => {
                            1 #(#size_calcs)*
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let refs_with_fields = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| !has_skip_attr(f))
                        .map(|(i, f)| (f, format!("__{}", i)))
                        .collect::<Vec<_>>();
                    
                    let size_calcs = refs_with_fields.iter().map(|(f, name)| {
                        let ident = Ident::new(name, Span::call_site());
                        let options = extract_field_options(f);
                        
                        if let Some(byte_size_fn) = options.byte_size {
                            quote! { + #byte_size_fn(#ident) }
                        } else if let Some(with_fn) = options.with {
                            quote! { + #with_fn::byte_size(#ident) }
                        } else {
                            quote! { + WireFormat::byte_size(#ident) }
                        }
                    });
                    
                    let refs = refs_with_fields.iter().map(|(_, name)|
                        Ident::new(name, Span::call_site())
                    );
                    
                    quote! {
                        Self::#variant_ident(#(ref #refs),*) => {
                            1 #(#size_calcs)*
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

pub(crate) fn encode_wire_format(data: &Data) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let fields =
                fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                    let field = &f.ident;
                    let span = field.span();
                    let options = extract_field_options(f);
                    
                    if let Some(encode_fn) = options.encode {
                        quote_spanned! {span=>
                            #encode_fn(&self.#field, _writer)?;
                        }
                    } else if let Some(with_fn) = options.with {
                        quote_spanned! {span=>
                            #with_fn::encode(&self.#field, _writer)?;
                        }
                    } else if let Some(into_fn) = options.into {
                        quote_spanned! {span=>
                            WireFormat::encode(&(#into_fn(&self.#field)), _writer)?;
                        }
                    } else if let Some(as_fn) = options.as_ {
                        quote_spanned! {span=>
                            WireFormat::encode(&#as_fn(&self.#field), _writer)?;
                        }
                    } else {
                        quote_spanned! {span=>
                            WireFormat::encode(&self.#field, _writer)?;
                        }
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
                .map(|(i, f)| {
                    let index = syn::Index::from(i);
                    let options = extract_field_options(f);
                    
                    if let Some(encode_fn) = options.encode {
                        quote! {
                            #encode_fn(&self.#index, _writer)?;
                        }
                    } else if let Some(with_fn) = options.with {
                        quote! {
                            #with_fn::encode(&self.#index, _writer)?;
                        }
                    } else if let Some(into_fn) = options.into {
                        quote! {
                            WireFormat::encode(&(#into_fn(&self.#index)), _writer)?;
                        }
                    } else if let Some(as_fn) = options.as_ {
                        quote! {
                            WireFormat::encode(&#as_fn(&self.#index), _writer)?;
                        }
                    } else {
                        quote! {
                            WireFormat::encode(&self.#index, _writer)?;
                        }
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
                        let field_idents_with_fields = fields
                            .named
                            .iter()
                            .filter(|f| !has_skip_attr(f))
                            .map(|f| (f, &f.ident))
                            .collect::<Vec<_>>();
                        
                        let encode_stmts = field_idents_with_fields.iter().map(|(f, ident)| {
                            let options = extract_field_options(f);
                            
                            if let Some(encode_fn) = options.encode {
                                quote! { #encode_fn(#ident, _writer)?; }
                            } else if let Some(with_fn) = options.with {
                                quote! { #with_fn::encode(#ident, _writer)?; }
                            } else if let Some(into_fn) = options.into {
                                quote! { WireFormat::encode(&(#into_fn(#ident)), _writer)?; }
                            } else if let Some(as_fn) = options.as_ {
                                quote! { WireFormat::encode(&#as_fn(#ident), _writer)?; }
                            } else {
                                quote! { WireFormat::encode(#ident, _writer)?; }
                            }
                        });
                        
                        let field_idents = field_idents_with_fields.iter().map(|(_, ident)| ident);

                        quote! {
                            Self::#variant_ident { #(ref #field_idents),* } => {
                                WireFormat::encode(&(#idx), _writer)?;
                                #(#encode_stmts)*
                            }
                        }
                    }
                    Fields::Unnamed(ref fields) => {
                        let field_refs_with_fields = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .filter(|(_, f)| !has_skip_attr(f))
                            .map(|(i, f)| (f, format!("__{}", i)))
                            .collect::<Vec<_>>();
                        
                        let encode_stmts = field_refs_with_fields.iter().map(|(f, name)| {
                            let ident = Ident::new(name, Span::call_site());
                            let options = extract_field_options(f);
                            
                            if let Some(encode_fn) = options.encode {
                                quote! { #encode_fn(#ident, _writer)?; }
                            } else if let Some(with_fn) = options.with {
                                quote! { #with_fn::encode(#ident, _writer)?; }
                            } else if let Some(into_fn) = options.into {
                                quote! { WireFormat::encode(&(#into_fn(#ident)), _writer)?; }
                            } else if let Some(as_fn) = options.as_ {
                                quote! { WireFormat::encode(&#as_fn(#ident), _writer)?; }
                            } else {
                                quote! { WireFormat::encode(#ident, _writer)?; }
                            }
                        });
                        
                        let field_refs = field_refs_with_fields.iter().map(|(_, name)| 
                            Ident::new(name, Span::call_site())
                        );
                        
                        quote! {
                            Self::#variant_ident(#(ref #field_refs),*) => {
                                WireFormat::encode(&(#idx), _writer)?;
                                #(#encode_stmts)*
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

pub(crate) fn decode_wire_format(data: &Data, container: &Ident) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            let all_fields = fields.named.iter().collect::<Vec<_>>();
            let non_skipped_values =
                fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                    let field = &f.ident;
                    let span = field.span();
                    let options = extract_field_options(f);
                    
                    if let Some(decode_fn) = options.decode {
                        quote_spanned! {span=>
                            let #field = #decode_fn(_reader)?;
                        }
                    } else if let Some(with_fn) = options.with {
                        quote_spanned! {span=>
                            let #field = #with_fn::decode(_reader)?;
                        }
                    } else if let Some(from_fn) = options.from {
                        quote_spanned! {span=>
                            let #field = #from_fn(WireFormat::decode(_reader)?);
                        }
                    } else {
                        quote_spanned! {span=>
                            let #field = WireFormat::decode(_reader)?;
                        }
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
                .map(|(i, f)| {
                    let ident = Ident::new(&format!("__{}", i), Span::call_site());
                    let options = extract_field_options(f);
                    
                    if let Some(decode_fn) = options.decode {
                        quote! {
                            let #ident = #decode_fn(_reader)?;
                        }
                    } else if let Some(with_fn) = options.with {
                        quote! {
                            let #ident = #with_fn::decode(_reader)?;
                        }
                    } else if let Some(from_fn) = options.from {
                        quote! {
                            let #ident = #from_fn(WireFormat::decode(_reader)?);
                        }
                    } else {
                        quote! {
                            let #ident = WireFormat::decode(_reader)?;
                        }
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
                                let options = extract_field_options(f);
                                
                                if let Some(decode_fn) = options.decode {
                                    quote! { let #field_ident = #decode_fn(_reader)?; }
                                } else if let Some(with_fn) = options.with {
                                    quote! { let #field_ident = #with_fn::decode(_reader)?; }
                                } else if let Some(from_fn) = options.from {
                                    quote! { let #field_ident = #from_fn(WireFormat::decode(_reader)?); }
                                } else {
                                    quote! { let #field_ident = WireFormat::decode(_reader)?; }
                                }
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
                            .map(|(i, f)| {
                                let field_name = Ident::new(&format!("__{}", i), Span::call_site());
                                let options = extract_field_options(f);
                                
                                if let Some(decode_fn) = options.decode {
                                    quote! { let #field_name = #decode_fn(_reader)?; }
                                } else if let Some(with_fn) = options.with {
                                    quote! { let #field_name = #with_fn::decode(_reader)?; }
                                } else if let Some(from_fn) = options.from {
                                    quote! { let #field_name = #from_fn(WireFormat::decode(_reader)?); }
                                } else {
                                    quote! { let #field_name = WireFormat::decode(_reader)?; }
                                }
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

