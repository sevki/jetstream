#![cfg(test)]

use super::service_impl;
use core::panic;
use syn::parse_quote;

fn run_test_with_filters<F>(test_fn: F)
where
    F: FnOnce() + panic::UnwindSafe,
{
    let filters = vec![
        // Filter for protocol version strings
        (
            r"dev\.branch\.jetstream\.proto/\w+/\d+\.\d+\.\d+-[a-f0-9]{8}",
            "dev.branch.jetstream.proto/NAME/VERSION-HASH",
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

#[test]
fn test_simple_service() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    };
    let output = service_impl(input, false, false);
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
    let output = service_impl(input, false, false);
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
    let output = service_impl(input, true, false);
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}
