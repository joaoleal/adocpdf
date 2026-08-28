//! What a caller sends across the boundary, and what comes back.
//!
//! These types are plain data. They validate nothing and decide nothing, so a
//! delivery layer can describe a request without linking the domain — which is
//! what lets the WASM surface stay small.
//!
//! Paths are strings rather than `PathBuf` because this is a wire shape: a
//! caller on the other side of a language boundary has a string, not an OS
//! path. Turning one into the other, and checking it, happens further in.
//!
//! The mapping between these and the domain's own types lives in
//! `adocpdf-adapters`. It cannot live here: this crate may depend only on
//! `adocpdf-core`, and it cannot live in the domain either, for the same
//! reason. The interface-adapter ring is the innermost one that sees both.

/// A request to render a document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderRequestDto {
    /// The AsciiDoc file to read.
    pub input: String,
    /// Where to write the PDF.
    pub output: String,
    /// The directory both must sit inside.
    ///
    /// `None` asks the application to derive one; it does not mean
    /// "unconstrained".
    pub project_root: Option<String>,
}

/// A construct the renderer left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedConstructDto {
    /// What was skipped.
    pub construct: String,
    /// The 1-based line it appeared on.
    pub line: u32,
    /// The 1-based column it appeared at.
    pub column: u32,
}

/// The outcome of a render.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderReportDto {
    /// Where the PDF was written.
    pub output: String,
    /// How many bytes it contains.
    pub bytes_written: u64,
    /// Everything left out, in source order.
    pub skipped: Vec<SkippedConstructDto>,
    /// How many page breaks the document's theme changes forced.
    pub forced_page_breaks: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_carries_the_three_paths_it_needs() {
        let request = RenderRequestDto {
            input: "book.adoc".to_owned(),
            output: "book.pdf".to_owned(),
            project_root: Some("/project".to_owned()),
        };

        assert_eq!(request.input, "book.adoc");
        assert_eq!(request.project_root.as_deref(), Some("/project"));
    }

    #[test]
    fn a_request_without_a_root_leaves_it_unset() {
        let request = RenderRequestDto {
            input: "book.adoc".to_owned(),
            output: "book.pdf".to_owned(),
            project_root: None,
        };

        assert!(
            request.project_root.is_none(),
            "an absent root asks the application to derive one, not to skip confinement"
        );
    }

    #[test]
    fn a_report_with_no_skipped_constructs_lists_none() {
        let report = RenderReportDto {
            output: "book.pdf".to_owned(),
            bytes_written: 1024,
            skipped: Vec::new(),
            forced_page_breaks: 0,
        };

        assert!(report.skipped.is_empty());
    }

    #[test]
    fn a_report_keeps_skipped_constructs_in_order() {
        let report = RenderReportDto {
            output: "book.pdf".to_owned(),
            bytes_written: 2048,
            skipped: vec![
                SkippedConstructDto {
                    construct: "table".to_owned(),
                    line: 4,
                    column: 1,
                },
                SkippedConstructDto {
                    construct: "admonition".to_owned(),
                    line: 9,
                    column: 1,
                },
            ],
            forced_page_breaks: 2,
        };

        let lines: Vec<u32> = report.skipped.iter().map(|s| s.line).collect();
        assert_eq!(lines, [4, 9]);
    }
}
