//! Confining file access to a project root.
//!
//! The rule is that a path is judged by where it *resolves to*, never by how it
//! is spelled. Traversal segments, absolute paths and symbolic links pointing
//! outward are then all the same case, and none of them needs its own check.
//!
//! Resolving a path means touching the filesystem, which this layer must not
//! do. So resolution is a port ([`PathResolver`]) and the containment rule
//! itself is pure: given a resolved path and a resolved root, is one inside the
//! other? That keeps the security-relevant decision here, in a layer that can
//! be tested exhaustively without a filesystem, while the I/O lives in an
//! adapter.

use std::path::{Path, PathBuf};

use crate::error::DomainError;

/// Why a path could not be resolved to a real location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionError {
    /// Nothing along the path exists, so there is no location to resolve to.
    NotFound,
    /// An existing part of the path could not be read.
    Unreadable(String),
}

/// Resolves a path to the location it really refers to.
///
/// Implementations must follow symbolic links and eliminate `.` and `..`, so
/// that the result names one location and one only.
///
/// The path need not exist. A path whose final components have not been created
/// yet — an output file, typically — must resolve against its nearest existing
/// ancestor, with the remaining components appended. Without that, confinement
/// could not be checked before a file is written, which is precisely when it
/// matters.
pub trait PathResolver {
    /// Resolves `path` to an absolute location with symlinks followed.
    ///
    /// # Errors
    ///
    /// Returns [`ResolutionError`] when no part of the path exists, or when an
    /// existing part cannot be read.
    fn resolve(&self, path: &Path) -> Result<PathBuf, ResolutionError>;

    /// Whether `path` names a directory.
    fn is_directory(&self, path: &Path) -> bool;
}

/// The directory every readable and writable path must sit inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoot {
    resolved: PathBuf,
    requested: String,
}

impl ProjectRoot {
    /// Establishes the project root.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::RootNotADirectory`] when the path does not
    /// resolve, or resolves to something that is not a directory.
    pub fn new(requested: &Path, resolver: &dyn PathResolver) -> Result<Self, DomainError> {
        let display = requested.display().to_string();

        let location = resolver
            .resolve(requested)
            .map_err(|_| DomainError::RootNotADirectory {
                root: display.clone(),
            })?;

        if !resolver.is_directory(&location) {
            return Err(DomainError::RootNotADirectory { root: display });
        }

        Ok(Self {
            resolved: location,
            requested: display,
        })
    }

    /// The root's resolved location.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.resolved
    }

    /// The root as it was supplied, for use in diagnostics.
    #[must_use]
    pub fn as_requested(&self) -> &str {
        &self.requested
    }

    /// Whether a resolved location sits inside this root.
    ///
    /// Comparison is component-wise, so a sibling directory whose name merely
    /// begins with the root's name — `/project-old` against `/project` — is
    /// correctly outside.
    #[must_use]
    pub fn contains(&self, resolved: &Path) -> bool {
        resolved.starts_with(&self.resolved)
    }
}

/// A path that has been proven to sit inside the project root.
///
/// Holding one is the evidence that confinement was checked. Adapters accept
/// this type rather than a bare path, so an unchecked path cannot reach an I/O
/// call by being forgotten about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxedPath {
    resolved: PathBuf,
    requested: String,
}

impl SandboxedPath {
    /// Resolves a path and checks that it lies inside the root.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::PathOutsideRoot`] when the path resolves outside
    /// the root, or cannot be resolved at all. The two are reported alike on
    /// purpose: distinguishing them would tell a caller whether a path outside
    /// the sandbox exists.
    pub fn resolve(
        requested: &Path,
        root: &ProjectRoot,
        resolver: &dyn PathResolver,
    ) -> Result<Self, DomainError> {
        let display = requested.display().to_string();

        let outside = || DomainError::PathOutsideRoot {
            requested: display.clone(),
            root: root.as_requested().to_owned(),
        };

        let location = resolver.resolve(requested).map_err(|_| outside())?;

        if !root.contains(&location) {
            return Err(outside());
        }

        Ok(Self {
            resolved: location,
            requested: display,
        })
    }

    /// The resolved location.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.resolved
    }

    /// The path as it was supplied, for use in diagnostics.
    #[must_use]
    pub fn as_requested(&self) -> &str {
        &self.requested
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// A resolver backed by a map instead of a filesystem.
    ///
    /// Every case the real resolver faces — traversal, absolute paths,
    /// symlinks — reduces to "this spelling resolves to that location", which
    /// is exactly what the map expresses. The adapter's own tests cover that
    /// the real filesystem agrees.
    struct FakeResolver {
        locations: BTreeMap<String, PathBuf>,
        directories: Vec<PathBuf>,
    }

    impl FakeResolver {
        fn new() -> Self {
            Self {
                locations: BTreeMap::new(),
                directories: Vec::new(),
            }
        }

        fn resolving(mut self, spelled: &str, to: &str) -> Self {
            self.locations.insert(spelled.to_owned(), PathBuf::from(to));
            self
        }

        fn with_directory(mut self, path: &str) -> Self {
            self.directories.push(PathBuf::from(path));
            self
        }
    }

    impl PathResolver for FakeResolver {
        fn resolve(&self, path: &Path) -> Result<PathBuf, ResolutionError> {
            self.locations
                .get(&path.display().to_string())
                .cloned()
                .ok_or(ResolutionError::NotFound)
        }

        fn is_directory(&self, path: &Path) -> bool {
            self.directories.iter().any(|d| d == path)
        }
    }

    fn root_of(resolver: &FakeResolver, spelled: &str) -> ProjectRoot {
        ProjectRoot::new(Path::new(spelled), resolver).unwrap()
    }

    fn project_resolver() -> FakeResolver {
        FakeResolver::new()
            .resolving("project", "/home/user/project")
            .with_directory("/home/user/project")
    }

    #[test]
    fn a_path_inside_the_root_is_accepted() {
        let resolver =
            project_resolver().resolving("project/book.adoc", "/home/user/project/book.adoc");
        let root = root_of(&resolver, "project");

        let path =
            SandboxedPath::resolve(Path::new("project/book.adoc"), &root, &resolver).unwrap();

        assert_eq!(path.as_path(), Path::new("/home/user/project/book.adoc"));
    }

    #[test]
    fn a_traversal_path_that_escapes_is_refused() {
        let resolver = project_resolver().resolving("project/../../etc/passwd", "/etc/passwd");
        let root = root_of(&resolver, "project");

        let error = SandboxedPath::resolve(Path::new("project/../../etc/passwd"), &root, &resolver)
            .unwrap_err();

        assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
    }

    #[test]
    fn a_traversal_path_that_stays_inside_is_accepted() {
        let resolver = project_resolver().resolving(
            "project/chapters/../book.adoc",
            "/home/user/project/book.adoc",
        );
        let root = root_of(&resolver, "project");

        assert!(
            SandboxedPath::resolve(Path::new("project/chapters/../book.adoc"), &root, &resolver)
                .is_ok(),
            "traversal is only a problem when it leaves the root"
        );
    }

    #[test]
    fn a_symlink_pointing_outward_is_refused() {
        let resolver = project_resolver().resolving("project/link.adoc", "/etc/shadow");
        let root = root_of(&resolver, "project");

        let error =
            SandboxedPath::resolve(Path::new("project/link.adoc"), &root, &resolver).unwrap_err();

        assert!(
            matches!(error, DomainError::PathOutsideRoot { .. }),
            "a path spelled inside the root but resolving outside it must be refused"
        );
    }

    #[test]
    fn a_refusal_does_not_reveal_where_the_path_resolved_to() {
        let resolver = project_resolver().resolving("project/link.adoc", "/etc/shadow");
        let root = root_of(&resolver, "project");

        let error =
            SandboxedPath::resolve(Path::new("project/link.adoc"), &root, &resolver).unwrap_err();

        let message = error.to_string();
        assert!(
            !message.contains("shadow"),
            "the resolved target must not appear in the message, got: {message}"
        );
        assert!(message.contains("project/link.adoc"), "got: {message}");
    }

    #[test]
    fn an_absolute_path_outside_the_root_is_refused() {
        let resolver = project_resolver().resolving("/etc/passwd", "/etc/passwd");
        let root = root_of(&resolver, "project");

        let error = SandboxedPath::resolve(Path::new("/etc/passwd"), &root, &resolver).unwrap_err();

        assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
    }

    #[test]
    fn an_unresolvable_path_is_refused_the_same_way_as_one_outside() {
        let resolver = project_resolver();
        let root = root_of(&resolver, "project");

        let error = SandboxedPath::resolve(Path::new("project/missing.adoc"), &root, &resolver)
            .unwrap_err();

        assert!(
            matches!(error, DomainError::PathOutsideRoot { .. }),
            "reporting these differently would disclose whether a path exists outside the root"
        );
    }

    #[test]
    fn a_sibling_directory_with_a_similar_name_is_outside_the_root() {
        let resolver = project_resolver()
            .resolving("project-old/book.adoc", "/home/user/project-old/book.adoc");
        let root = root_of(&resolver, "project");

        let error = SandboxedPath::resolve(Path::new("project-old/book.adoc"), &root, &resolver)
            .unwrap_err();

        assert!(
            matches!(error, DomainError::PathOutsideRoot { .. }),
            "containment must compare path components, not string prefixes"
        );
    }

    #[test]
    fn a_root_that_does_not_exist_is_refused() {
        let resolver = FakeResolver::new();

        let error = ProjectRoot::new(Path::new("nowhere"), &resolver).unwrap_err();

        assert_eq!(
            error,
            DomainError::RootNotADirectory {
                root: "nowhere".to_owned()
            }
        );
    }

    #[test]
    fn a_root_that_is_not_a_directory_is_refused() {
        let resolver = FakeResolver::new().resolving("book.adoc", "/home/user/book.adoc");

        let error = ProjectRoot::new(Path::new("book.adoc"), &resolver).unwrap_err();

        assert!(matches!(error, DomainError::RootNotADirectory { .. }));
    }

    #[test]
    fn a_refusal_names_the_root_in_effect() {
        let resolver = project_resolver().resolving("/etc/passwd", "/etc/passwd");
        let root = root_of(&resolver, "project");

        let error = SandboxedPath::resolve(Path::new("/etc/passwd"), &root, &resolver).unwrap_err();

        assert!(
            error.to_string().contains("project"),
            "the boundary must never be ambiguous, got: {error}"
        );
    }

    #[test]
    fn a_root_remembers_the_spelling_it_was_given() {
        let resolver = project_resolver();
        let root = root_of(&resolver, "project");

        // Not the resolved location. A diagnostic has to name the thing the
        // caller typed, or they cannot connect the message to what they wrote.
        assert_eq!(root.as_requested(), "project");
        assert_eq!(root.path(), Path::new("/home/user/project"));
    }

    #[test]
    fn an_accepted_path_remembers_the_spelling_it_was_given() {
        let resolver = project_resolver().resolving(
            "project/./docs/../book.adoc",
            "/home/user/project/book.adoc",
        );
        let root = root_of(&resolver, "project");

        let path =
            SandboxedPath::resolve(Path::new("project/./docs/../book.adoc"), &root, &resolver)
                .unwrap();

        // The two differ here on purpose: the spelling went through `.` and
        // `..`, and the resolved location did not. Reporting the resolved one
        // would both confuse the reader and disclose more than they asked for.
        assert_eq!(path.as_requested(), "project/./docs/../book.adoc");
        assert_eq!(path.as_path(), Path::new("/home/user/project/book.adoc"));
        assert_ne!(path.as_requested(), path.as_path().display().to_string());
    }

    /// The containment rule, stated for every path expression rather than for
    /// the handful of spellings someone thought to write down.
    ///
    /// The rule these check is the one in the module docs: a path is judged by
    /// where it *resolves*, never by how it is spelled. Traversal, absolute
    /// prefixes, `.`, empty components and symlinks are not special cases —
    /// they are all just spellings that resolve somewhere.
    mod properties {
        use proptest::prelude::*;

        use super::super::{Path, SandboxedPath};
        use super::{project_resolver, root_of};

        const ROOT: &str = "/home/user/project";

        /// The pieces a path expression is built from, including the ones that
        /// mean something to a resolver: parent, current, and empty.
        fn segment() -> impl Strategy<Value = String> {
            prop_oneof![
                Just("a"),
                Just("b"),
                Just("docs"),
                Just("chapter"),
                Just(".."),
                Just("."),
                Just(""),
            ]
            .prop_map(str::to_owned)
        }

        /// A path expression. Half are written relative to the project, half
        /// are absolute — a distinction that must not matter on its own.
        fn spelling() -> impl Strategy<Value = String> {
            (any::<bool>(), proptest::collection::vec(segment(), 1..6)).prop_map(
                |(absolute, segments)| {
                    let joined = segments.join("/");
                    if absolute {
                        format!("/{joined}")
                    } else {
                        format!("project/{joined}")
                    }
                },
            )
        }

        /// Somewhere a path might resolve to: inside the root, or outside it.
        ///
        /// `/home/user/project-notes` is deliberately present. It shares a
        /// textual prefix with the root but is a different directory, and a
        /// containment check written with string comparison rather than path
        /// components would wrongly admit it.
        fn location() -> impl Strategy<Value = String> {
            prop_oneof![
                Just("/home/user/project/book.adoc"),
                Just("/home/user/project/docs/chapter.adoc"),
                Just("/home/user/project"),
                Just("/etc/passwd"),
                Just("/home/user/other/book.adoc"),
                Just("/home/user/project-notes/book.adoc"),
                Just("/"),
            ]
            .prop_map(str::to_owned)
        }

        proptest! {
            /// Two different spellings that resolve to the same place are
            /// accepted or refused alike.
            ///
            /// This is the property the whole sandbox rests on. If it ever
            /// fails, some spelling has become its own case — which is how a
            /// traversal gets through a checker that was looking for `..`.
            #[test]
            fn two_spellings_of_one_location_get_the_same_verdict(
                first in spelling(),
                second in spelling(),
                target in location(),
            ) {
                prop_assume!(first != second);
                prop_assume!(first != "project" && second != "project");

                let resolver = project_resolver()
                    .resolving(&first, &target)
                    .resolving(&second, &target);
                let root = root_of(&resolver, "project");

                let one = SandboxedPath::resolve(Path::new(&first), &root, &resolver);
                let other = SandboxedPath::resolve(Path::new(&second), &root, &resolver);

                prop_assert_eq!(
                    one.is_ok(),
                    other.is_ok(),
                    "{:?} and {:?} both resolve to {:?} but were judged differently",
                    first, second, target
                );

                if let (Ok(one), Ok(other)) = (&one, &other) {
                    prop_assert_eq!(
                        one.as_path(),
                        other.as_path(),
                        "same location, different accepted path"
                    );
                }
            }

            /// The verdict is exactly containment of the resolved location,
            /// and nothing else about the expression.
            #[test]
            fn the_verdict_is_containment_of_the_resolved_location(
                spelled in spelling(),
                target in location(),
            ) {
                prop_assume!(spelled != "project");

                let resolver = project_resolver().resolving(&spelled, &target);
                let root = root_of(&resolver, "project");

                let accepted =
                    SandboxedPath::resolve(Path::new(&spelled), &root, &resolver).is_ok();
                let contained = Path::new(&target).starts_with(Path::new(ROOT));

                prop_assert_eq!(
                    accepted,
                    contained,
                    "{:?} resolves to {:?}; containment says {} but the sandbox said {}",
                    spelled, target, contained, accepted
                );
            }

            /// A path that resolves nowhere is refused, and is refused the same
            /// way a path outside the root is — the sandbox must not double as
            /// an oracle for which files exist.
            #[test]
            fn an_unresolvable_path_is_refused_like_any_other(spelled in spelling()) {
                prop_assume!(spelled != "project");

                let resolver = project_resolver();
                let root = root_of(&resolver, "project");

                let error = SandboxedPath::resolve(Path::new(&spelled), &root, &resolver)
                    .expect_err("a path the resolver does not know cannot be accepted");

                prop_assert!(
                    matches!(error, crate::error::DomainError::PathOutsideRoot { .. }),
                    "expected the same refusal as an escaping path, got {error:?}"
                );
            }
        }
    }
}
