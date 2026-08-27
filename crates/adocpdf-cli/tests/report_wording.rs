//! Exactly what the binary prints when a render succeeds.
//!
//! A characterisation test: it pins the report word for word, not by substring.
//! `end_to_end.rs` asserts only that the output *contains* `out.pdf`, `bytes`
//! and `skipped`, so a reworded, reordered or re-punctuated report passes it.
//! That looseness is fine for the questions those tests ask, and useless for
//! the one this file asks — whether moving the wording into a presenter left
//! the terminal output byte-identical.
//!
//! So: every line, its order, the stream it goes to, and the spelling of a
//! source position as `line N, column N`.
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

/// The date every render here is given, so no output depends on the day.
const FIXED_DATE: &str = "2026-08-16";

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A directory that holds one render, deleted when the test ends.
struct Workspace {
    path: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("adocpdf-report-{}-{name}-{unique}", process::id()));

        drop(fs::remove_dir_all(&path));
        fs::create_dir_all(&path).expect("can create the workspace");

        Self { path }
    }

    /// Writes `source` into the workspace and renders it.
    fn render(&self, source: &str) -> (PathBuf, Output) {
        let input = self.path.join("in.adoc");
        let output = self.path.join("out.pdf");
        fs::write(&input, source).expect("can write the source");

        let result = Command::new(env!("CARGO_BIN_EXE_adocpdf"))
            .arg(&input)
            .arg(&output)
            .args(["--date", FIXED_DATE])
            .output()
            .expect("the binary runs");

        assert!(
            result.status.success(),
            "the render must succeed, got: {}",
            String::from_utf8_lossy(&result.stderr)
        );

        (output, result)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.path));
    }
}

/// The success line, spelled the way the binary spells it.
///
/// The byte count is read back from the file rather than hard-coded, because a
/// Typst or font upgrade legitimately changes it and must not fail this test.
/// Everything around the two numbers — the verb, the parentheses, the unit, the
/// single trailing newline — is what is being pinned.
fn expected_success_line(output: &Path) -> String {
    let bytes = fs::metadata(output).expect("the PDF was written").len();

    format!("wrote {} ({bytes} bytes)\n", output.display())
}

#[test]
fn a_complete_render_prints_one_line_and_leaves_standard_error_empty() {
    let workspace = Workspace::new("complete");
    let (output, result) = workspace.render("= Title\n\nAn ordinary paragraph.\n");

    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        expected_success_line(&output)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stderr),
        "",
        "nothing was left out, so there is nothing to warn about"
    );
}

#[test]
fn a_render_that_skipped_constructs_names_each_one_and_then_counts_them() {
    let workspace = Workspace::new("skipped");
    let (output, result) = workspace.render(
        "= Title\n\nBefore.\n\n|===\n| one | two\n|===\n\n\
         Between.\n\n|===\n| three | four\n|===\n\nAfter.\n",
    );

    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        expected_success_line(&output),
        "what was produced is still reported on standard output, unchanged"
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stderr),
        "adocpdf: skipped table at line 5, column 1\n\
         adocpdf: skipped table at line 11, column 1\n\
         adocpdf: 2 construct(s) were not rendered\n",
        "each omission is named where it occurred, in source order, and the \
         count follows them"
    );
}
