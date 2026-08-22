//! The whole path, through the real binary.
//!
//! These tests invoke the compiled executable rather than calling into the
//! library, and they read back the PDF it wrote rather than inspecting a buffer
//! the code already held. That is the point: everything else in the workspace
//! can pass while the assembled program still fails to produce a usable file.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, process};

/// The date every test renders under, so output never depends on the day.
const FIXED_DATE: &str = "2026-08-16";

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A directory holding a copy of one fixture, deleted when the test ends.
struct Workspace {
    path: PathBuf,
}

impl Workspace {
    /// Copies `fixture` into a fresh directory that becomes the project root.
    fn with(fixture: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "adocpdf-e2e-{}-{}-{unique}",
            process::id(),
            fixture.replace('.', "-")
        ));

        drop(fs::remove_dir_all(&path));
        fs::create_dir_all(&path).expect("can create the workspace");

        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture);
        fs::copy(&source, path.join(fixture))
            .unwrap_or_else(|e| panic!("cannot copy {}: {e}", source.display()));

        Self { path }
    }

    fn input(&self, fixture: &str) -> PathBuf {
        self.path.join(fixture)
    }

    fn output(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.path));
    }
}

/// Runs the real `adocpdf` binary.
fn run(arguments: &[&Path], extra: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_adocpdf"));
    for argument in arguments {
        command.arg(argument);
    }
    command.args(extra);
    command.output().expect("the binary runs")
}

/// Renders a fixture and returns the PDF bytes, failing loudly if it did not.
fn render(fixture: &str) -> (Workspace, Vec<u8>) {
    let workspace = Workspace::with(fixture);
    let output_path = workspace.output("out.pdf");

    let output = run(
        &[&workspace.input(fixture), &output_path],
        &["--date", FIXED_DATE],
    );

    assert!(
        output.status.success(),
        "rendering {fixture} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let bytes = fs::read(&output_path).expect("the PDF was written");
    (workspace, bytes)
}

/// The text a reader would see, extracted from the finished PDF.
fn text_of(pdf: &[u8]) -> String {
    pdf_extract::extract_text_from_mem(pdf).expect("the PDF can be read back")
}

/// Every page's width and height in points, read from the PDF's page boxes.
fn page_sizes(pdf: &[u8]) -> Vec<(f64, f64)> {
    let text = String::from_utf8_lossy(pdf);
    let mut sizes = Vec::new();

    for start in text.match_indices("/MediaBox").map(|(index, _)| index) {
        let Some(open) = text[start..].find('[') else {
            continue;
        };
        let Some(close) = text[start + open..].find(']') else {
            continue;
        };
        let numbers: Vec<f64> = text[start + open + 1..start + open + close]
            .split_whitespace()
            .filter_map(|value| value.parse().ok())
            .collect();

        if let [x0, y0, x1, y1] = numbers[..] {
            sizes.push(((x1 - x0).abs(), (y1 - y0).abs()));
        }
    }

    sizes
}

#[test]
fn a_supported_document_renders_to_a_readable_pdf() {
    let (_workspace, pdf) = render("supported.adoc");

    assert!(pdf.starts_with(b"%PDF-"), "the output must be a PDF");

    let text = text_of(&pdf);
    assert!(
        text.contains("The Walking Skeleton"),
        "the document title must reach the page, got:\n{text}"
    );
    assert!(
        text.contains("An opening paragraph"),
        "body text must reach the page, got:\n{text}"
    );
    assert!(
        text.contains("Overview") && text.contains("Nested Detail"),
        "both heading levels must reach the page, got:\n{text}"
    );
}

#[test]
fn the_binary_reports_what_it_wrote() {
    let workspace = Workspace::with("supported.adoc");
    let output_path = workspace.output("out.pdf");

    let output = run(
        &[&workspace.input("supported.adoc"), &output_path],
        &["--date", FIXED_DATE],
    );

    let reported = String::from_utf8_lossy(&output.stdout);
    assert!(
        reported.contains("out.pdf") && reported.contains("bytes"),
        "the caller must be told what was produced, got: {reported}"
    );
}

#[test]
fn rendering_twice_produces_byte_identical_output() {
    let (_first_workspace, first) = render("supported.adoc");
    let (_second_workspace, second) = render("supported.adoc");

    assert_eq!(
        first, second,
        "identical input and an identical supplied date must produce identical bytes"
    );
}

#[test]
fn a_themed_document_breaks_the_page_only_where_geometry_changes() {
    let (_workspace, pdf) = render("themed.adoc");
    let sizes = page_sizes(&pdf);

    assert_eq!(
        sizes.len(),
        2,
        "one page for the default and large-print sections, one for the wide \
         section — got {sizes:?}"
    );

    let (first_width, first_height) = sizes[0];
    assert!(
        first_height > first_width,
        "the first page must stay portrait: the large-print section changes only \
         typography, so it must not break the page — got {sizes:?}"
    );

    let (second_width, second_height) = sizes[1];
    assert!(
        second_width > second_height,
        "the wide section changes page geometry, so it must start a new, \
         landscape page — got {sizes:?}"
    );
}

#[test]
fn the_typography_only_section_shares_a_page_with_what_precedes_it() {
    let (_workspace, pdf) = render("themed.adoc");
    let text = text_of(&pdf);

    assert!(
        text.contains("Set Larger"),
        "the restyled section must render, got:\n{text}"
    );
    assert!(
        text.contains("Turned Sideways"),
        "the geometry-changed section must render, got:\n{text}"
    );
    assert_eq!(
        page_sizes(&pdf).len(),
        2,
        "three sections over two pages means the typography change did not break one"
    );
}

#[test]
fn content_that_looks_like_instructions_is_rendered_as_text() {
    let (_workspace, pdf) = render("hostile.adoc");
    let sizes = page_sizes(&pdf);

    assert_eq!(
        sizes.len(),
        1,
        "content must not be able to introduce a page instruction, got {sizes:?}"
    );

    let (width, height) = sizes[0];
    assert!(
        width > 500.0 && height > 700.0,
        "the page must keep its A4 geometry: a `set page` in the body must be \
         inert text — got {width}x{height} points"
    );

    let text = text_of(&pdf);
    assert!(
        text.contains("set page"),
        "the payload must appear as visible text, got:\n{text}"
    );
}

#[test]
fn a_missing_input_fails_without_writing_an_output() {
    let workspace = Workspace::with("supported.adoc");
    let output_path = workspace.output("out.pdf");

    let output = run(
        &[&workspace.input("absent.adoc"), &output_path],
        &["--date", FIXED_DATE],
    );

    assert!(!output.status.success());
    assert!(
        !output_path.exists(),
        "a failed render must leave no output file"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("absent.adoc"),
        "the error must name the missing input"
    );
}

#[test]
fn an_input_outside_the_project_root_is_refused() {
    let workspace = Workspace::with("supported.adoc");
    let elsewhere = Workspace::with("hostile.adoc");
    let output_path = workspace.output("out.pdf");

    let output = run(
        &[&elsewhere.input("hostile.adoc"), &output_path],
        &[
            "--date",
            FIXED_DATE,
            "--project-root",
            &workspace.path.display().to_string(),
        ],
    );

    assert!(!output.status.success(), "the sandbox must refuse this");
    assert!(!output_path.exists());
}

#[test]
fn an_unreadable_date_is_a_usage_error() {
    let workspace = Workspace::with("supported.adoc");

    let output = run(
        &[
            &workspace.input("supported.adoc"),
            &workspace.output("out.pdf"),
        ],
        &["--date", "the day before yesterday"],
    );

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--date"),
        "the error must name the argument at fault"
    );
}

#[test]
fn a_skipped_construct_is_reported_on_standard_error() {
    // A table: still unsupported, and scheduled for tier 3 in
    // `docs/asciidoc-support.md`. This test used to use a list, which this
    // change made supported — so the example moved rather than the assertion.
    let workspace = Workspace::with("supported.adoc");
    let with_table = workspace.input("with-table.adoc");
    fs::write(
        &with_table,
        "= Title\n\nBefore.\n\n|===\n| one | two\n|===\n\nAfter.\n",
    )
    .unwrap();

    let output = run(
        &[&with_table, &workspace.output("out.pdf")],
        &["--date", FIXED_DATE],
    );

    assert!(
        output.status.success(),
        "an unsupported construct must not abort the render"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("skipped"),
        "the omission must never be silent, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
