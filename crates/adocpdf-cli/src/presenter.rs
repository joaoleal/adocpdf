//! Turning a finished render into the words a terminal sees.
//!
//! This module reads `RenderReportDto` — the boundary shape — and never the
//! domain result it was converted from. That is the point of it: a result
//! produced by the use case must not reach the terminal without crossing the
//! boundary built for it, so the conversion happens in `main` and only the DTO
//! arrives here.
//!
//! One consequence is visible below. The domain's `SourceLocation` has a
//! `Display` that writes `line N, column N`, and the DTO carries the two
//! numbers instead of that string. So this module spells the position itself.
//! The duplication is deliberate: a wire shape should carry numbers, and how a
//! terminal spells a position is a presentation choice.

use adocpdf_shared::render::{RenderReportDto, SkippedConstructDto};

/// Prints what was produced, and anything that was left out.
///
/// Skipped constructs go to standard error rather than standard output: they
/// are a warning about incomplete output, and they must be visible even when
/// the success line is redirected somewhere.
pub(crate) fn report_success(report: &RenderReportDto) {
    println!("{}", produced_line(report));

    for line in omission_lines(report) {
        eprintln!("{line}");
    }
}

/// What was written, and how large it turned out.
fn produced_line(report: &RenderReportDto) -> String {
    format!("wrote {} ({} bytes)", report.output, report.bytes_written)
}

/// Every warning the render earned, in the order it is printed.
///
/// The per-construct lines come first and the count last, so a reader who
/// stops at the first line still learns where the omission was, and one who
/// reads to the end learns how many there were.
fn omission_lines(report: &RenderReportDto) -> Vec<String> {
    if report.skipped.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<String> = report.skipped.iter().map(omission_line).collect();
    lines.push(format!(
        "adocpdf: {} construct(s) were not rendered",
        report.skipped.len()
    ));

    lines
}

/// One omission, named and placed.
fn omission_line(skipped: &SkippedConstructDto) -> String {
    format!(
        "adocpdf: skipped {} at line {}, column {}",
        skipped.construct, skipped.line, skipped.column
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skipped(construct: &str, line: u32, column: u32) -> SkippedConstructDto {
        SkippedConstructDto {
            construct: construct.to_owned(),
            line,
            column,
        }
    }

    #[test]
    fn a_complete_render_is_announced_as_a_path_and_a_size() {
        let report = RenderReportDto {
            output: "/project/out.pdf".to_owned(),
            bytes_written: 16_556,
            skipped: Vec::new(),
            forced_page_breaks: 0,
        };

        assert_eq!(
            produced_line(&report),
            "wrote /project/out.pdf (16556 bytes)"
        );
    }

    #[test]
    fn a_complete_render_warns_about_nothing() {
        let report = RenderReportDto {
            output: "/project/out.pdf".to_owned(),
            bytes_written: 16_556,
            skipped: Vec::new(),
            forced_page_breaks: 0,
        };

        assert!(
            omission_lines(&report).is_empty(),
            "nothing was left out, so there is nothing to warn about"
        );
    }

    #[test]
    fn each_omission_is_named_and_placed_before_the_count() {
        let report = RenderReportDto {
            output: "/project/out.pdf".to_owned(),
            bytes_written: 16_556,
            skipped: vec![skipped("table", 5, 1), skipped("table", 11, 1)],
            forced_page_breaks: 0,
        };

        assert_eq!(
            omission_lines(&report),
            [
                "adocpdf: skipped table at line 5, column 1",
                "adocpdf: skipped table at line 11, column 1",
                "adocpdf: 2 construct(s) were not rendered",
            ]
        );
    }

    #[test]
    fn a_position_is_spelled_the_way_the_domain_spells_it() {
        // The domain's `SourceLocation` writes `line N, column N`, and this
        // module reproduces that from two numbers. If either spelling changes,
        // the two drift apart — so the wording is asserted here on its own.
        assert_eq!(
            omission_line(&skipped("admonition", 42, 7)),
            "adocpdf: skipped admonition at line 42, column 7"
        );
    }
}
