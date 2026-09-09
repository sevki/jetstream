//! The `r[...]` annotations have to resolve.
//!
//! `r[impl jetstream.foo.bar]` in the source claims that this code
//! implements a rule written down in `docs/specs/`, and `r[verify ...]`
//! claims a test checks one. The claim is the whole value: it is what
//! lets you read a rule and find its implementation, or read code and
//! find the requirement it answers.
//!
//! A dangling annotation is worse than no annotation. It asserts a
//! requirement exists, and anyone who goes looking finds nothing — so the
//! reader cannot tell whether the rule was renamed, was deleted, or was
//! never written. Three had accumulated by the time anyone checked, two of
//! them on a type that had no implementation at all.
//!
//! Nothing kept them honest, because nothing could: renaming a rule in
//! `docs/specs/` does not break a build. This does.

use std::{collections::BTreeSet, fs, path::Path};

/// Every rule id defined in the specification, from lines of the form
/// `r[some.rule.id]` at the start of a line.
fn defined_rules(root: &Path) -> BTreeSet<String> {
    let mut rules = BTreeSet::new();
    let specs = root.join("docs/specs");
    let Ok(entries) = fs::read_dir(&specs) else {
        panic!("docs/specs is missing: {}", specs.display());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if let Some(id) =
                line.strip_prefix("r[").and_then(|r| r.strip_suffix(']'))
            {
                // Definitions are bare; `r[impl ...]` and `r[verify ...]`
                // are references and are collected separately.
                if !id.contains(' ') {
                    rules.insert(id.to_string());
                }
            }
        }
    }
    rules
}

/// Every `r[impl ...]` / `r[verify ...]` reference, with where it is.
fn references(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut stack = vec![
        root.join("components"),
        root.join("src"),
        root.join("tests"),
    ];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Build output is not source, and is enormous.
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            // Not itself: this file spells out the patterns it searches
            // for, so scanning it finds its own literals and every
            // example in its documentation.
            if path.file_name().is_some_and(|n| n == "spec_annotations.rs") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let where_ = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            for kind in ["r[impl ", "r[verify "] {
                let mut rest = text.as_str();
                while let Some(at) = rest.find(kind) {
                    rest = &rest[at + kind.len()..];
                    if let Some(end) = rest.find(']') {
                        found.push((
                            rest[..end].trim().to_string(),
                            where_.clone(),
                        ));
                    }
                }
            }
        }
    }
    found
}

#[test]
fn every_annotation_names_a_rule_that_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let defined = defined_rules(root);
    assert!(
        defined.len() > 100,
        "only {} rules parsed out of docs/specs — the parser has drifted \
         from the format, which would make this test pass by finding \
         nothing to check",
        defined.len(),
    );

    let mut dangling: Vec<String> = references(root)
        .into_iter()
        .filter(|(id, _)| !defined.contains(id))
        .map(|(id, at)| format!("  {at}: r[... {id}]"))
        .collect();
    dangling.sort();
    dangling.dedup();

    assert!(
        dangling.is_empty(),
        "{} annotation(s) name a rule that is not in docs/specs/.\n{}\n\n\
         Either the rule was renamed and the annotation was not, or the \
         annotation claims an implementation of something never written. \
         Fix whichever it is — do not delete the annotation to get green \
         unless the code really implements nothing.",
        dangling.len(),
        dangling.join("\n"),
    );
}
