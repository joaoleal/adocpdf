//! The architecture guard.
//!
//! Dependencies in this workspace flow strictly inward. That rule is stated in
//! `architecture.toml` and enforced here, by reading crate manifests and
//! comparing what they declare against what the rule permits.
//!
//! The check lives in a test rather than a script so it runs wherever
//! `cargo test` runs, and so a violation is reported with the same ergonomics
//! as any other failing test. See `design.md` — decision D5.
//!
//! This is manifest-level checking. It catches a crate depending on a crate it
//! must not see; it does not catch a layering mistake made *within* a crate.
//! That is the right granularity here because the layers are crates.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeMap;
use std::fmt;

/// Crates in this workspace share this name prefix.
///
/// A dependency carrying it is an intra-workspace edge and is checked against
/// the `workspace` allow-list; anything else is third-party and is checked
/// against `external`.
pub const WORKSPACE_PREFIX: &str = "adocpdf-";

/// Which dependencies one crate is permitted to declare.
///
/// Both lists are exhaustive — a dependency absent from the relevant list is a
/// violation, so new dependencies require a deliberate edit to
/// `architecture.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerRule {
    /// Other workspace crates this crate may depend on.
    pub workspace: Vec<String>,
    /// Third-party crates this crate may depend on.
    pub external: Vec<String>,
}

/// A dependency edge the rules forbid, or a crate the rules do not cover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A crate depends on a workspace crate outside its allowed set.
    ///
    /// This is the outward edge the dependency rule exists to prevent.
    OutwardDependency {
        /// The crate whose manifest declares the dependency.
        crate_name: String,
        /// The workspace crate it must not depend on.
        dependency: String,
    },
    /// A crate depends on a third-party crate its layer may not use.
    ///
    /// This is what keeps the layout engine and the AsciiDoc parser confined
    /// to the infrastructure layer.
    ForbiddenExternal {
        /// The crate whose manifest declares the dependency.
        crate_name: String,
        /// The third-party crate it must not depend on.
        dependency: String,
    },
    /// A crate exists in the workspace but `architecture.toml` says nothing
    /// about it.
    ///
    /// Reported so a new crate cannot escape the rule simply by not being
    /// mentioned.
    UngovernedCrate {
        /// The crate found on disk with no rule covering it.
        crate_name: String,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutwardDependency {
                crate_name,
                dependency,
            } => write!(
                f,
                "{crate_name} depends on {dependency}, which is outside its allowed set. \
                 Dependencies must flow inward; see architecture.toml."
            ),
            Self::ForbiddenExternal {
                crate_name,
                dependency,
            } => write!(
                f,
                "{crate_name} depends on the third-party crate {dependency}, which its layer \
                 may not use. Add it to architecture.toml only if it belongs at this layer."
            ),
            Self::UngovernedCrate { crate_name } => write!(
                f,
                "{crate_name} has no entry in architecture.toml. Every crate must be governed \
                 by the dependency rule."
            ),
        }
    }
}

/// Parses `architecture.toml` into the rule for each crate.
///
/// # Errors
///
/// Returns a message describing the problem when the document is not valid
/// TOML, has no `crates` table, or an entry is not shaped as a rule.
pub fn parse_rules(source: &str) -> Result<BTreeMap<String, LayerRule>, String> {
    let document: toml::Table =
        toml::from_str(source).map_err(|e| format!("architecture.toml is not valid TOML: {e}"))?;

    let crates = document
        .get("crates")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "architecture.toml has no [crates] table".to_owned())?;

    crates
        .iter()
        .map(|(name, entry)| {
            let rule = LayerRule {
                workspace: string_list(entry, "workspace", name)?,
                external: string_list(entry, "external", name)?,
            };
            Ok((name.clone(), rule))
        })
        .collect()
}

fn string_list(entry: &toml::Value, key: &str, crate_name: &str) -> Result<Vec<String>, String> {
    let Some(value) = entry.get(key) else {
        return Err(format!("[crates.{crate_name}] is missing `{key}`"));
    };
    let items = value
        .as_array()
        .ok_or_else(|| format!("[crates.{crate_name}].{key} must be an array"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("[crates.{crate_name}].{key} must contain only strings"))
        })
        .collect()
}

/// Extracts every dependency a crate manifest declares.
///
/// Both `[dependencies]` and `[dev-dependencies]` are read: a test that reaches
/// outward breaks the layering just as a library that does. Build dependencies
/// are read for the same reason.
///
/// # Errors
///
/// Returns a message when the manifest is not valid TOML.
pub fn declared_dependencies(manifest: &str) -> Result<Vec<String>, String> {
    let document: toml::Table =
        toml::from_str(manifest).map_err(|e| format!("manifest is not valid TOML: {e}"))?;

    let mut found = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = document.get(section).and_then(toml::Value::as_table) {
            found.extend(table.keys().cloned());
        }
    }
    found.sort_unstable();
    found.dedup();
    Ok(found)
}

/// Checks one crate's declared dependencies against its rule.
///
/// Returns every violation rather than stopping at the first, so a single test
/// run reports the whole picture.
#[must_use]
pub fn check_crate(crate_name: &str, dependencies: &[String], rule: &LayerRule) -> Vec<Violation> {
    dependencies
        .iter()
        .filter_map(|dependency| {
            if dependency.starts_with(WORKSPACE_PREFIX) {
                (!rule.workspace.contains(dependency)).then(|| Violation::OutwardDependency {
                    crate_name: crate_name.to_owned(),
                    dependency: dependency.clone(),
                })
            } else {
                (!rule.external.contains(dependency)).then(|| Violation::ForbiddenExternal {
                    crate_name: crate_name.to_owned(),
                    dependency: dependency.clone(),
                })
            }
        })
        .collect()
}

/// Checks every crate found on disk against the rules.
///
/// `manifests` maps a crate name to the text of its manifest. A crate present
/// here but absent from `rules` is reported as ungoverned rather than skipped.
#[must_use]
pub fn check_all(
    manifests: &BTreeMap<String, String>,
    rules: &BTreeMap<String, LayerRule>,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (crate_name, manifest) in manifests {
        let Some(rule) = rules.get(crate_name) else {
            violations.push(Violation::UngovernedCrate {
                crate_name: crate_name.clone(),
            });
            continue;
        };
        match declared_dependencies(manifest) {
            Ok(dependencies) => violations.extend(check_crate(crate_name, &dependencies, rule)),
            Err(message) => violations.push(Violation::UngovernedCrate {
                crate_name: format!("{crate_name} ({message})"),
            }),
        }
    }
    violations
}
