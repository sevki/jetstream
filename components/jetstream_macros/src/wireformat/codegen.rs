use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Data, Fields, Ident};

use crate::utils::error;
use jetstream_codegen::attributes::{extract_field_options, has_skip_attr};

pub fn byte_size_sum(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => generate_struct_byte_size(&data.fields),
        Data::Enum(ref data) => generate_enum_byte_size(data),
        Data::Union(_) => error::unsupported_data_type(),
    }
}

pub fn encode_wire_format(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => generate_struct_encode(&data.fields),
        Data::Enum(ref data) => generate_enum_encode(data),
        Data::Union(_) => error::unsupported_data_type(),
    }
}

pub fn decode_wire_format(data: &Data, container: &Ident) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            generate_struct_decode(&data.fields, container)
        }
        Data::Enum(ref data) => generate_enum_decode(data, container),
        Data::Union(_) => error::unsupported_data_type(),
    }
}

// Struct implementations
fn generate_struct_byte_size(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            let fields = fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                let field = &f.ident;
                let span = field.span();
                let options = extract_field_options(f);

                if let Some(byte_size_fn) = options.byte_size {
                    quote_spanned! {span=> #byte_size_fn(&self.#field) }
                } else if let Some(with_fn) = options.with {
                    quote_spanned! {span=> #with_fn::byte_size(&self.#field) }
                } else {
                    quote_spanned! {span=> WireFormat::byte_size(&self.#field) }
                }
            });

            quote! { 0 #(+ #fields)* }
        }
        Fields::Unnamed(ref unnamed) => {
            let fields = unnamed
                .unnamed
                .iter()
                .enumerate()
                .filter(|(_, f)| !has_skip_attr(f))
                .map(|(i, f)| {
                    let index = syn::Index::from(i);
                    let options = extract_field_options(f);

                    if let Some(byte_size_fn) = options.byte_size {
                        quote! { #byte_size_fn(&self.#index) }
                    } else if let Some(with_fn) = options.with {
                        quote! { #with_fn::byte_size(&self.#index) }
                    } else {
                        quote! { WireFormat::byte_size(&self.#index) }
                    }
                });

            quote! { 0 #(+ #fields)* }
        }
        Fields::Unit => quote! { 0 },
    }
}

fn generate_struct_encode(fields: &Fields) -> TokenStream {
    let encode_fields = match fields {
        Fields::Named(ref fields) => {
            let fields = fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                let field = &f.ident;
                let span = field.span();
                let options = extract_field_options(f);
                
                if let Some(encode_fn) = options.encode {
                    quote_spanned! {span=> #encode_fn(&self.#field, _writer)?; }
                } else if let Some(with_fn) = options.with {
                    quote_spanned! {span=> #with_fn::encode(&self.#field, _writer)?; }
                } else if let Some(into_fn) = options.into {
                    quote_spanned! {span=> WireFormat::encode(&(#into_fn(&self.#field)), _writer)?; }
                } else if let Some(as_fn) = options.as_ {
                    quote_spanned! {span=> WireFormat::encode(&#as_fn(&self.#field), _writer)?; }
                } else {
                    quote_spanned! {span=> WireFormat::encode(&self.#field, _writer)?; }
                }
            });
            quote! { #(#fields)* }
        }
        Fields::Unnamed(ref unnamed) => {
            let fields = unnamed
                .unnamed
                .iter()
                .enumerate()
                .filter(|(_, f)| !has_skip_attr(f))
                .map(|(i, f)| {
                    let index = syn::Index::from(i);
                    let options = extract_field_options(f);
                    
                    if let Some(encode_fn) = options.encode {
                        quote! { #encode_fn(&self.#index, _writer)?; }
                    } else if let Some(with_fn) = options.with {
                        quote! { #with_fn::encode(&self.#index, _writer)?; }
                    } else if let Some(into_fn) = options.into {
                        quote! { WireFormat::encode(&(#into_fn(&self.#index)), _writer)?; }
                    } else if let Some(as_fn) = options.as_ {
                        quote! { WireFormat::encode(&#as_fn(&self.#index), _writer)?; }
                    } else {
                        quote! { WireFormat::encode(&self.#index, _writer)?; }
                    }
                });
            quote! { #(#fields)* }
        }
        Fields::Unit => quote! {},
    };

    quote! {
        #encode_fields
        Ok(())
    }
}

fn generate_struct_decode(fields: &Fields, container: &Ident) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            let all_fields = fields.named.iter().collect::<Vec<_>>();
            let non_skipped_values =
                fields.named.iter().filter(|f| !has_skip_attr(f)).map(|f| {
                    let field = &f.ident;
                    let span = field.span();
                    let options = extract_field_options(f);
                    
                    if let Some(decode_fn) = options.decode {
                        quote_spanned! {span=> let #field = #decode_fn(_reader)?; }
                    } else if let Some(with_fn) = options.with {
                        quote_spanned! {span=> let #field = #with_fn::decode(_reader)?; }
                    } else if let Some(from_fn) = options.from {
                        quote_spanned! {span=> let #field = #from_fn(WireFormat::decode(_reader)?); }
                    } else {
                        quote_spanned! {span=> let #field = WireFormat::decode(_reader)?; }
                    }
                });

            let members = all_fields.iter().map(|f| {
                let field = &f.ident;
                if has_skip_attr(f) {
                    quote! { #field: Default::default(), }
                } else {
                    quote! { #field: #field, }
                }
            });

            quote! {
                #(#non_skipped_values)*
                Ok(#container {
                    #(#members)*
                })
            }
        }
        Fields::Unnamed(unnamed) => {
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
                        quote! { let #ident = #decode_fn(_reader)?; }
                    } else if let Some(with_fn) = options.with {
                        quote! { let #ident = #with_fn::decode(_reader)?; }
                    } else if let Some(from_fn) = options.from {
                        quote! { let #ident = #from_fn(WireFormat::decode(_reader)?); }
                    } else {
                        quote! { let #ident = WireFormat::decode(_reader)?; }
                    }
                });

            let members = all_fields.iter().map(|(i, is_skipped)| {
                if *is_skipped {
                    quote! { Default::default() }
                } else {
                    let ident =
                        Ident::new(&format!("__{}", i), Span::call_site());
                    quote! { #ident }
                }
            });

            quote! {
                #(#non_skipped_values)*
                Ok(#container(#(#members,)*))
            }
        }
        Fields::Unit => quote! { Ok(#container) },
    }
}

// Enum implementations
fn generate_enum_byte_size(data: &syn::DataEnum) -> TokenStream {
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

                let refs = refs_with_fields
                    .iter()
                    .map(|(_, name)| Ident::new(name, Span::call_site()));

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
}

fn generate_enum_encode(data: &syn::DataEnum) -> TokenStream {
    let variants = data.variants.iter().enumerate().map(|(idx, variant)| {
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
}

fn generate_enum_decode(
    data: &syn::DataEnum,
    _container: &Ident,
) -> TokenStream {
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
}
