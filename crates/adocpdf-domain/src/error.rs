//! The failure modes of rendering a document.

use std::fmt;

use thiserror::Error;

/// Where in a source document something happened.
///
/// Lines and columns are 1-based, matching how editors and AsciiDoc tooling
/// report positions, so a location can be pasted straight into an editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceLocation {
    /// The 1-based line number.
    pub line: u32,
    /// The 1-based column number.
    pub column: u32,
}

impl SourceLocation {
    /// The first position in a document.
    pub const START: Self = Self { line: 1, column: 1 };

    /// Creates a source location.
    #[must_use]
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Everything that can go wrong while rendering a document.
///
/// One enum rather than one per operation: a caller mapping failures to exit
/// codes needs to distinguish them from each other, and that is easier to get
/// right — and to test exhaustively — when the whole set is in one place.
///
/// Adapters map foreign error types into these variants at the boundary, so no
/// external error type travels inward.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// The input document does not exist.
    #[error("input document not found: {path}")]
    InputNotFound {
        /// The path as the caller supplied it.
        path: String,
    },

    /// The input document exists but could not be read.
    #[error("cannot read input {path}: {reason}")]
    InputUnreadable {
        /// The path as the caller supplied it.
        path: String,
        /// What the underlying system reported.
        reason: String,
    },

    /// The source could not be parsed as AsciiDoc.
    #[error("cannot parse {path} at {location}: {reason}")]
    ParseFailed {
        /// The document that failed to parse.
        path: String,
        /// Where in the source the parser gave up.
        location: SourceLocation,
        /// What the parser reported.
        reason: String,
    },

    /// The output file could not be written.
    #[error("cannot write output {path}: {reason}")]
    OutputUnwritable {
        /// The path as the caller supplied it.
        path: String,
        /// What the underlying system reported.
        reason: String,
    },

    /// Laying the document out failed.
    ///
    /// Distinct from [`DomainError::ParseFailed`]: the source was understood,
    /// but could not be turned into pages.
    #[error("cannot lay out {path}: {reason}")]
    LayoutFailed {
        /// The document being laid out.
        path: String,
        /// What the layout engine reported.
        reason: String,
    },

    /// A theme's settings are not usable.
    #[error("theme {id} is invalid: {reason}")]
    InvalidTheme {
        /// Which theme is at fault. The default theme reports as `default`.
        id: String,
        /// Which setting is wrong and why.
        reason: String,
    },

    /// A section referred to a theme that does not exist.
    #[error("section {section:?} refers to theme {id}, which is not defined")]
    UnknownTheme {
        /// The theme that was referenced.
        id: String,
        /// The heading of the section that referenced it.
        section: String,
    },

    /// A path resolves outside the project root.
    ///
    /// Deliberately reports only the path as requested and the root in effect.
    /// Naming the location the path actually resolved to would confirm what
    /// exists outside the sandbox, which is exactly what confinement is meant
    /// to prevent.
    #[error("{requested} is outside the project root {root}")]
    PathOutsideRoot {
        /// The path as the caller or the document supplied it.
        requested: String,
        /// The project root in effect.
        root: String,
    },

    /// A reference inside a document pointed outside the project root.
    #[error("{reference} at {location} in {path} points outside the project root {root}")]
    ReferenceOutsideRoot {
        /// The reference as it appeared in the source.
        reference: String,
        /// Where it appeared.
        location: SourceLocation,
        /// The document it appeared in.
        path: String,
        /// The project root in effect.
        root: String,
    },

    /// The supplied project root is not a usable directory.
    #[error("project root {root} does not exist or is not a directory")]
    RootNotADirectory {
        /// The root as the caller supplied it.
        root: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_input_names_the_path_that_was_not_found() {
        let error = DomainError::InputNotFound {
            path: "book.adoc".to_owned(),
        };

        assert!(error.to_string().contains("book.adoc"));
    }

    #[test]
    fn a_parse_failure_reports_where_it_happened() {
        let error = DomainError::ParseFailed {
            path: "book.adoc".to_owned(),
            location: SourceLocation::new(12, 5),
            reason: "unterminated block".to_owned(),
        };

        let message = error.to_string();

        assert!(message.contains("line 12"), "got: {message}");
        assert!(message.contains("column 5"), "got: {message}");
        assert!(message.contains("unterminated block"), "got: {message}");
    }

    #[test]
    fn an_unwritable_output_is_distinguishable_from_a_parse_or_layout_failure() {
        let output = DomainError::OutputUnwritable {
            path: "out.pdf".to_owned(),
            reason: "permission denied".to_owned(),
        };
        let parse = DomainError::ParseFailed {
            path: "book.adoc".to_owned(),
            location: SourceLocation::START,
            reason: "bad".to_owned(),
        };
        let layout = DomainError::LayoutFailed {
            path: "book.adoc".to_owned(),
            reason: "bad".to_owned(),
        };

        assert_ne!(output, parse);
        assert_ne!(output, layout);
        assert_ne!(parse, layout);
        assert!(output.to_string().contains("cannot write output"));
        assert!(parse.to_string().contains("cannot parse"));
        assert!(layout.to_string().contains("cannot lay out"));
    }

    #[test]
    fn an_unknown_theme_names_both_the_theme_and_the_section() {
        let error = DomainError::UnknownTheme {
            id: "wide".to_owned(),
            section: "Appendix".to_owned(),
        };

        let message = error.to_string();

        assert!(message.contains("wide"), "got: {message}");
        assert!(message.contains("Appendix"), "got: {message}");
    }

    #[test]
    fn an_invalid_theme_says_which_setting_is_wrong() {
        let error = DomainError::InvalidTheme {
            id: "narrow".to_owned(),
            reason: "left and right margins total 120pt, which leaves no printable width \
                     on a page 100pt wide"
                .to_owned(),
        };

        let message = error.to_string();

        assert!(message.contains("narrow"), "got: {message}");
        assert!(message.contains("margins"), "got: {message}");
    }

    #[test]
    fn an_out_of_root_path_reports_the_root_in_effect() {
        let error = DomainError::PathOutsideRoot {
            requested: "../../etc/passwd".to_owned(),
            root: "/home/user/project".to_owned(),
        };

        let message = error.to_string();

        assert!(message.contains("/home/user/project"), "got: {message}");
        assert!(message.contains("../../etc/passwd"), "got: {message}");
    }

    #[test]
    fn an_out_of_root_path_does_not_disclose_where_it_resolved_to() {
        // The variant has nowhere to put a resolved target: the only path it
        // carries is the one the caller already supplied. This is a structural
        // guarantee rather than a formatting choice, so it cannot regress by
        // someone editing a message.
        let error = DomainError::PathOutsideRoot {
            requested: "link.adoc".to_owned(),
            root: "/home/user/project".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "link.adoc is outside the project root /home/user/project"
        );
    }

    #[test]
    fn a_reference_outside_the_root_is_located_in_its_source() {
        let error = DomainError::ReferenceOutsideRoot {
            reference: "include::/etc/passwd[]".to_owned(),
            location: SourceLocation::new(4, 1),
            path: "book.adoc".to_owned(),
            root: "/home/user/project".to_owned(),
        };

        let message = error.to_string();

        assert!(message.contains("line 4"), "got: {message}");
        assert!(message.contains("book.adoc"), "got: {message}");
    }

    #[test]
    fn a_bad_project_root_names_the_root_that_was_supplied() {
        let error = DomainError::RootNotADirectory {
            root: "/nowhere".to_owned(),
        };

        assert!(error.to_string().contains("/nowhere"));
    }

    #[test]
    fn a_location_reads_the_way_an_editor_reports_one() {
        assert_eq!(SourceLocation::new(3, 17).to_string(), "line 3, column 17");
        assert_eq!(SourceLocation::START.to_string(), "line 1, column 1");
    }
}
