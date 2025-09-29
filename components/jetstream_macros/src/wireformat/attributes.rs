use syn::{DeriveInput, Ident, Meta};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Options {
    /// #[jetstream(with(impl WireFormat))]
    pub with: Option<syn::Path>,
    /// #[jetstream(with_encode(impl WireFormat))]
    pub encode: Option<syn::Path>,
    /// #[jetstream(with_decode(impl WireFormat))]
    pub decode: Option<syn::Path>,
    /// #[jetstream(with_byte_size(FnOnce(As<T> -> u32)))]
    pub byte_size: Option<syn::Path>,
    /// #[jetstream(from(impl From<WireFormat>))]
    pub from: Option<syn::Path>,
    /// #[jetstream(into(impl Into<WireFormat>))]
    pub into: Option<syn::Path>,
    /// #[jetstream(as(impl As<WireFormat>))]
    pub as_: Option<syn::Path>,
}

pub fn has_skip_attr(field: &syn::Field) -> bool {
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

pub fn extract_jetstream_type(input: &DeriveInput) -> Option<Ident> {
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

pub fn extract_field_options(field: &syn::Field) -> Options {
    let mut options = Options::default();

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