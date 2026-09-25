//! Versioned service traits.
//!
//! One `#[service]` trait can describe the *history* of an API rather
//! than a single shape of it:
//!
//! ```ignore
//! #[service]
//! pub trait Echo {
//!     #[since("0.1.0")]
//!     async fn ping(&self) -> Result<()>;
//!     #[since("0.1.0")]
//!     async fn pong(&self) -> Result<()>;
//!     #[since("1.0.0")]
//!     async fn ping(&self, msg: String) -> Result<String>;
//! }
//! ```
//!
//! That trait is a DSL, not Rust: two `ping` declarations cannot both
//! exist in one trait. So it is never emitted. Instead each distinct
//! `since` version produces a *snapshot* — an ordinary trait naming the
//! methods as they stood at that version — and each snapshot goes
//! through the same expansion an unversioned `#[service]` trait does.
//! There is one expansion path, not two.
//!
//! ## Selecting a declaration
//!
//! For a snapshot at version `V`, a declaration is active when
//! `since <= V` and, if an `until` is given, `V < until`. Among the
//! active declarations of one method name the one with the greatest
//! `since` wins, so a later declaration replaces an earlier one. A
//! method whose earliest `since` is above `V` is simply absent.
//!
//! Source order never decides this. Declarations may be written in any
//! order; only the versions are consulted.
//!
//! ## Why slot order is not selection order
//!
//! Wire message ids are `MESSAGE_ID_START + 2 * slot`, so a method's
//! slot within a snapshot *is* its identity on the wire. Ordering
//! snapshots by their selected declarations would renumber every method
//! below a replacement the moment one was added.
//!
//! So a method's slot is fixed by where it was *introduced* — the
//! earliest `since` among its declarations, then that declaration's
//! position in the source. Adding a newer declaration of an existing
//! method cannot move it, and a genuinely new method sorts after
//! everything introduced before it. For a trait whose methods all share
//! one version this is exactly source order, which is what the
//! unversioned macro already does.

use std::collections::{BTreeMap, HashMap, HashSet};

use proc_macro2::Span;
use quote::format_ident;
use semver::Version;
use syn::{
    spanned::Spanned, Attribute, Ident, ItemTrait, TraitItem, TraitItemFn,
};

/// The attribute introducing a declaration.
const SINCE: &str = "since";
/// The attribute retiring one.
const UNTIL: &str = "until";

/// How a version must be written, quoted in diagnostics so the fix is
/// visible without going to the docs.
const VERSION_FORM: &str = "expected a quoted three-part semantic version, \
                            as in #[since(\"1.2.3\")]";

fn is_version_attr(attr: &Attribute) -> bool {
    attr.path().is_ident(SINCE) || attr.path().is_ident(UNTIL)
}

/// Whether this trait uses the versioning DSL at all.
///
/// Versioned expansion is opt-in by use: a trait with no `#[since]` and
/// no `#[until]` anywhere expands exactly as it did before any of this
/// existed.
pub(crate) fn is_versioned(item: &ItemTrait) -> bool {
    item.items.iter().any(|i| match i {
        TraitItem::Fn(f) => f.attrs.iter().any(is_version_attr),
        _ => false,
    })
}

/// One declaration of one method, at one version.
struct Decl {
    /// Position in the source trait. Breaks ties when two methods are
    /// introduced at the same version, and nothing else.
    source_index: usize,
    since: Version,
    /// Exclusive. `None` means the declaration stands until a newer one
    /// of the same name replaces it.
    until: Option<Version>,
    since_span: Span,
    until_span: Option<Span>,
    /// The method with `since` and `until` already removed, so the
    /// snapshot never carries this crate's own attributes into the
    /// emitted Rust.
    func: TraitItemFn,
}

/// Read one version out of `#[since("…")]` or `#[until("…")]`.
///
/// Only the quoted three-part form is accepted. Prerelease and build
/// metadata are rejected rather than mangled into an identifier,
/// because there is no sanitisation of `1.0.0-rc.1` into a Rust
/// identifier that is both readable and injective — `-rc.1` and `-rc-1`
/// would collide. Saying so is better than a surprising collision.
fn parse_version(attr: &Attribute) -> syn::Result<(Version, Span)> {
    let lit: syn::LitStr = attr
        .parse_args()
        .map_err(|_| syn::Error::new_spanned(attr, VERSION_FORM))?;
    let span = lit.span();
    let raw = lit.value();

    let version = Version::parse(&raw).map_err(|e| {
        syn::Error::new(span, format!("`{raw}` is not a semantic version: {e}"))
    })?;

    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err(syn::Error::new(
            span,
            format!(
                "`{raw}` carries prerelease or build metadata, which has no \
                 unambiguous spelling as a Rust identifier; {VERSION_FORM}"
            ),
        ));
    }

    Ok((version, span))
}

/// Pull the version attributes off one method.
fn parse_decl(source_index: usize, func: &TraitItemFn) -> syn::Result<Decl> {
    let mut since: Option<(Version, Span)> = None;
    let mut until: Option<(Version, Span)> = None;

    for attr in func.attrs.iter().filter(|a| is_version_attr(a)) {
        let parsed = parse_version(attr)?;
        let slot = if attr.path().is_ident(SINCE) {
            &mut since
        } else {
            &mut until
        };
        if slot.is_some() {
            let which = if attr.path().is_ident(SINCE) {
                SINCE
            } else {
                UNTIL
            };
            return Err(syn::Error::new_spanned(
                attr,
                format!("duplicate `#[{which}]` on one declaration"),
            ));
        }
        *slot = Some(parsed);
    }

    let (since, since_span) = since.ok_or_else(|| {
        syn::Error::new(
            func.sig.ident.span(),
            format!(
                "`{}` has no `#[since(\"…\")]`; every method in a versioned \
                 service must say which version introduced it",
                func.sig.ident
            ),
        )
    })?;

    if let Some((until, until_span)) = &until {
        if *until <= since {
            return Err(syn::Error::new(
                *until_span,
                format!(
                    "`until` must be greater than `since`, but {until} <= \
                     {since}; `until` is exclusive, so this declaration would \
                     never be active"
                ),
            ));
        }
    }

    // The emitted method keeps its documentation and every unrelated
    // attribute, and loses only ours.
    let mut func = func.clone();
    func.attrs.retain(|a| !is_version_attr(a));

    Ok(Decl {
        source_index,
        since,
        until: until.as_ref().map(|(v, _)| v.clone()),
        since_span,
        until_span: until.map(|(_, s)| s),
        func,
    })
}

/// Everything the snapshots are built from, already validated.
struct Plan {
    /// Declarations grouped by method name, each group sorted by
    /// `since`.
    by_name: HashMap<String, Vec<Decl>>,
    /// Method names in wire-slot order.
    slots: Vec<String>,
    /// Every distinct `since`, ascending. One snapshot each.
    versions: Vec<Version>,
}

fn plan(item: &ItemTrait) -> syn::Result<Plan> {
    let mut decls = Vec::new();
    for (source_index, ti) in item.items.iter().enumerate() {
        match ti {
            TraitItem::Fn(f) => decls.push(parse_decl(source_index, f)?),
            other => {
                // Dispatch indexes methods by their position among the
                // trait's items, so an associated type or const sitting
                // between two methods would shift the wire ids of every
                // method after it. Refusing is better than generating a
                // protocol that is quietly wrong.
                return Err(syn::Error::new(
                    other.span(),
                    "a versioned service trait may contain only methods",
                ));
            }
        }
    }

    let mut by_name: HashMap<String, Vec<Decl>> = HashMap::new();
    for decl in decls {
        by_name
            .entry(decl.func.sig.ident.to_string())
            .or_default()
            .push(decl);
    }

    for (name, group) in by_name.iter_mut() {
        group.sort_by(|a, b| {
            a.since
                .cmp(&b.since)
                .then(a.source_index.cmp(&b.source_index))
        });

        for pair in group.windows(2) {
            let (earlier, later) = (&pair[0], &pair[1]);

            if earlier.since == later.since {
                return Err(syn::Error::new(
                    later.since_span,
                    format!(
                        "`{name}` is declared twice at version {}; each \
                         declaration of a method must have a distinct `since`",
                        later.since
                    ),
                ));
            }

            // An implicit end — no `until` — is the ordinary
            // replacement this whole feature is for, and is not an
            // error. An *explicit* `until` reaching past the next
            // declaration is, because both would be active at once and
            // nothing in the version ordering says which wins.
            if let Some(until) = &earlier.until {
                if *until > later.since {
                    return Err(syn::Error::new(
                        earlier.until_span.unwrap_or(earlier.since_span),
                        format!(
                            "`{name}` is active until {until}, which overlaps \
                             the declaration introduced at {}; two \
                             declarations of one method may not be active at \
                             the same version",
                            later.since
                        ),
                    ));
                }
            }
        }
    }

    // Introduction order, not selection order — see the module comment.
    //
    // The version dominates, so a method introduced later can never take
    // a slot from one introduced earlier, however the declarations are
    // arranged in the source. Within a single version the tie-break is
    // where the *name* first appears, not where its earliest declaration
    // sits, so moving one declaration of a method past another method
    // does not reshuffle the wire layout.
    let mut slots: Vec<(Version, usize, String)> = by_name
        .iter()
        .map(|(name, group)| {
            let introduced = group[0].since.clone();
            let first_mention = group
                .iter()
                .map(|d| d.source_index)
                .min()
                .expect("a group is never empty");
            (introduced, first_mention, name.clone())
        })
        .collect();
    slots.sort();
    let slots = slots.into_iter().map(|(_, _, name)| name).collect();

    // A BTreeSet would do, but the map keeps the ordering explicit.
    let versions: Vec<Version> = by_name
        .values()
        .flatten()
        .map(|d| (d.since.clone(), ()))
        .collect::<BTreeMap<_, _>>()
        .into_keys()
        .collect();

    Ok(Plan {
        by_name,
        slots,
        versions,
    })
}

/// `Echo` + `1.2.3` becomes `EchoV1_2_3`.
fn snapshot_ident(base: &Ident, version: &Version) -> Ident {
    format_ident!(
        "{}V{}_{}_{}",
        base,
        version.major,
        version.minor,
        version.patch,
        span = base.span()
    )
}

/// Build the trait for one version: the original trait with a versioned
/// name and only the methods that were live at that version.
fn snapshot(
    item: &ItemTrait,
    plan: &Plan,
    version: &Version,
) -> syn::Result<ItemTrait> {
    let mut items = Vec::new();

    for name in &plan.slots {
        let group = &plan.by_name[name];
        // Sorted by `since`, so the last active one is the greatest.
        let active = group.iter().rfind(|d| {
            d.since <= *version
                && d.until.as_ref().is_none_or(|u| *version < *u)
        });
        if let Some(decl) = active {
            items.push(TraitItem::Fn(decl.func.clone()));
        }
    }

    if items.is_empty() {
        return Err(syn::Error::new(
            item.ident.span(),
            format!(
                "version {version} of `{}` would have no methods; a service \
                 needs at least one",
                item.ident
            ),
        ));
    }

    // Cloning keeps the visibility, generics, supertraits, docs and any
    // other attributes the author put on the trait.
    let mut snap = item.clone();
    snap.ident = snapshot_ident(&item.ident, version);
    snap.items = items;
    Ok(snap)
}

/// Every snapshot this trait describes, oldest version first.
pub(crate) fn snapshots(item: &ItemTrait) -> syn::Result<Vec<ItemTrait>> {
    let plan = plan(item)?;

    let mut out = Vec::with_capacity(plan.versions.len());
    let mut seen: HashSet<String> = HashSet::new();

    for version in &plan.versions {
        let snap = snapshot(item, &plan, version)?;
        // Distinct three-part versions cannot produce the same
        // identifier today. The check is here so that if the naming
        // scheme ever grows a lossy case, it fails loudly at the macro
        // rather than as a confusing duplicate-definition error.
        if !seen.insert(snap.ident.to_string()) {
            return Err(syn::Error::new(
                item.ident.span(),
                format!(
                    "two versions of `{}` both generate `{}`",
                    item.ident, snap.ident
                ),
            ));
        }
        out.push(snap);
    }

    Ok(out)
}
