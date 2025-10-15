use syn::{Attribute, TraitItemFn};

/// Extract tracing-related attributes from a method.
/// This includes #[instrument], #[trace], #[debug], #[info], #[warn], #[error]
pub fn extract_tracing_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| {
            attr.path().is_ident("instrument")
                || attr.path().is_ident("trace")
                || attr.path().is_ident("debug")
                || attr.path().is_ident("info")
                || attr.path().is_ident("warn")
                || attr.path().is_ident("error")
        })
        .cloned()
        .collect()
}

/// Extract tracing attributes from a trait method
pub fn extract_method_tracing_attrs(method: &TraitItemFn) -> Vec<Attribute> {
    extract_tracing_attrs(&method.attrs)
}

pub fn take_attributes(
    items: &[TraitItemFn],
) -> Vec<(TraitItemFn, Vec<Attribute>)> {
    let mut result = Vec::new();
    for item in items {
        let attrs = extract_method_tracing_attrs(item);
        let mut item = item.clone();
        item.attrs.retain(|attr| !attrs.contains(attr));
        result.push((item, attrs));
    }
    result
}
