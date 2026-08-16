//! The engine's view of the outside world.
//!
//! `World` is Typst's own port — its inversion of control, not ours. It belongs
//! here in infrastructure and must never leak inward; the domain knows only
//! [`DocumentRenderer`](adocpdf_domain::ports::DocumentRenderer).
//!
//! This implementation is backed by memory, not the filesystem. Everything the
//! engine can read is something we put here, and everything we put here came
//! through the sandbox first. A world that read the disk directly would sit
//! *below* our confinement check, so a document could reach files the sandbox
//! never saw. It also makes the engine usable where there is no filesystem at
//! all, which the WASM target will need.

use std::fmt;

use adocpdf_domain::ports::Date;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};

use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use crate::fonts::EmbeddedFonts;

/// A world serving one document from memory.
pub struct InMemoryWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: EmbeddedFonts,
    main: Source,
    /// Additional files the document may read.
    ///
    /// Empty for now: the walking skeleton renders a single source with no
    /// includes. It exists so that adding includes later is a matter of filling
    /// it from sandbox-approved paths, rather than opening the world up to the
    /// filesystem.
    ///
    /// A list rather than a map because `FileId` is neither `Ord` nor usefully
    /// hashable here, and because a list has one iteration order — which is
    /// what byte-identical output requires.
    files: Vec<(FileId, Bytes)>,
    today: Date,
}

impl fmt::Debug for InMemoryWorld {
    /// Reports the world's shape, not its contents.
    ///
    /// The markup can be an entire document and the font is three quarters of a
    /// megabyte; printing either would bury whatever the reader was actually
    /// looking for.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InMemoryWorld")
            .field("markup_bytes", &self.main.text().len())
            .field("extra_files", &self.files.len())
            .field("fonts", &self.fonts.len())
            .field("today", &self.today)
            .finish_non_exhaustive()
    }
}

impl InMemoryWorld {
    /// Creates a world serving `markup` as the main document.
    #[must_use]
    pub fn new(markup: String, fonts: EmbeddedFonts, today: Date) -> Self {
        let book = FontBook::clone(fonts.book());

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main: Source::detached(markup),
            files: Vec::new(),
            today,
        }
    }

    /// The markup this world serves.
    #[must_use]
    pub fn markup(&self) -> &str {
        self.main.text()
    }
}

impl World for InMemoryWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            // Refused rather than looked up on disk. There is deliberately no
            // path from here to the filesystem.
            Err(FileError::NotSource)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map(|(_, bytes)| bytes.clone())
            .or_else(|| {
                (id == self.main.id()).then(|| Bytes::new(self.main.text().as_bytes().to_vec()))
            })
            .ok_or(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        // The offset is ignored: the date is supplied by the caller's clock, so
        // there is no local time to convert from and nothing that varies by
        // where the render happens to run.
        Datetime::from_ymd(self.today.year, self.today.month, self.today.day)
    }
}

#[cfg(test)]
mod tests {
    use typst::syntax::{RootedPath, VirtualPath, VirtualRoot};

    use super::*;

    /// A file id that is genuinely not the main document.
    ///
    /// `Source::detached` always interns the same id, so it cannot be used to
    /// stand in for "some other file".
    fn another_file() -> FileId {
        RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new("elsewhere.typ").unwrap(),
        )
        .intern()
    }

    fn a_world(markup: &str) -> InMemoryWorld {
        InMemoryWorld::new(
            markup.to_owned(),
            EmbeddedFonts::load(),
            Date::new(2026, 8, 16).unwrap(),
        )
    }

    #[test]
    fn the_main_source_is_the_markup_it_was_given() {
        let world = a_world("#par(\"Hello.\")");

        let source = world
            .source(world.main())
            .expect("the main source resolves");

        assert_eq!(source.text(), "#par(\"Hello.\")");
    }

    #[test]
    fn an_unknown_source_is_refused_rather_than_looked_up() {
        let world = a_world("#par(\"Hello.\")");

        assert!(
            world.source(another_file()).is_err(),
            "there must be no path from the engine to the filesystem"
        );
    }

    #[test]
    fn an_unknown_file_is_refused() {
        let world = a_world("#par(\"Hello.\")");

        assert!(world.file(another_file()).is_err());
    }

    #[test]
    fn the_date_is_the_injected_one() {
        let world = a_world("#par(\"Hello.\")");

        let today = world.today(None).expect("the injected date is valid");

        assert_eq!(today, Datetime::from_ymd(2026, 8, 16).unwrap());
    }

    #[test]
    fn the_embedded_font_is_reachable_by_index() {
        let world = a_world("#par(\"Hello.\")");

        assert!(world.font(0).is_some());
        assert!(world.font(99).is_none());
    }

    #[test]
    fn the_book_lists_the_embedded_family() {
        let world = a_world("#par(\"Hello.\")");

        assert!(world.book().families().count() >= 1);
    }
}
