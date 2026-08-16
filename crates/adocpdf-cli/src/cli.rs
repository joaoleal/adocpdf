//! Command-line arguments.
//!
//! Parsing only: this module turns a command line into a request and nothing
//! else. What the request means is decided further in.

use std::path::PathBuf;

use adocpdf_shared::render::RenderRequestDto;
use clap::Parser;

/// Render an AsciiDoc document to PDF.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(name = "adocpdf", version, about, long_about = None)]
pub(crate) struct Cli {
    /// The AsciiDoc file to render.
    pub(crate) input: PathBuf,

    /// Where to write the PDF.
    pub(crate) output: PathBuf,

    /// The directory every path must stay inside.
    ///
    /// Defaults to the input file's directory.
    #[arg(long, value_name = "DIR")]
    pub(crate) project_root: Option<PathBuf>,

    /// The date the document should treat as today, as `YYYY-MM-DD`.
    ///
    /// Supply this for a reproducible build: with it, the same source always
    /// produces the same bytes. Without it, the host clock is used.
    #[arg(long, value_name = "YYYY-MM-DD")]
    pub(crate) date: Option<String>,
}

impl Cli {
    /// Turns the parsed arguments into a render request.
    #[must_use]
    pub(crate) fn to_request(&self) -> RenderRequestDto {
        RenderRequestDto {
            input: self.input.display().to_string(),
            output: self.output.display().to_string(),
            project_root: self
                .project_root
                .as_ref()
                .map(|root| root.display().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::*;

    fn parse(arguments: &[&str]) -> Cli {
        Cli::try_parse_from(arguments).expect("the arguments are valid")
    }

    #[test]
    fn the_two_paths_are_taken_in_order() {
        let cli = parse(&["adocpdf", "book.adoc", "book.pdf"]);

        assert_eq!(cli.input, PathBuf::from("book.adoc"));
        assert_eq!(cli.output, PathBuf::from("book.pdf"));
    }

    #[test]
    fn the_project_root_is_optional() {
        assert!(
            parse(&["adocpdf", "book.adoc", "book.pdf"])
                .project_root
                .is_none()
        );
    }

    #[test]
    fn the_project_root_can_be_given() {
        let cli = parse(&[
            "adocpdf",
            "book.adoc",
            "book.pdf",
            "--project-root",
            "/project",
        ]);

        assert_eq!(cli.project_root, Some(PathBuf::from("/project")));
    }

    #[test]
    fn the_date_can_be_given() {
        let cli = parse(&["adocpdf", "book.adoc", "book.pdf", "--date", "2026-08-16"]);

        assert_eq!(cli.date.as_deref(), Some("2026-08-16"));
    }

    #[test]
    fn a_missing_output_is_a_usage_error() {
        let error =
            Cli::try_parse_from(["adocpdf", "book.adoc"]).expect_err("both paths are required");

        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn no_arguments_at_all_is_a_usage_error() {
        assert!(Cli::try_parse_from(["adocpdf"]).is_err());
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error() {
        let error = Cli::try_parse_from(["adocpdf", "a.adoc", "b.pdf", "--nonsense"])
            .expect_err("unknown flags must not be ignored");

        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn the_request_carries_the_paths_across() {
        let request = parse(&[
            "adocpdf",
            "book.adoc",
            "book.pdf",
            "--project-root",
            "/project",
        ])
        .to_request();

        assert_eq!(request.input, "book.adoc");
        assert_eq!(request.output, "book.pdf");
        assert_eq!(request.project_root.as_deref(), Some("/project"));
    }

    #[test]
    fn an_absent_root_stays_absent_in_the_request() {
        let request = parse(&["adocpdf", "book.adoc", "book.pdf"]).to_request();

        assert!(
            request.project_root.is_none(),
            "deriving the default is not the CLI's decision to make"
        );
    }
}
