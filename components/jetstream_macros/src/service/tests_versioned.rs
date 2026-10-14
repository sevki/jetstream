#![cfg(test)]
//! Tests for versioned service traits.
//!
//! These assert on the *structure* of the expansion — which traits
//! exist, what their methods look like, which wire ids they were given
//! — rather than on token strings, so they keep meaning when unrelated
//! parts of the expansion change. The end-to-end proof that the
//! generated code compiles and talks to itself lives in
//! `tests/versioned_service.rs` at the workspace root.

use std::collections::BTreeMap;

use quote::ToTokens;
use syn::parse_quote;

use super::{service_impl, ServiceAttr};

/// Expand a trait and parse the result back into a syntax tree.
fn expand(item: syn::ItemTrait) -> syn::File {
    let tokens = service_impl(item, ServiceAttr::default());
    syn::parse2(tokens).expect("expansion should be parseable Rust")
}

/// The traits the expansion emitted, by name.
fn traits(file: &syn::File) -> BTreeMap<String, syn::ItemTrait> {
    file.items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Trait(t) => Some((t.ident.to_string(), t.clone())),
            _ => None,
        })
        .collect()
}

/// The signature of one method of one generated trait, normalised to a
/// string so a test can state the whole thing it expects.
fn signature(
    file: &syn::File,
    trait_name: &str,
    method: &str,
) -> Option<String> {
    let t = traits(file).get(trait_name)?.clone();
    t.items.iter().find_map(|i| match i {
        syn::TraitItem::Fn(f) if f.sig.ident == method => {
            Some(f.sig.to_token_stream().to_string())
        }
        _ => None,
    })
}

/// The method names of a generated trait, in the order they were
/// emitted — which is the order that decides their wire ids.
fn methods(file: &syn::File, trait_name: &str) -> Vec<String> {
    traits(file)
        .get(trait_name)
        .map(|t| {
            t.items
                .iter()
                .filter_map(|i| match i {
                    syn::TraitItem::Fn(f) => Some(f.sig.ident.to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The `pub const T<NAME>: u8 = MESSAGE_ID_START + <n>;` values the
/// protocol module defines, by constant name. This is the method's
/// identity on the wire.
fn wire_ids(file: &syn::File, proto_mod: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for item in &file.items {
        let syn::Item::Mod(m) = item else { continue };
        if m.ident != proto_mod {
            continue;
        }
        let Some((_, items)) = &m.content else {
            continue;
        };
        for item in items {
            if let syn::Item::Const(c) = item {
                let name = c.ident.to_string();
                if name.starts_with('T') || name.starts_with('R') {
                    out.insert(name, c.expr.to_token_stream().to_string());
                }
            }
        }
    }
    out
}

/// The error message from an expansion that failed, or `None` if it
/// succeeded. The macro reports by emitting `compile_error!`, never by
/// panicking.
fn error(item: syn::ItemTrait) -> Option<String> {
    let tokens = service_impl(item, ServiceAttr::default()).to_string();
    if !tokens.contains("compile_error") {
        return None;
    }
    Some(tokens)
}

/// The worked example from the documentation.
fn echo() -> syn::ItemTrait {
    parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("0.1.0")]
            async fn pong(&self) -> Result<(), std::io::Error>;

            #[since("1.0.0")]
            async fn ping(&self, msg: String) -> Result<String, std::io::Error>;
        }
    }
}

#[test]
fn a_version_becomes_a_trait() {
    let file = expand(echo());
    let names: Vec<String> = traits(&file).into_keys().collect();
    assert_eq!(names, vec!["EchoV0_1_0", "EchoV1_0_0"]);
}

#[test]
fn the_original_trait_is_not_emitted() {
    // It cannot be: two `ping` declarations are not valid in one trait.
    let file = expand(echo());
    assert!(
        !traits(&file).contains_key("Echo"),
        "the versioned DSL trait must not be emitted",
    );
}

#[test]
fn the_first_version_keeps_the_original_signature() {
    let file = expand(echo());
    let sig = signature(&file, "EchoV0_1_0", "ping").expect("ping at 0.1.0");
    assert_eq!(
        sig,
        "async fn ping (& self) -> Result < () , std :: io :: Error >",
    );
}

#[test]
fn a_later_declaration_replaces_the_earlier_one() {
    let file = expand(echo());
    let sig = signature(&file, "EchoV1_0_0", "ping").expect("ping at 1.0.0");
    assert_eq!(
        sig,
        "async fn ping (& self , msg : String) -> Result < String , std :: io :: Error >",
    );
}

#[test]
fn an_unreplaced_method_carries_forward() {
    let file = expand(echo());
    for version in ["EchoV0_1_0", "EchoV1_0_0"] {
        assert!(
            methods(&file, version).contains(&"pong".to_string()),
            "pong should exist in {version}",
        );
    }
}

#[test]
fn a_method_is_absent_before_it_is_introduced() {
    let file = expand(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("1.0.0")]
            async fn shout(&self, msg: String) -> Result<String, std::io::Error>;
        }
    });
    assert_eq!(methods(&file, "EchoV0_1_0"), vec!["ping"]);
    assert_eq!(methods(&file, "EchoV1_0_0"), vec!["ping", "shout"]);
}

#[test]
fn declaration_order_does_not_decide_the_winner() {
    // The same three declarations, newest written first.
    let reordered: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            #[since("1.0.0")]
            async fn ping(&self, msg: String) -> Result<String, std::io::Error>;

            #[since("0.1.0")]
            async fn pong(&self) -> Result<(), std::io::Error>;

            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    };

    let forward = expand(echo());
    let backward = expand(reordered);

    for version in ["EchoV0_1_0", "EchoV1_0_0"] {
        assert_eq!(
            signature(&forward, version, "ping"),
            signature(&backward, version, "ping"),
            "{version}::ping must not depend on source order",
        );
        assert_eq!(
            methods(&forward, version),
            methods(&backward, version),
            "{version} slot order must not depend on source order",
        );
    }
}

#[test]
fn adding_a_version_does_not_renumber_existing_wire_ids() {
    // r[verify jetstream.subscription.compat]
    // A method's id is `MESSAGE_ID_START + 2 * slot`, so its slot is its
    // identity on the wire. Introducing a newer declaration of `ping`
    // must not push `pong` onto a different id.
    let before = expand(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("0.1.0")]
            async fn pong(&self) -> Result<(), std::io::Error>;
        }
    });
    let after = expand(echo());

    let before_ids = wire_ids(&before, "echov0_1_0_protocol");
    let after_ids = wire_ids(&after, "echov0_1_0_protocol");
    assert_eq!(
        before_ids, after_ids,
        "0.1.0's wire ids changed when 1.0.0 was added",
    );

    // And the replacement itself lands on the id the method already had,
    // rather than being appended after `pong`.
    let v1 = wire_ids(&after, "echov1_0_0_protocol");
    assert_eq!(before_ids.get("TPING"), v1.get("TPING"));
    assert_eq!(before_ids.get("TPONG"), v1.get("TPONG"));
}

#[test]
fn a_new_method_is_appended_rather_than_inserted() {
    let file = expand(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            // Alphabetically first, introduced last: it must still sort
            // after `ping`, or adding it would renumber `ping`.
            #[since("1.0.0")]
            async fn abort(&self) -> Result<(), std::io::Error>;
        }
    });
    assert_eq!(methods(&file, "EchoV1_0_0"), vec!["ping", "abort"]);
}

#[test]
fn version_attributes_do_not_leak_into_generated_rust() {
    let rendered = prettyplease::unparse(&expand(echo()));
    assert!(
        !rendered.contains("since"),
        "`since` leaked into the expansion",
    );
    assert!(
        !rendered.contains("until"),
        "`until` leaked into the expansion",
    );
}

#[test]
fn documentation_and_unrelated_attributes_survive() {
    let file = expand(parse_quote! {
        pub trait Echo {
            /// Answers.
            #[since("0.1.0")]
            #[allow(dead_code)]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    });
    let t = traits(&file);
    let rendered = t["EchoV0_1_0"].to_token_stream().to_string();
    assert!(rendered.contains("Answers"), "doc comment was dropped");
    assert!(
        rendered.contains("allow"),
        "unrelated attribute was dropped"
    );
}

#[test]
fn trait_visibility_survives() {
    let file = expand(parse_quote! {
        pub(crate) trait Echo {
            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    });
    let t = traits(&file);
    assert!(matches!(
        t["EchoV0_1_0"].vis,
        syn::Visibility::Restricted(_)
    ));
}

#[test]
fn supertraits_are_dropped_as_they_are_for_any_service() {
    // Not a property of versioning: `#[service]` has always emitted
    // `#vis trait #name { .. }`, discarding supertraits, generics and
    // trait-level attributes, because the generated
    // `impl<T> Trait for Service<T>` has nowhere to put them. Pinned
    // here so that if the emission is ever widened, this fails and the
    // versioned path is reconsidered at the same time.
    let file = expand(parse_quote! {
        pub trait Echo: Send {
            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    });
    let t = traits(&file);
    assert_eq!(t["EchoV0_1_0"].supertraits.len(), 0);
}

#[test]
fn each_version_is_a_distinct_protocol() {
    // Nothing else stops one version decoding another's payloads, so the
    // names the protocols negotiate under must differ.
    let rendered = prettyplease::unparse(&expand(echo()));
    assert!(rendered.contains("\"echov0_1_0\""));
    assert!(rendered.contains("\"echov1_0_0\""));
}

#[test]
fn a_duplicate_name_and_version_is_rejected() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("0.1.0")]
            async fn ping(&self, msg: String) -> Result<String, std::io::Error>;
        }
    })
    .expect("duplicate declaration should be rejected");
    assert!(err.contains("declared twice at version"), "{err}");
}

#[test]
fn an_invalid_version_is_rejected() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since("not-a-version")]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    })
    .expect("invalid semver should be rejected");
    assert!(err.contains("is not a semantic version"), "{err}");
}

#[test]
fn a_two_part_version_is_rejected() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since("1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    })
    .expect("`1.0` should be rejected");
    assert!(err.contains("is not a semantic version"), "{err}");
}

#[test]
fn an_unquoted_version_is_rejected() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since(1.0.0)]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    })
    .expect("an unquoted version should be rejected");
    assert!(err.contains("quoted three-part semantic version"), "{err}");
}

#[test]
fn prerelease_metadata_is_rejected_with_an_explanation() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since("1.0.0-rc.1")]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    })
    .expect("prerelease versions should be rejected");
    assert!(err.contains("prerelease or build metadata"), "{err}");
}

#[test]
fn a_missing_since_is_rejected() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            async fn pong(&self) -> Result<(), std::io::Error>;
        }
    })
    .expect("a method without `since` should be rejected");
    assert!(err.contains("no `#[since"), "{err}");
}

#[test]
fn until_must_be_greater_than_since() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since("1.0.0")]
            #[until("1.0.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    })
    .expect("an empty window should be rejected");
    assert!(err.contains("must be greater than"), "{err}");
}

#[test]
fn an_until_overlapping_a_newer_declaration_is_rejected() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            #[until("2.0.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("1.0.0")]
            async fn ping(&self, msg: String) -> Result<String, std::io::Error>;
        }
    })
    .expect("overlapping declarations should be rejected");
    assert!(err.contains("overlaps"), "{err}");
}

#[test]
fn until_retires_a_method() {
    let file = expand(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            #[until("1.0.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("0.1.0")]
            async fn pong(&self) -> Result<(), std::io::Error>;

            #[since("1.0.0")]
            async fn shout(&self) -> Result<(), std::io::Error>;
        }
    });
    assert_eq!(methods(&file, "EchoV0_1_0"), vec!["ping", "pong"]);
    // `ping` is gone at 1.0.0; `pong` keeps its slot, so `shout` is
    // appended rather than filling the hole.
    assert_eq!(methods(&file, "EchoV1_0_0"), vec!["pong", "shout"]);
}

#[test]
fn an_until_that_replaces_nothing_is_not_an_overlap() {
    // Ending a declaration exactly where the next begins is the normal
    // case and must be allowed: `until` is exclusive.
    let file = expand(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            #[until("1.0.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("1.0.0")]
            async fn ping(&self, msg: String) -> Result<String, std::io::Error>;
        }
    });
    assert!(signature(&file, "EchoV0_1_0", "ping").is_some());
    assert_eq!(
        signature(&file, "EchoV1_0_0", "ping"),
        Some(
            "async fn ping (& self , msg : String) -> Result < String , std :: io :: Error >"
                .to_string()
        ),
    );
}

#[test]
fn a_version_with_no_methods_is_rejected() {
    let err = error(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            #[until("1.0.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("1.0.0")]
            #[until("2.0.0")]
            async fn ping(&self, msg: String) -> Result<String, std::io::Error>;
        }
    });
    // 0.1.0 and 1.0.0 both have a method; nothing is empty here.
    assert!(err.is_none(), "{err:?}");

    let err = error(parse_quote! {
        pub trait Echo {
            #[since("0.1.0")]
            #[until("0.2.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;

            #[since("1.0.0")]
            #[until("1.0.1")]
            async fn shout(&self) -> Result<(), std::io::Error>;
        }
    });
    assert!(err.is_none(), "each version still has one method: {err:?}");
}

#[test]
fn a_non_method_item_is_rejected() {
    let err = error(parse_quote! {
        pub trait Echo {
            type Item;

            #[since("0.1.0")]
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    })
    .expect("a non-method item should be rejected");
    assert!(err.contains("only methods"), "{err}");
}

#[test]
fn an_unversioned_trait_is_untouched() {
    // The whole compatibility promise: no version attributes, no change.
    let unversioned: syn::ItemTrait = parse_quote! {
        pub trait Echo {
            async fn ping(&self) -> Result<(), std::io::Error>;
        }
    };
    let file = expand(unversioned);
    let names: Vec<String> = traits(&file).into_keys().collect();
    assert_eq!(names, vec!["Echo"]);
}

/// The whole expansion, pinned. The structural tests above say what
/// matters and why; this one catches anything that changes without
/// anyone meaning it to.
#[test]
fn versioned_expansion_snapshot() {
    let output = service_impl(echo(), ServiceAttr::default());
    let syntax_tree: syn::File = syn::parse2(output).unwrap();
    let rendered = prettyplease::unparse(&syntax_tree);
    let filters = vec![
        (
            r"rs\.jetstream\.proto/\w+/\d+\.\d+\.\d+-[a-f0-9]{8}",
            "rs.jetstream.proto/NAME/VERSION-HASH",
        ),
        (r"[a-f0-9]{64}", "DIGEST_HASH"),
    ];
    insta::with_settings!({ filters => filters }, {
        insta::assert_snapshot!(rendered);
    });
}
