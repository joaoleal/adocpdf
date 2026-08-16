//! The use case: turn an AsciiDoc file into a PDF file.

use std::path::PathBuf;

use crate::document_plan::plan_document;
use crate::error::DomainError;
use crate::ports::{
    Clock, DocumentParser, DocumentRenderer, SkippedConstruct, SourceStore, ThemeRepository,
};
use crate::sandbox::{PathResolver, ProjectRoot, SandboxedPath};

/// What the caller asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderRequest {
    /// The AsciiDoc file to read.
    pub input: PathBuf,
    /// Where to write the PDF.
    pub output: PathBuf,
    /// The directory both must sit inside.
    pub project_root: PathBuf,
}

/// What happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderReport {
    /// Where the PDF was written.
    pub output: PathBuf,
    /// How many bytes it contains.
    pub bytes_written: usize,
    /// Constructs left out of the render, in source order.
    ///
    /// Empty when the document used only supported constructs.
    pub skipped: Vec<SkippedConstruct>,
    /// How many page breaks the document's theme changes forced.
    pub forced_page_breaks: usize,
}

impl RenderReport {
    /// Whether anything in the source was left out.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.skipped.is_empty()
    }
}

/// Renders a document, given the ports to do it with.
///
/// The order of operations is the point: every path is confined before it is
/// touched, the source is parsed before any theme work, and the output is
/// written last. Writing last is what makes "a failed render leaves no output
/// file" true by construction rather than by cleanup.
pub struct RenderDocument<'a> {
    resolver: &'a dyn PathResolver,
    sources: &'a dyn SourceStore,
    parser: &'a dyn DocumentParser,
    themes: &'a dyn ThemeRepository,
    renderer: &'a dyn DocumentRenderer,
    clock: &'a dyn Clock,
}

impl std::fmt::Debug for RenderDocument<'_> {
    /// Names the type without attempting to describe its ports.
    ///
    /// Every field is a trait object, so there is nothing meaningful to print:
    /// a port's identity is which adapter was injected, which the composition
    /// root knows and this type does not.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderDocument").finish_non_exhaustive()
    }
}

impl<'a> RenderDocument<'a> {
    /// Assembles the use case from its ports.
    #[must_use]
    pub fn new(
        resolver: &'a dyn PathResolver,
        sources: &'a dyn SourceStore,
        parser: &'a dyn DocumentParser,
        themes: &'a dyn ThemeRepository,
        renderer: &'a dyn DocumentRenderer,
        clock: &'a dyn Clock,
    ) -> Self {
        Self {
            resolver,
            sources,
            parser,
            themes,
            renderer,
            clock,
        }
    }

    /// Renders the requested document.
    ///
    /// # Errors
    ///
    /// Returns whichever [`DomainError`] the failing step produced:
    /// [`DomainError::RootNotADirectory`] for an unusable project root,
    /// [`DomainError::PathOutsideRoot`] for a path escaping it,
    /// [`DomainError::InputNotFound`] or [`DomainError::InputUnreadable`] for
    /// the source, [`DomainError::ParseFailed`] for invalid AsciiDoc,
    /// [`DomainError::InvalidTheme`] or [`DomainError::UnknownTheme`] for
    /// theming, [`DomainError::LayoutFailed`] for typesetting, and
    /// [`DomainError::OutputUnwritable`] for the write.
    pub fn execute(&self, request: &RenderRequest) -> Result<RenderReport, DomainError> {
        let root = ProjectRoot::new(&request.project_root, self.resolver)?;
        let input = SandboxedPath::resolve(&request.input, &root, self.resolver)?;
        let output = SandboxedPath::resolve(&request.output, &root, self.resolver)?;

        let today = self.clock.today();
        let source = self.sources.read(&input)?;
        let outcome = self.parser.parse(&source, input.as_requested(), today)?;

        let themes = self.themes.load()?;
        let plan = plan_document(&outcome.document, &themes)?;

        let bytes = self.renderer.render(&plan, input.as_requested(), today)?;
        self.sources.write(&output, &bytes)?;

        Ok(RenderReport {
            output: output.as_path().to_path_buf(),
            bytes_written: bytes.len(),
            skipped: outcome.skipped,
            forced_page_breaks: plan.forced_page_breaks(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use adocpdf_core::document::{Block, Document, HeadingLevel, InlineText, Paragraph, Section};
    use adocpdf_core::theme::{ThemeId, ThemeSet, built_in_default_theme};

    use super::*;
    use crate::document_plan::LayoutPlan;
    use crate::error::SourceLocation;
    use crate::ports::{Date, ParseOutcome};
    use crate::sandbox::ResolutionError;

    /// Resolves anything under `/project`, and nothing else.
    struct FakeResolver;

    impl PathResolver for FakeResolver {
        fn resolve(&self, path: &Path) -> Result<PathBuf, ResolutionError> {
            Ok(PathBuf::from("/").join(path.strip_prefix("/").unwrap_or(path)))
        }

        fn is_directory(&self, path: &Path) -> bool {
            path == Path::new("/project")
        }
    }

    /// Records every write so a test can assert one did not happen.
    struct FakeStore {
        source: Result<String, DomainError>,
        writes: RefCell<Vec<(PathBuf, usize)>>,
        write_result: Result<(), DomainError>,
    }

    impl FakeStore {
        fn holding(source: &str) -> Self {
            Self {
                source: Ok(source.to_owned()),
                writes: RefCell::new(Vec::new()),
                write_result: Ok(()),
            }
        }

        fn failing_to_read(error: DomainError) -> Self {
            Self {
                source: Err(error),
                writes: RefCell::new(Vec::new()),
                write_result: Ok(()),
            }
        }

        fn failing_to_write(error: DomainError) -> Self {
            Self {
                source: Ok("= Title\n".to_owned()),
                writes: RefCell::new(Vec::new()),
                write_result: Err(error),
            }
        }

        fn wrote_nothing(&self) -> bool {
            self.writes.borrow().is_empty()
        }
    }

    impl SourceStore for FakeStore {
        fn read(&self, _path: &SandboxedPath) -> Result<String, DomainError> {
            self.source.clone()
        }

        fn write(&self, path: &SandboxedPath, bytes: &[u8]) -> Result<(), DomainError> {
            self.writes
                .borrow_mut()
                .push((path.as_path().to_path_buf(), bytes.len()));
            self.write_result.clone()
        }
    }

    struct FakeParser {
        outcome: Result<ParseOutcome, DomainError>,
        seen_date: RefCell<Option<Date>>,
    }

    impl FakeParser {
        fn producing(document: Document, skipped: Vec<SkippedConstruct>) -> Self {
            Self {
                outcome: Ok(ParseOutcome { document, skipped }),
                seen_date: RefCell::new(None),
            }
        }

        fn failing(error: DomainError) -> Self {
            Self {
                outcome: Err(error),
                seen_date: RefCell::new(None),
            }
        }
    }

    impl DocumentParser for FakeParser {
        fn parse(
            &self,
            _source: &str,
            _origin: &str,
            today: Date,
        ) -> Result<ParseOutcome, DomainError> {
            *self.seen_date.borrow_mut() = Some(today);
            self.outcome.clone()
        }
    }

    struct FakeThemes(Result<ThemeSet, DomainError>);

    impl ThemeRepository for FakeThemes {
        fn load(&self) -> Result<ThemeSet, DomainError> {
            self.0.clone()
        }
    }

    struct FakeRenderer {
        result: Result<Vec<u8>, DomainError>,
        seen_date: RefCell<Option<Date>>,
    }

    impl FakeRenderer {
        fn producing(bytes: &[u8]) -> Self {
            Self {
                result: Ok(bytes.to_vec()),
                seen_date: RefCell::new(None),
            }
        }

        fn failing(error: DomainError) -> Self {
            Self {
                result: Err(error),
                seen_date: RefCell::new(None),
            }
        }
    }

    impl DocumentRenderer for FakeRenderer {
        fn render(
            &self,
            _plan: &LayoutPlan,
            _origin: &str,
            today: Date,
        ) -> Result<Vec<u8>, DomainError> {
            *self.seen_date.borrow_mut() = Some(today);
            self.result.clone()
        }
    }

    struct FixedClock(Date);

    impl Clock for FixedClock {
        fn today(&self) -> Date {
            self.0
        }
    }

    fn a_date() -> Date {
        Date::new(2026, 8, 16).unwrap()
    }

    fn a_document() -> Document {
        Document::new()
            .with_title(InlineText::new("Report"))
            .with_block(Block::Paragraph(Paragraph::new(InlineText::new("Body."))))
    }

    fn a_request() -> RenderRequest {
        RenderRequest {
            input: PathBuf::from("/project/book.adoc"),
            output: PathBuf::from("/project/book.pdf"),
            project_root: PathBuf::from("/project"),
        }
    }

    struct Fixture {
        store: FakeStore,
        parser: FakeParser,
        themes: FakeThemes,
        renderer: FakeRenderer,
        clock: FixedClock,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                store: FakeStore::holding("= Report\n"),
                parser: FakeParser::producing(a_document(), Vec::new()),
                themes: FakeThemes(Ok(ThemeSet::default())),
                renderer: FakeRenderer::producing(b"%PDF-1.7 fake"),
                clock: FixedClock(a_date()),
            }
        }

        fn run(&self) -> Result<RenderReport, DomainError> {
            RenderDocument::new(
                &FakeResolver,
                &self.store,
                &self.parser,
                &self.themes,
                &self.renderer,
                &self.clock,
            )
            .execute(&a_request())
        }
    }

    #[test]
    fn a_document_renders_end_to_end() {
        let fixture = Fixture::new();

        let report = fixture.run().unwrap();

        assert_eq!(report.output, PathBuf::from("/project/book.pdf"));
        assert_eq!(report.bytes_written, b"%PDF-1.7 fake".len());
        assert!(report.is_complete());
    }

    #[test]
    fn the_rendered_bytes_are_the_ones_written() {
        let fixture = Fixture::new();

        fixture.run().unwrap();

        let writes = fixture.store.writes.borrow();
        assert_eq!(
            writes.as_slice(),
            &[(PathBuf::from("/project/book.pdf"), b"%PDF-1.7 fake".len())]
        );
    }

    #[test]
    fn the_injected_date_reaches_both_the_parser_and_the_renderer() {
        let fixture = Fixture::new();

        fixture.run().unwrap();

        assert_eq!(*fixture.parser.seen_date.borrow(), Some(a_date()));
        assert_eq!(*fixture.renderer.seen_date.borrow(), Some(a_date()));
    }

    #[test]
    fn skipped_constructs_are_reported_rather_than_swallowed() {
        let skipped = vec![SkippedConstruct {
            construct: "table".to_owned(),
            location: SourceLocation::new(9, 1),
        }];
        let fixture = Fixture {
            parser: FakeParser::producing(a_document(), skipped.clone()),
            ..Fixture::new()
        };

        let report = fixture.run().unwrap();

        assert_eq!(report.skipped, skipped);
        assert!(!report.is_complete());
    }

    #[test]
    fn forced_page_breaks_are_counted() {
        let wide = {
            use adocpdf_core::geometry::{Margins, PageGeometry};
            use adocpdf_core::length::Length;
            use adocpdf_core::theme::Theme;
            Theme::new(
                PageGeometry::new(
                    Length::from_millimeters(420.0).unwrap(),
                    Length::from_millimeters(297.0).unwrap(),
                    Margins::uniform(Length::from_millimeters(20.0).unwrap()),
                )
                .unwrap(),
                built_in_default_theme().typography().clone(),
            )
        };
        let document = Document::new().with_block(Block::Section(
            Section::new(InlineText::new("Wide"), HeadingLevel::new(1).unwrap())
                .with_theme(ThemeId::new("wide").unwrap())
                .with_block(Block::Paragraph(Paragraph::new(InlineText::new("Body.")))),
        ));
        let fixture = Fixture {
            parser: FakeParser::producing(document, Vec::new()),
            themes: FakeThemes(Ok(
                ThemeSet::default().with(ThemeId::new("wide").unwrap(), wide)
            )),
            ..Fixture::new()
        };

        let report = fixture.run().unwrap();

        assert_eq!(report.forced_page_breaks, 1);
    }

    #[test]
    fn a_missing_source_leaves_no_output_file() {
        let fixture = Fixture {
            store: FakeStore::failing_to_read(DomainError::InputNotFound {
                path: "/project/book.adoc".to_owned(),
            }),
            ..Fixture::new()
        };

        let error = fixture.run().unwrap_err();

        assert!(matches!(error, DomainError::InputNotFound { .. }));
        assert!(fixture.store.wrote_nothing());
    }

    #[test]
    fn a_parse_failure_leaves_no_output_file() {
        let fixture = Fixture {
            parser: FakeParser::failing(DomainError::ParseFailed {
                path: "/project/book.adoc".to_owned(),
                location: SourceLocation::new(3, 1),
                reason: "unterminated block".to_owned(),
            }),
            ..Fixture::new()
        };

        let error = fixture.run().unwrap_err();

        assert!(matches!(error, DomainError::ParseFailed { .. }));
        assert!(fixture.store.wrote_nothing());
    }

    #[test]
    fn an_invalid_theme_leaves_no_output_file() {
        let fixture = Fixture {
            themes: FakeThemes(Err(DomainError::InvalidTheme {
                id: "narrow".to_owned(),
                reason: "margins leave no printable width".to_owned(),
            })),
            ..Fixture::new()
        };

        let error = fixture.run().unwrap_err();

        assert!(matches!(error, DomainError::InvalidTheme { .. }));
        assert!(fixture.store.wrote_nothing());
    }

    #[test]
    fn an_unknown_theme_leaves_no_output_file() {
        let document = Document::new().with_block(Block::Section(
            Section::new(InlineText::new("Appendix"), HeadingLevel::new(1).unwrap())
                .with_theme(ThemeId::new("absent").unwrap()),
        ));
        let fixture = Fixture {
            parser: FakeParser::producing(document, Vec::new()),
            ..Fixture::new()
        };

        let error = fixture.run().unwrap_err();

        assert!(matches!(error, DomainError::UnknownTheme { .. }));
        assert!(fixture.store.wrote_nothing());
    }

    #[test]
    fn a_layout_failure_leaves_no_output_file() {
        let fixture = Fixture {
            renderer: FakeRenderer::failing(DomainError::LayoutFailed {
                path: "/project/book.adoc".to_owned(),
                reason: "cannot fit".to_owned(),
            }),
            ..Fixture::new()
        };

        let error = fixture.run().unwrap_err();

        assert!(matches!(error, DomainError::LayoutFailed { .. }));
        assert!(fixture.store.wrote_nothing());
    }

    #[test]
    fn an_unwritable_output_is_reported_as_such() {
        let fixture = Fixture {
            store: FakeStore::failing_to_write(DomainError::OutputUnwritable {
                path: "/project/book.pdf".to_owned(),
                reason: "permission denied".to_owned(),
            }),
            ..Fixture::new()
        };

        let error = fixture.run().unwrap_err();

        assert!(matches!(error, DomainError::OutputUnwritable { .. }));
    }

    #[test]
    fn an_input_outside_the_project_root_is_refused_before_anything_is_read() {
        let store = FakeStore::holding("= Report\n");
        let parser = FakeParser::producing(a_document(), Vec::new());
        let themes = FakeThemes(Ok(ThemeSet::default()));
        let renderer = FakeRenderer::producing(b"%PDF");
        let clock = FixedClock(a_date());

        let error = RenderDocument::new(&FakeResolver, &store, &parser, &themes, &renderer, &clock)
            .execute(&RenderRequest {
                input: PathBuf::from("/elsewhere/book.adoc"),
                output: PathBuf::from("/project/book.pdf"),
                project_root: PathBuf::from("/project"),
            })
            .unwrap_err();

        assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
        assert!(store.wrote_nothing());
        assert!(
            parser.seen_date.borrow().is_none(),
            "confinement must be checked before the source is parsed"
        );
    }

    #[test]
    fn an_output_outside_the_project_root_is_refused() {
        let fixture = Fixture::new();

        let error = RenderDocument::new(
            &FakeResolver,
            &fixture.store,
            &fixture.parser,
            &fixture.themes,
            &fixture.renderer,
            &fixture.clock,
        )
        .execute(&RenderRequest {
            input: PathBuf::from("/project/book.adoc"),
            output: PathBuf::from("/elsewhere/book.pdf"),
            project_root: PathBuf::from("/project"),
        })
        .unwrap_err();

        assert!(matches!(error, DomainError::PathOutsideRoot { .. }));
        assert!(fixture.store.wrote_nothing());
    }

    #[test]
    fn an_unusable_project_root_is_refused_first() {
        let fixture = Fixture::new();

        let error = RenderDocument::new(
            &FakeResolver,
            &fixture.store,
            &fixture.parser,
            &fixture.themes,
            &fixture.renderer,
            &fixture.clock,
        )
        .execute(&RenderRequest {
            input: PathBuf::from("/nowhere/book.adoc"),
            output: PathBuf::from("/nowhere/book.pdf"),
            project_root: PathBuf::from("/nowhere"),
        })
        .unwrap_err();

        assert!(matches!(error, DomainError::RootNotADirectory { .. }));
        assert!(fixture.store.wrote_nothing());
    }
}
