#![cfg(test)]

use core::panic;

use quote::quote;
use syn::parse_quote;

use super::{parse_service_attr, service_impl, ServiceAttr};

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

/// r[verify jetstream.subscription.surface.declared]
/// r[verify jetstream.subscription.termination.discriminant]
/// r[verify jetstream.subscription.dispatch.declared]
#[test]
fn test_service_with_subscription() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Room {
            async fn post(&self, body: String) -> Result<u64, std::io::Error>;
            #[subscription]
            fn events(&self, from: u64) -> Subscription<Event, Closed>;
        }
    };
    let output = service_impl(input, ServiceAttr::default());
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}

/// r[verify jetstream.subscription.compat.rpc-layer]
/// A protocol with no subscriptions must emit none of the machinery —
/// no cancellation variant, no terminator, no dispatcher hooks — so it
/// compiles exactly as it did before subscriptions existed.
#[test]
fn a_protocol_without_subscriptions_gains_nothing() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            async fn ping(&self, message: String) -> Result<String, std::io::Error>;
        }
    };
    let output = service_impl(input, ServiceAttr::default()).to_string();
    for absent in [
        "Cancel",
        "CancelAck",
        "RDONE",
        "TCANCEL",
        "RCANCEL",
        "is_streaming",
        "rpc_stream",
        "cancelled_terminator",
        "enum Done",
    ] {
        assert!(
            !output.contains(absent),
            "a protocol with no subscriptions emitted `{absent}`"
        );
    }
}

/// r[verify jetstream.subscription.compat]
/// The per-method ids are untouched by a streaming method: the
/// terminator and cancellation live below `MESSAGE_ID_START`, which is
/// the whole reason they were put there.
#[test]
fn a_subscription_costs_no_per_method_id() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Room {
            #[subscription]
            fn events(&self, from: u64) -> Subscription<Event, Closed>;
            async fn post(&self, body: String) -> Result<u64, std::io::Error>;
        }
    };
    let output = service_impl(input, ServiceAttr::default()).to_string();
    // `events` is index 0 and `post` index 1, exactly as they would be
    // if neither streamed.
    assert!(output.contains("TEVENTS : u8 = MESSAGE_ID_START + 0u8"));
    assert!(output.contains("TPOST : u8 = MESSAGE_ID_START + 2u8"));
}

/// A `#[subscription]` method has to say what it yields and what it
/// ends with, and the error has to say so rather than failing somewhere
/// inside the generated code.
#[test]
fn a_subscription_must_return_a_subscription() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Room {
            #[subscription]
            fn events(&self, from: u64) -> Vec<Event>;
        }
    };
    let output = service_impl(input, ServiceAttr::default()).to_string();
    assert!(
        output.contains("must return `Subscription<Item, Done>`"),
        "expected a compile error naming the required return type: {output}"
    );
}

/// r[verify jetstream.subscription.lossy.declared]
/// Refused rather than quietly served reliably: a caller told they have
/// a lossy subscription, who in fact has a reliable one, has been
/// misinformed about delivery.
#[test]
fn lossy_is_refused_until_it_exists() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Room {
            #[subscription(lossy)]
            fn presence(&self) -> Subscription<Who, Closed>;
        }
    };
    let output = service_impl(input, ServiceAttr::default()).to_string();
    assert!(
        output.contains("not implemented yet"),
        "expected lossy to be refused: {output}"
    );
}
