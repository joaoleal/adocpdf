//! The sandbox against a real filesystem.
//!
//! The domain's own tests prove the containment rule with a fake resolver. These
//! prove the rule still holds when the resolver is the operating system — in
//! particular that a real symbolic link is followed before the check, which is
//! the case a lexical check would get wrong.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::path::Path;

use adocpdf_domain::error::DomainError;
use adocpdf_domain::sandbox::{PathResolver, ProjectRoot, SandboxedPath};
use adocpdf_infra::path_resolver::FilesystemPathResolver;
use support::TempDir;

fn root_at(directory: &Path) -> ProjectRoot {
    ProjectRoot::new(directory, &FilesystemPathResolver::new()).expect("a real directory is a root")
}

#[test]
fn a_file_inside_the_root_is_accepted() {
    let temp = TempDir::new("inside");
    let source = temp.write("book.adoc", "= Title\n");
    let root = root_at(temp.path());

    let path = SandboxedPath::resolve(&source, &root, &FilesystemPathResolver::new())
        .expect("a file in the root is inside the root");

    assert!(path.as_path().ends_with("book.adoc"));
}

#[test]
fn a_traversal_leaving_the_root_is_refused() {
    let temp = TempDir::new("traversal");
    let root_dir = temp.create_dir("project");
    temp.write("secret.adoc", "outside\n");
    let root = root_at(&root_dir);

    let escape = root_dir.join("../secret.adoc");

    let error = SandboxedPath::resolve(&escape, &root, &FilesystemPathResolver::new())
        .expect_err("a path climbing out of the root must be refused");

    assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
}

#[test]
fn an_absolute_path_outside_the_root_is_refused() {
    let temp = TempDir::new("absolute");
    let root_dir = temp.create_dir("project");
    let outside = temp.write("elsewhere.adoc", "outside\n");
    let root = root_at(&root_dir);

    let error = SandboxedPath::resolve(&outside, &root, &FilesystemPathResolver::new())
        .expect_err("an absolute path outside the root must be refused");

    assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
}

#[cfg(unix)]
#[test]
fn a_symlink_pointing_out_of_the_root_is_refused() {
    let temp = TempDir::new("symlink");
    let root_dir = temp.create_dir("project");
    let target = temp.write("secret.adoc", "confidential\n");
    let link = root_dir.join("innocent.adoc");
    std::os::unix::fs::symlink(&target, &link).expect("can create a symlink");
    let root = root_at(&root_dir);

    let error = SandboxedPath::resolve(&link, &root, &FilesystemPathResolver::new())
        .expect_err("a link spelled inside the root but pointing outside must be refused");

    let message = error.to_string();
    assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
    assert!(
        !message.contains("secret"),
        "the refusal must not reveal the link's target, got: {message}"
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_staying_inside_the_root_is_accepted() {
    let temp = TempDir::new("symlink-inside");
    let root_dir = temp.create_dir("project");
    let target = temp.write("project/real.adoc", "= Real\n");
    let link = root_dir.join("alias.adoc");
    std::os::unix::fs::symlink(&target, &link).expect("can create a symlink");
    let root = root_at(&root_dir);

    let path = SandboxedPath::resolve(&link, &root, &FilesystemPathResolver::new())
        .expect("a link to a file inside the root is inside the root");

    assert!(
        path.as_path().ends_with("real.adoc"),
        "the link must resolve to its target"
    );
}

#[test]
fn an_output_file_that_does_not_exist_yet_can_be_checked() {
    let temp = TempDir::new("output");
    let root = root_at(temp.path());
    let output = temp.path().join("out.pdf");

    let path = SandboxedPath::resolve(&output, &root, &FilesystemPathResolver::new())
        .expect("confinement must be checkable before the file is written");

    assert!(path.as_path().ends_with("out.pdf"));
    assert!(
        !path.as_path().exists(),
        "checking must not create the file"
    );
}

#[test]
fn an_output_path_that_does_not_exist_and_escapes_is_refused() {
    let temp = TempDir::new("output-escape");
    let root_dir = temp.create_dir("project");
    let root = root_at(&root_dir);

    let escape = root_dir.join("../out.pdf");

    let error = SandboxedPath::resolve(&escape, &root, &FilesystemPathResolver::new())
        .expect_err("a not-yet-created file outside the root must still be refused");

    assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
}

#[test]
fn a_root_that_does_not_exist_is_refused() {
    let temp = TempDir::new("missing-root");
    let missing = temp.path().join("nowhere");

    let error = ProjectRoot::new(&missing, &FilesystemPathResolver::new())
        .expect_err("a root that does not exist is not a root");

    assert!(matches!(error, DomainError::RootNotADirectory { .. }));
    assert!(
        error.to_string().contains("nowhere"),
        "the error must name the supplied root, got: {error}"
    );
}

#[test]
fn a_file_is_not_a_root() {
    let temp = TempDir::new("file-root");
    let file = temp.write("book.adoc", "= Title\n");

    let error = ProjectRoot::new(&file, &FilesystemPathResolver::new())
        .expect_err("a file is not a directory");

    assert!(matches!(error, DomainError::RootNotADirectory { .. }));
}

#[test]
fn a_sibling_directory_with_a_similar_name_is_outside_the_root() {
    let temp = TempDir::new("sibling");
    let root_dir = temp.create_dir("project");
    let sibling = temp.write("project-old/book.adoc", "= Old\n");
    let root = root_at(&root_dir);

    let error = SandboxedPath::resolve(&sibling, &root, &FilesystemPathResolver::new())
        .expect_err("a name sharing a prefix with the root is not inside it");

    assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
}

#[test]
fn the_resolver_reports_directories_as_directories() {
    let temp = TempDir::new("is-dir");
    let file = temp.write("book.adoc", "= Title\n");
    let resolver = FilesystemPathResolver::new();

    assert!(resolver.is_directory(temp.path()));
    assert!(!resolver.is_directory(&file));
}
