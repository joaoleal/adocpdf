//! The filesystem store against real files.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use adocpdf_domain::error::DomainError;
use adocpdf_domain::ports::SourceStore;
use adocpdf_domain::sandbox::{ProjectRoot, SandboxedPath};
use adocpdf_infra::path_resolver::FilesystemPathResolver;
use adocpdf_infra::source_store::FilesystemSourceStore;
use support::TempDir;

fn sandboxed(temp: &TempDir, relative: &str) -> SandboxedPath {
    let resolver = FilesystemPathResolver::new();
    let root = ProjectRoot::new(temp.path(), &resolver).expect("the temp dir is a root");
    SandboxedPath::resolve(&temp.path().join(relative), &root, &resolver)
        .expect("the path is inside the root")
}

#[test]
fn a_file_inside_the_root_is_read_back() {
    let temp = TempDir::new("store-read");
    temp.write("book.adoc", "= Title\n\nBody.\n");

    let contents = FilesystemSourceStore::new()
        .read(&sandboxed(&temp, "book.adoc"))
        .expect("an existing file reads");

    assert_eq!(contents, "= Title\n\nBody.\n");
}

#[test]
fn a_missing_file_is_reported_as_not_found() {
    let temp = TempDir::new("store-missing");

    let error = FilesystemSourceStore::new()
        .read(&sandboxed(&temp, "absent.adoc"))
        .expect_err("a file that is not there cannot be read");

    assert!(matches!(error, DomainError::InputNotFound { .. }));
    assert!(
        error.to_string().contains("absent.adoc"),
        "the error must name the missing input, got: {error}"
    );
}

#[test]
fn a_directory_read_as_a_document_is_unreadable_rather_than_missing() {
    let temp = TempDir::new("store-dir");
    temp.create_dir("chapters");

    let error = FilesystemSourceStore::new()
        .read(&sandboxed(&temp, "chapters"))
        .expect_err("a directory is not a document");

    assert!(
        matches!(error, DomainError::InputUnreadable { .. }),
        "something that exists but cannot be read is not the same as something absent"
    );
}

#[test]
fn bytes_are_written_verbatim() {
    let temp = TempDir::new("store-write");
    let bytes = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n";

    FilesystemSourceStore::new()
        .write(&sandboxed(&temp, "out.pdf"), bytes)
        .expect("a file in the root can be written");

    assert_eq!(std::fs::read(temp.path().join("out.pdf")).unwrap(), bytes);
}

#[test]
fn writing_replaces_an_existing_file() {
    let temp = TempDir::new("store-replace");
    temp.write("out.pdf", "old");

    FilesystemSourceStore::new()
        .write(&sandboxed(&temp, "out.pdf"), b"new")
        .expect("an existing file can be replaced");

    assert_eq!(
        std::fs::read_to_string(temp.path().join("out.pdf")).unwrap(),
        "new"
    );
}

#[test]
fn a_write_into_a_missing_directory_is_reported_not_created() {
    let temp = TempDir::new("store-nodir");

    let error = FilesystemSourceStore::new()
        .write(&sandboxed(&temp, "nested/out.pdf"), b"bytes")
        .expect_err("the parent directory does not exist");

    assert!(matches!(error, DomainError::OutputUnwritable { .. }));
    assert!(
        !temp.path().join("nested").exists(),
        "a render must not create directories the caller never named"
    );
}
