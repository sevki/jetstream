#![cfg(test)]

use super::service_impl;
use syn::parse_quote;

fn run_test_with_filters<F>(test_fn: F)
where
    F: FnOnce() + std::panic::UnwindSafe,
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
fn test_service_with_instrument_attribute() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            #[instrument(skip(self))]
            async fn ping(&mut self, message: String) -> Result<String, std::io::Error>;
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
fn test_service_with_tracing_enabled() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            async fn ping(&mut self, message: String) -> Result<String>;
        }
    };
    let output = service_impl(input, false, true);
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}

#[test]
fn test_service_with_custom_instrument() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            #[instrument(
                name = "echo_ping",
                skip(self),
                fields(message_len = message.len()),
                level = "debug"
            )]
            async fn ping(&mut self, message: String) -> Result<String, std::io::Error>;
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
fn test_service_with_tracing_and_manual_override() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            #[instrument(level = "trace")]
            async fn ping(&mut self, message: String) -> Result<String, std::io::Error>;

            async fn pong(&mut self) -> Result<(), std::io::Error>;
        }
    };
    let output = service_impl(input, false, true);
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}

#[test]
fn test_service_multiple_methods_with_tracing() {
    let input: syn::ItemTrait = parse_quote! {
        pub trait ComplexService {
            #[instrument(skip(self, password))]
            async fn login(&mut self, username: String, password: String) -> Result<String, std::io::Error>;

            async fn logout(&mut self) -> Result<(), std::io::Error>;

            #[instrument(level = "debug")]
            async fn get_status(&self) -> Result<String, std::io::Error>;
        }
    };
    let output = service_impl(input, false, true);
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let output_str = prettyplease::unparse(&syntax_tree);
    run_test_with_filters(|| {
        insta::assert_snapshot!(output_str);
    })
}
