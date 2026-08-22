//! Reading a laid-out page, rather than the markup that produced it.
//!
//! Every claim this module supports is about geometry: where a line broke, what
//! column an item's text starts in, how tall a heading is set. None of those can
//! be answered by asserting on emitted markup — which is exactly how three
//! layout defects survived a suite that asserted on markup alone. A test that
//! reads `#strong("word")` out of a string knows the emitter said "bold"; it
//! does not know a single glyph reached the page.
//!
//! The document goes through the real path — parse, plan, emit, compile — so
//! what is measured here is what a reader gets.
// `dead_code` and `unreachable_pub`: this module is included by several test
// binaries, each of which uses a different part of it. Within any one binary
// the rest looks unused and privately reachable; across the suite it is
// neither.
#![allow(dead_code, unreachable_pub, clippy::expect_used, clippy::unwrap_used)]

use adocpdf_core::theme::ThemeSet;
use adocpdf_domain::document_plan::plan_document;
use adocpdf_domain::ports::{Date, DocumentParser, ThemeRepository};
use adocpdf_infra::emitter::emit;
use adocpdf_infra::fonts::EmbeddedFonts;
use adocpdf_infra::parser::AsciidocParser;
use adocpdf_infra::themes::BuiltInThemes;
use adocpdf_infra::world::InMemoryWorld;
use typst::layout::{Frame, FrameItem, Point};
use typst_layout::PagedDocument;

/// A run of text as the engine placed it.
///
/// One run is whatever the engine chose to shape together, which is not a word,
/// a line or a paragraph — it is only ever a fragment. [`Page::lines`] is what
/// turns runs into something a test can reason about.
#[derive(Debug, Clone)]
pub struct Run {
    /// The characters in the run.
    pub text: String,
    /// Distance from the page's left edge, in points.
    pub x: f64,
    /// Distance from the page's top edge, in points. This is the run's
    /// baseline, which is why two runs on one line share it exactly.
    pub y: f64,
    /// The family the glyphs were taken from, as the face reports it.
    pub family: String,
    /// The size the run was set at, in points.
    pub size: f64,
}

/// One laid-out page.
#[derive(Debug, Clone)]
pub struct Page {
    /// Every run on the page, in the order the engine emitted them.
    pub runs: Vec<Run>,
    /// The page's width in points.
    pub width: f64,
    /// The page's height in points.
    pub height: f64,
}

/// The markup a source produces, for the few claims that are about the markup.
///
/// Most questions here are about the page. This one is not: whether a marker is
/// written as text for the engine to read back can only be asked of what was
/// written.
pub fn markup(source: &str) -> String {
    let today = Date::new(2026, 8, 22).expect("a real date");
    let outcome = AsciidocParser
        .parse(source, "layout.adoc", today)
        .expect("the fixture parses");
    let plan = plan_document(&outcome.document, &ThemeSet::default()).expect("the fixture plans");

    emit(&plan)
}

/// Renders `source` under the default themes and returns its pages.
pub fn render(source: &str) -> Vec<Page> {
    render_with(source, &ThemeSet::default())
}

/// The themes the binary actually ships, `wide` and `large-print` included.
///
/// [`ThemeSet::default`] carries only the default theme, so a fixture naming a
/// section theme has to be rendered against these instead.
pub fn built_in_themes() -> ThemeSet {
    BuiltInThemes::new()
        .load()
        .expect("the built-in themes load")
}

/// Renders `source` under `themes` and returns its pages.
///
/// # Panics
///
/// Panics if the document does not parse, plan or compile. A test that wants to
/// assert about a page has no use for a `Result` it would only unwrap.
pub fn render_with(source: &str, themes: &ThemeSet) -> Vec<Page> {
    let today = Date::new(2026, 8, 22).expect("a real date");
    let outcome = AsciidocParser
        .parse(source, "layout.adoc", today)
        .expect("the fixture parses");
    let plan = plan_document(&outcome.document, themes).expect("the fixture plans");

    let world = InMemoryWorld::new(emit(&plan), EmbeddedFonts::load(), today);
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .expect("the emitted markup compiles");

    document
        .pages()
        .iter()
        .map(|page| {
            let mut runs = Vec::new();
            collect(&page.frame, Point::zero(), &mut runs);
            Page {
                runs,
                width: page.frame.width().to_pt(),
                height: page.frame.height().to_pt(),
            }
        })
        .collect()
}

/// Walks a frame, carrying the offset of every group it descends into.
///
/// The offset has to be accumulated rather than read from the item: a frame
/// item's position is relative to the frame holding it, so a run inside three
/// nested groups reports a position three groups deep. Adding them as the walk
/// descends is what makes the numbers comparable between runs.
fn collect(frame: &Frame, origin: Point, runs: &mut Vec<Run>) {
    for (offset, item) in frame.items() {
        let at = origin + *offset;
        match item {
            FrameItem::Text(text) => runs.push(Run {
                text: text.text.to_string(),
                x: at.x.to_pt(),
                y: at.y.to_pt(),
                family: text.font.info().family.clone(),
                size: text.size.to_pt(),
            }),
            FrameItem::Group(group) => collect(&group.frame, at, runs),
            _ => {}
        }
    }
}

/// One line of a laid-out page: the runs that share a baseline.
#[derive(Debug, Clone)]
pub struct Line {
    /// The line's text, runs joined in reading order.
    pub text: String,
    /// The shared baseline, in points from the top of the page.
    pub y: f64,
    /// The left edge of the leftmost run on the line, in points.
    pub left: f64,
    /// The size of the largest run on the line, in points.
    pub size: f64,
}

/// How far apart two baselines may be and still count as one line.
///
/// Not zero: a run set at a different size on the same line — an inline
/// monospace word, a superscript's host text — is placed by the same baseline
/// but arrives through a different code path, and floating-point equality is
/// the wrong question to ask of a laid-out coordinate. A quarter of a point is
/// far below the tightest leading a theme can ask for and far above the noise.
const SAME_LINE: f64 = 0.25;

impl Page {
    /// The page's lines, in reading order.
    ///
    /// Runs are grouped by baseline rather than by the order the engine emitted
    /// them: a line's runs are contiguous in practice, but nothing promises it,
    /// and a test that assumed it would be asserting about emission order while
    /// claiming to assert about the page.
    pub fn lines(&self) -> Vec<Line> {
        let mut lines: Vec<Vec<&Run>> = Vec::new();

        for run in &self.runs {
            match lines.iter_mut().find(|line| {
                line.first()
                    .is_some_and(|first| (first.y - run.y).abs() < SAME_LINE)
            }) {
                Some(line) => line.push(run),
                None => lines.push(vec![run]),
            }
        }

        for line in &mut lines {
            line.sort_by(|left, right| left.x.total_cmp(&right.x));
        }
        lines.sort_by(|first, second| baseline(first).total_cmp(&baseline(second)));

        lines
            .into_iter()
            .map(|line| Line {
                text: line.iter().map(|run| run.text.as_str()).collect(),
                y: baseline(&line),
                left: line.iter().map(|run| run.x).fold(f64::INFINITY, f64::min),
                size: line.iter().map(|run| run.size).fold(0.0, f64::max),
            })
            .collect()
    }

    /// Every run's text, joined in the order the engine emitted them.
    ///
    /// Answers "did this reach the page at all", which is a different and
    /// weaker question than [`Page::lines`] answers.
    pub fn text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }

    /// The family each run was set in, in emission order.
    pub fn families(&self) -> Vec<String> {
        self.runs.iter().map(|run| run.family.clone()).collect()
    }

    /// The text of every line, in reading order.
    pub fn line_texts(&self) -> Vec<String> {
        self.lines().into_iter().map(|line| line.text).collect()
    }
}

/// The baseline a group of runs was placed on.
///
/// Zero for an empty group, which cannot occur: a group exists because a run
/// created it. Returning a value rather than panicking keeps the sort total.
fn baseline(runs: &[&Run]) -> f64 {
    runs.first().map_or(0.0, |run| run.y)
}

/// Whether two runs were set on the same line.
pub fn share_baseline(one: &Run, other: &Run) -> bool {
    (one.y - other.y).abs() < SAME_LINE
}

/// The first run whose text contains `needle`.
///
/// # Panics
///
/// Panics when no run contains it, listing what was on the page — a test that
/// cannot find its subject has failed, and the page is what it needs to see.
pub fn run_containing<'a>(page: &'a Page, needle: &str) -> &'a Run {
    page.runs
        .iter()
        .find(|run| run.text.contains(needle))
        .unwrap_or_else(|| {
            panic!(
                "no run contains {needle:?}; the page holds {:?}",
                page.runs.iter().map(|run| &run.text).collect::<Vec<_>>()
            )
        })
}
