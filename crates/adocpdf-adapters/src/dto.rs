//! Translating between the boundary shapes and the domain's own types.
//!
//! This lives in infrastructure because it is the innermost layer that can see
//! both `adocpdf-shared` and `adocpdf-domain`. Neither of those may depend on
//! the other, so the translation has to sit outside both.
//!
//! Deriving a project root when the caller supplies none happens here too. It
//! is a defaulting rule about a wire shape, not a business rule: whatever root
//! comes out is still confined by the domain exactly as a supplied one is.

use std::path::{Path, PathBuf};

use adocpdf_domain::render_document::{RenderReport, RenderRequest};
use adocpdf_shared::render::{RenderReportDto, RenderRequestDto};

/// Turns a boundary request into a domain request.
///
/// When the caller supplies no project root, the input file's parent directory
/// becomes the root. An input with no parent — a bare file name — takes the
/// current directory.
#[must_use]
pub fn to_domain_request(dto: &RenderRequestDto) -> RenderRequest {
    let input = PathBuf::from(&dto.input);

    let project_root = dto.project_root.as_ref().map_or_else(
        || {
            input
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
        },
        PathBuf::from,
    );

    RenderRequest {
        input,
        output: PathBuf::from(&dto.output),
        project_root,
    }
}

/// Turns a domain report into a boundary report.
#[must_use]
pub fn from_domain_report(report: &RenderReport) -> RenderReportDto {
    RenderReportDto {
        output: report.output.display().to_string(),
        bytes_written: report.bytes_written as u64,
        skipped: report
            .skipped
            .iter()
            .map(|skipped| adocpdf_shared::render::SkippedConstructDto {
                construct: skipped.construct.clone(),
                line: skipped.location.line,
                column: skipped.location.column,
            })
            .collect(),
        forced_page_breaks: u32::try_from(report.forced_page_breaks).unwrap_or(u32::MAX),
    }
}

#[cfg(test)]
mod tests {
    use adocpdf_domain::error::SourceLocation;
    use adocpdf_domain::ports::SkippedConstruct;

    use super::*;

    #[test]
    fn a_supplied_root_is_used_as_given() {
        let dto = RenderRequestDto {
            input: "/project/book.adoc".to_owned(),
            output: "/project/book.pdf".to_owned(),
            project_root: Some("/project".to_owned()),
        };

        let request = to_domain_request(&dto);

        assert_eq!(request.project_root, PathBuf::from("/project"));
    }

    #[test]
    fn an_absent_root_defaults_to_the_input_directory() {
        let dto = RenderRequestDto {
            input: "/project/chapters/book.adoc".to_owned(),
            output: "/project/chapters/book.pdf".to_owned(),
            project_root: None,
        };

        let request = to_domain_request(&dto);

        assert_eq!(
            request.project_root,
            PathBuf::from("/project/chapters"),
            "the default must still be a real directory to confine against"
        );
    }

    #[test]
    fn a_bare_file_name_defaults_to_the_current_directory() {
        let dto = RenderRequestDto {
            input: "book.adoc".to_owned(),
            output: "book.pdf".to_owned(),
            project_root: None,
        };

        let request = to_domain_request(&dto);

        assert_eq!(request.project_root, PathBuf::from("."));
    }

    #[test]
    fn a_report_round_trips_its_skipped_constructs() {
        let report = RenderReport {
            output: PathBuf::from("/project/book.pdf"),
            bytes_written: 4096,
            skipped: vec![SkippedConstruct {
                construct: "table".to_owned(),
                location: SourceLocation::new(7, 3),
            }],
            forced_page_breaks: 2,
        };

        let dto = from_domain_report(&report);

        assert_eq!(dto.output, "/project/book.pdf");
        assert_eq!(dto.bytes_written, 4096);
        assert_eq!(dto.forced_page_breaks, 2);
        assert_eq!(dto.skipped.len(), 1);
        assert_eq!(dto.skipped[0].construct, "table");
        assert_eq!((dto.skipped[0].line, dto.skipped[0].column), (7, 3));
    }

    #[test]
    fn a_complete_report_lists_nothing_skipped() {
        let report = RenderReport {
            output: PathBuf::from("/project/book.pdf"),
            bytes_written: 10,
            skipped: Vec::new(),
            forced_page_breaks: 0,
        };

        assert!(from_domain_report(&report).skipped.is_empty());
    }

    #[test]
    fn a_request_keeps_the_paths_it_was_given() {
        let dto = RenderRequestDto {
            input: "in.adoc".to_owned(),
            output: "out.pdf".to_owned(),
            project_root: Some(".".to_owned()),
        };

        let request = to_domain_request(&dto);

        assert_eq!(request.input, PathBuf::from("in.adoc"));
        assert_eq!(request.output, PathBuf::from("out.pdf"));
    }
}
