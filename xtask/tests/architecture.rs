//! Enforces the dependency rule against the real workspace, and proves the
//! guard actually rejects violations rather than passing vacuously.
//!
//! A guard that never fails is indistinguishable from no guard at all, so the
//! synthetic cases below matter as much as the check against the real tree.
//!
//! Integration tests compile as their own crate without `cfg(test)` set, so the
//! test-only relaxations the library crates apply via `cfg_attr` have to be
//! stated outright here.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use xtask::{LayerRule, Violation, check_all, check_crate, parse_rules};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

fn real_rules() -> BTreeMap<String, LayerRule> {
    let path = workspace_root().join("architecture.toml");
    let source =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    parse_rules(&source).expect("architecture.toml is well formed")
}

/// Reads every crate manifest under `crates/`, keyed by directory name.
///
/// Directory names and package names are kept identical by convention; the
/// test below checks that, so keying by directory is safe here.
fn real_manifests() -> BTreeMap<String, String> {
    let crates_dir = workspace_root().join("crates");
    let mut manifests = BTreeMap::new();
    for entry in fs::read_dir(&crates_dir).expect("crates/ exists") {
        let entry = entry.expect("readable directory entry");
        if !entry.file_type().expect("known file type").is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let manifest = entry.path().join("Cargo.toml");
        let source = fs::read_to_string(&manifest)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", manifest.display()));
        manifests.insert(name, source);
    }
    assert!(!manifests.is_empty(), "found no crates to check");
    manifests
}

#[test]
fn every_crate_in_the_workspace_obeys_the_dependency_rule() {
    let violations = check_all(&real_manifests(), &real_rules());

    assert!(
        violations.is_empty(),
        "the dependency rule is violated:\n{}",
        violations
            .iter()
            .map(|v| format!("  - {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn each_crate_directory_matches_its_package_name() {
    for (directory, manifest) in real_manifests() {
        assert!(
            manifest.contains(&format!("name = \"{directory}\"")),
            "crates/{directory} declares a different package name; the guard keys crates by \
             directory, so the two must agree"
        );
    }
}

#[test]
fn no_crate_depends_on_the_tooling_crate() {
    for (name, manifest) in real_manifests() {
        let dependencies = xtask::declared_dependencies(&manifest).expect("valid manifest");
        assert!(
            !dependencies.iter().any(|d| d == "xtask"),
            "{name} depends on xtask; tooling is not a layer and nothing may depend on it"
        );
    }
}

#[test]
fn a_dependency_pointing_outward_is_rejected() {
    let rule = LayerRule {
        workspace: vec!["adocpdf-core".to_owned()],
        external: Vec::new(),
    };

    let violations = check_crate(
        "adocpdf-domain",
        &["adocpdf-core".to_owned(), "adocpdf-typst".to_owned()],
        &rule,
    );

    assert_eq!(
        violations,
        vec![Violation::OutwardDependency {
            crate_name: "adocpdf-domain".to_owned(),
            dependency: "adocpdf-typst".to_owned(),
        }],
        "depending on an outer layer must be reported, and the inward edge must not be"
    );
}

#[test]
fn a_third_party_crate_absent_from_the_layer_is_rejected() {
    let rule = LayerRule {
        workspace: Vec::new(),
        external: vec!["thiserror".to_owned()],
    };

    let violations = check_crate("adocpdf-domain", &["typst".to_owned()], &rule);

    assert_eq!(
        violations,
        vec![Violation::ForbiddenExternal {
            crate_name: "adocpdf-domain".to_owned(),
            dependency: "typst".to_owned(),
        }],
        "the layout engine must not be reachable from the domain layer"
    );
}

#[test]
fn a_crate_with_no_rule_is_reported_rather_than_skipped() {
    let manifests = BTreeMap::from([(
        "adocpdf-newcomer".to_owned(),
        "[package]\nname = \"adocpdf-newcomer\"\n".to_owned(),
    )]);

    let violations = check_all(&manifests, &BTreeMap::new());

    assert_eq!(
        violations,
        vec![Violation::UngovernedCrate {
            crate_name: "adocpdf-newcomer".to_owned(),
        }],
        "a new crate must not escape the rule by going unmentioned"
    );
}

#[test]
fn an_allowed_dependency_set_produces_no_violation() {
    let rule = LayerRule {
        workspace: vec!["adocpdf-core".to_owned()],
        external: vec!["thiserror".to_owned()],
    };

    let violations = check_crate(
        "adocpdf-domain",
        &["adocpdf-core".to_owned(), "thiserror".to_owned()],
        &rule,
    );

    assert!(violations.is_empty(), "permitted dependencies must pass");
}

#[test]
fn rules_missing_a_required_key_are_rejected() {
    let error = parse_rules("[crates.adocpdf-core]\nworkspace = []\n")
        .expect_err("a rule without `external` is incomplete");

    assert!(
        error.contains("external"),
        "the error must name the missing key, got: {error}"
    );
}
