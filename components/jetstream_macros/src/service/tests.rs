#![cfg(test)]

use super::{parse_service_attr, service_impl, ServiceAttr};
use core::panic;
use quote::quote;
use syn::parse_quote;

fn run_test_with_filters<F>(test_fn: F)
where
    F: FnOnce() + panic::UnwindSafe,
{
    let filters = vec![
        // Filter for protocol version strings
        (
            r"rs\.jetstream\.proto/\w+/\d+\.\d+\.\d+-[a-f0-9]{8}",
            "rs.jetstream.proto/NAME/VERSION-HASH",
        ),
        // Filter for digest strings
        (r"[a-f0-9]{64}", "DIGEST_HASH"),
    ];

    insta::with_settings!({
        filters => filters,
    }, {
        test_fn();
    });
}

/// r[verify jetstream.macro.source-span]
/// r[verify jetstream.macro.error-type]
/// r[verify jetstream.error-message-frame]
/// r[verify jetstream.macro.client-error]
/// r[verify jetstream.macro.server-error]
#[test]
fn test_simple_service() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    };
    let output = service_impl(input, ServiceAttr::default());
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}

#[test]
fn test_service_with_args() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            async fn ping(&self, message: String) -> Result<String, std::io::Error>;
        }
    };
    let output = service_impl(input, ServiceAttr::default());
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}

#[test]
fn test_async_trait_service_with_args() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            async fn ping(&mut self, message: String) -> Result<String, std::io::Error>;
        }
    };
    let output = service_impl(
        input,
        ServiceAttr {
            is_async_trait: true,
            ..Default::default()
        },
    );
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}

#[test]
fn test_parse_attr_uses_single() {
    let attr = quote! { uses(some::module::*) };
    let parsed = parse_service_attr(attr);
    assert_eq!(parsed.use_paths.len(), 1);
    assert!(!parsed.enable_tracing);
    assert!(!parsed.is_async_trait);
}

#[test]
fn test_parse_attr_uses_multiple() {
    let attr = quote! { uses(some::module::*, other::types::Type) };
    let parsed = parse_service_attr(attr);
    assert_eq!(parsed.use_paths.len(), 2);
}

#[test]
fn test_parse_attr_tracing() {
    let attr = quote! { tracing };
    let parsed = parse_service_attr(attr);
    assert!(parsed.enable_tracing);
    assert!(!parsed.is_async_trait);
    assert_eq!(parsed.use_paths.len(), 0);
}

#[test]
fn test_parse_attr_async_trait() {
    let attr = quote! { async_trait };
    let parsed = parse_service_attr(attr);
    assert!(parsed.is_async_trait);
    assert!(!parsed.enable_tracing);
}

#[test]
fn test_parse_attr_combined() {
    let attr = quote! { tracing, uses(some::module::*), async_trait };
    let parsed = parse_service_attr(attr);
    assert!(parsed.enable_tracing);
    assert!(parsed.is_async_trait);
    assert_eq!(parsed.use_paths.len(), 1);
}

#[test]
fn test_parse_attr_empty() {
    let attr = quote! {};
    let parsed = parse_service_attr(attr);
    assert_eq!(parsed.use_paths.len(), 0);
    assert!(!parsed.enable_tracing);
    assert!(!parsed.is_async_trait);
}

#[test]
fn test_service_with_uses() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    };
    let attr =
        parse_service_attr(quote! { uses(some::module::*, other::Type) });
    let output = service_impl(input, attr);
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}
