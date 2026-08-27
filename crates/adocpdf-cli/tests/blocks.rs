//! The block constructs this tier honours, mapped from real AsciiDoc sources.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod layout;

use adocpdf_asciidoc::parser::AsciidocParser;
use adocpdf_core::document::{
    AdmonitionKind, Block, BreakKind, ContainerKind, ListKind, QuotationKind, VerbatimKind,
};
use adocpdf_domain::ports::{Date, DocumentParser, ParseOutcome};

fn parse(source: &str) -> ParseOutcome {
    AsciidocParser::new()
        .parse(source, "book.adoc", Date::new(2026, 8, 16).unwrap())
        .expect("the source parses")
}

/// The first block of a document, with any title unwrapped.
fn first(source: &str) -> Block {
    parse(source)
        .document
        .body()
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("the source produced no blocks: {source:?}"))
}

fn text_of(block: &Block) -> String {
    match block {
        Block::Paragraph(paragraph) => paragraph.text().plain_text(),
        Block::Verbatim(verbatim) => verbatim.content().to_owned(),
        Block::Admonition(admonition) => admonition.body().iter().map(text_of).collect(),
        Block::Quotation(quotation) => quotation.body().iter().map(text_of).collect(),
        Block::Container(container) => container.body().iter().map(text_of).collect(),
        Block::List(list) => list
            .items()
            .iter()
            .map(|item| item.body().iter().map(text_of).collect::<String>())
            .collect(),
        Block::Titled { block, .. } => text_of(block),
        Block::Section(section) => section.body().iter().map(text_of).collect(),
        Block::Heading { text, .. } => text.plain_text(),
        Block::Break(_) => String::new(),
    }
}

// Verbatim blocks.

#[test]
fn a_listing_block_keeps_its_whitespace_and_blank_lines() {
    let Block::Verbatim(verbatim) = first("----\nfn main() {\n\n    println!();\n}\n----\n") else {
        panic!("expected a verbatim block");
    };

    assert_eq!(verbatim.kind(), VerbatimKind::Listing);
    assert_eq!(verbatim.content(), "fn main() {\n\n    println!();\n}");
}

#[test]
fn a_literal_block_is_distinguished_from_a_listing() {
    let Block::Verbatim(verbatim) = first("....\nplain text\n....\n") else {
        panic!("expected a verbatim block");
    };

    assert_eq!(verbatim.kind(), VerbatimKind::Literal);
}

#[test]
fn an_indented_paragraph_is_literal_content() {
    let Block::Verbatim(verbatim) = first("  indented, so literal\n") else {
        panic!("expected an indented paragraph to be literal content");
    };

    assert_eq!(verbatim.kind(), VerbatimKind::Literal);
    assert!(verbatim.content().contains("indented, so literal"));
}

#[test]
fn verbatim_content_is_not_substituted() {
    let Block::Verbatim(verbatim) = first("----\n*not bold* and {attr} and (C) and a < b\n----\n")
    else {
        panic!("expected a verbatim block");
    };

    assert_eq!(
        verbatim.content(),
        "*not bold* and {attr} and (C) and a < b",
        "no substitution may be applied inside a verbatim block"
    );
}

#[test]
fn a_fenced_code_block_is_a_listing() {
    let Block::Verbatim(verbatim) = first("```\ncode here\n```\n") else {
        panic!("expected a verbatim block from a fenced code block");
    };

    assert_eq!(verbatim.content(), "code here");
}

#[test]
fn an_unterminated_verbatim_block_does_not_abort_the_render() {
    let outcome = parse("Before.\n\n----\nnever closed\n");

    assert!(
        !outcome.document.body().is_empty(),
        "what could be understood must still render"
    );
}

// Admonitions.

#[test]
fn every_admonition_kind_is_recognised_in_paragraph_form() {
    for (marker, expected) in [
        ("NOTE", AdmonitionKind::Note),
        ("TIP", AdmonitionKind::Tip),
        ("IMPORTANT", AdmonitionKind::Important),
        ("CAUTION", AdmonitionKind::Caution),
        ("WARNING", AdmonitionKind::Warning),
    ] {
        let Block::Admonition(admonition) = first(&format!("{marker}: Mind this.\n")) else {
            panic!("expected an admonition for {marker}");
        };

        assert_eq!(admonition.kind(), expected);
        assert_eq!(admonition.kind().label(), marker);
        assert_eq!(text_of(&Block::Admonition(admonition)), "Mind this.");
    }
}

#[test]
fn a_delimited_admonition_may_hold_several_blocks() {
    let Block::Admonition(admonition) = first("[NOTE]\n====\nFirst.\n\nSecond.\n====\n") else {
        panic!("expected an admonition");
    };

    assert_eq!(admonition.kind(), AdmonitionKind::Note);
    assert_eq!(
        admonition.body().len(),
        2,
        "both paragraphs belong to the one admonition"
    );
}

// Quotations.

#[test]
fn a_quote_carries_its_attribution_and_citation() {
    let Block::Quotation(quotation) =
        first("[quote,Ursula K. Le Guin,The Dispossessed]\n____\nA wall is a wall.\n____\n")
    else {
        panic!("expected a quotation");
    };

    assert_eq!(quotation.kind(), QuotationKind::Quote);
    assert_eq!(
        quotation
            .attribution()
            .map(adocpdf_core::InlineText::plain_text),
        Some("Ursula K. Le Guin".to_owned())
    );
    assert_eq!(
        quotation
            .citation()
            .map(adocpdf_core::InlineText::plain_text),
        Some("The Dispossessed".to_owned())
    );
}

#[test]
fn a_quote_without_attribution_still_renders() {
    let Block::Quotation(quotation) = first("[quote]\n____\nJust the words.\n____\n") else {
        panic!("expected a quotation");
    };

    assert!(quotation.attribution().is_none());
    assert!(quotation.citation().is_none());
    assert_eq!(text_of(&Block::Quotation(quotation)), "Just the words.");
}

#[test]
fn a_verse_is_distinguished_from_a_quote() {
    let Block::Quotation(quotation) = first("[verse]\n____\nline one\nline two\n____\n") else {
        panic!("expected a quotation");
    };

    assert_eq!(quotation.kind(), QuotationKind::Verse);
}

// Compound containers.

#[test]
fn each_container_kind_is_recognised() {
    for (source, expected) in [
        ("====\nInside.\n====\n", ContainerKind::Example),
        ("****\nInside.\n****\n", ContainerKind::Sidebar),
        ("--\nInside.\n--\n", ContainerKind::Open),
    ] {
        let Block::Container(container) = first(source) else {
            panic!("expected a container for {source:?}");
        };

        assert_eq!(container.kind(), expected);
        assert_eq!(text_of(&Block::Container(container)), "Inside.");
    }
}

#[test]
fn containers_nest() {
    let Block::Container(outer) = first("====\n****\nDeep inside.\n****\n====\n") else {
        panic!("expected the outer container");
    };

    assert_eq!(outer.kind(), ContainerKind::Example);
    let Block::Container(inner) = &outer.body()[0] else {
        panic!("expected a nested container, got {:?}", outer.body());
    };
    assert_eq!(inner.kind(), ContainerKind::Sidebar);
}

#[test]
fn a_sidebar_renders_its_nested_content() {
    let Block::Container(sidebar) = first("****\nA paragraph.\n\n* an item\n****\n") else {
        panic!("expected a sidebar");
    };

    assert_eq!(sidebar.body().len(), 2, "got {:?}", sidebar.body());
    assert!(matches!(sidebar.body()[1], Block::List(_)));
}

// Breaks.

#[test]
fn both_breaks_are_recognised_and_distinguished() {
    assert!(matches!(
        first("Before.\n\n'''\n\nAfter.\n"),
        Block::Paragraph(_)
    ));

    let blocks = parse("Before.\n\n'''\n\nAfter.\n").document.body().to_vec();
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, Block::Break(BreakKind::Thematic))),
        "expected a thematic break, got {blocks:?}"
    );

    let paged = parse("Before.\n\n<<<\n\nAfter.\n").document.body().to_vec();
    assert!(
        paged
            .iter()
            .any(|b| matches!(b, Block::Break(BreakKind::Page))),
        "expected a page break, got {paged:?}"
    );
}

// Block titles.

#[test]
fn a_titled_block_keeps_its_title_with_the_block() {
    let Block::Titled { title, block } = first(".The Title\n----\ncode\n----\n") else {
        panic!("expected a titled block");
    };

    assert_eq!(title.plain_text(), "The Title");
    assert!(matches!(*block, Block::Verbatim(_)));
}

#[test]
fn a_block_title_does_not_affect_section_nesting() {
    let outcome = parse("== A heading\n\n.A block title\nA paragraph.\n");
    let Block::Section(section) = &outcome.document.body()[0] else {
        panic!("expected a section");
    };

    assert_eq!(section.level().get(), 1);
    assert_eq!(section.body().len(), 1, "the title is not a second block");
    assert!(matches!(section.body()[0], Block::Titled { .. }));
}

// Comments.

#[test]
fn comments_reach_neither_the_page_nor_the_report() {
    let outcome = parse("Before.\n\n// a line comment\n\n////\na comment block\n////\n\nAfter.\n");

    let text: String = outcome.document.body().iter().map(text_of).collect();
    assert!(
        !text.contains("comment"),
        "a comment reached the page: {text}"
    );
    assert!(
        outcome.skipped.is_empty(),
        "a comment is content the author asked to omit, not a construct that was skipped: {:?}",
        outcome.skipped
    );
}

// Lists.

#[test]
fn an_unordered_list_keeps_its_items() {
    let Block::List(list) = first("* one\n* two\n* three\n") else {
        panic!("expected a list");
    };

    assert_eq!(list.kind(), ListKind::Unordered);
    assert_eq!(list.items().len(), 3);
}

#[test]
fn an_ordered_list_is_distinguished_from_an_unordered_one() {
    let Block::List(list) = first(". one\n. two\n") else {
        panic!("expected a list");
    };

    assert_eq!(list.kind(), ListKind::Ordered);
    assert_eq!(list.items().len(), 2);
}

#[test]
fn a_nested_list_nests_inside_its_parent_item() {
    let Block::List(list) = first("* one\n** nested\n* two\n") else {
        panic!("expected a list");
    };

    assert_eq!(list.items().len(), 2, "the nested list is not a third item");
    assert!(
        list.items()[0]
            .body()
            .iter()
            .any(|block| matches!(block, Block::List(_))),
        "the nested list belongs to the first item, got {:?}",
        list.items()[0].body()
    );
}

#[test]
fn a_description_list_pairs_terms_with_descriptions() {
    let Block::List(list) = first("first:: the first thing\nsecond:: the second thing\n") else {
        panic!("expected a list");
    };

    assert_eq!(list.kind(), ListKind::Description);
    assert_eq!(list.items().len(), 2);
    assert_eq!(
        list.items()[0]
            .term()
            .map(adocpdf_core::InlineText::plain_text),
        Some("first".to_owned())
    );
    assert!(text_of(&Block::List(list)).contains("the first thing"));
}

#[test]
fn a_continuation_keeps_a_block_inside_its_item() {
    let Block::List(list) = first("* one\n+\nStill part of the first item.\n\n* two\n") else {
        panic!("expected a list");
    };

    assert_eq!(list.items().len(), 2);
    assert!(
        text_of(&Block::List(list.clone())).contains("Still part of the first item."),
        "the continued paragraph must survive"
    );
    assert!(
        list.items()[0].body().len() >= 2,
        "the continued paragraph belongs to the first item, got {:?}",
        list.items()[0].body()
    );
}

#[test]
fn inline_formatting_works_inside_every_container() {
    for source in [
        "NOTE: A *bold* word.\n",
        "****\nA *bold* word.\n****\n",
        "* A *bold* word.\n",
        "[quote]\n____\nA *bold* word.\n____\n",
    ] {
        let text = text_of(&first(source));

        assert!(
            text.contains("bold") && !text.contains('*'),
            "formatting must be interpreted inside {source:?}, got {text:?}"
        );
    }
}

// What the reader actually gets: the same constructs through the engine.

mod rendered {
    use super::layout;
    use adocpdf_core::theme::{Theme, ThemeSet, built_in_default_theme};
    use adocpdf_core::typography::{FontFamily, Typography};

    /// Renders a source document and returns the text of each page.
    ///
    /// The laid-out frames are read rather than the finished PDF's text,
    /// because the question several of these tests ask is *which page*
    /// something landed on, and a frame knows that exactly.
    fn pages(source: &str) -> Vec<String> {
        layout::render(source)
            .iter()
            .map(layout::Page::text)
            .collect()
    }

    /// The families every glyph on the first page was set in.
    fn families(source: &str, themes: &ThemeSet) -> Vec<String> {
        layout::render_with(source, themes)
            .first()
            .expect("one page")
            .families()
    }

    /// A theme whose monospace family is deliberately *not* the engine's own
    /// default, so that a rule which fails to name it is visible.
    ///
    /// `DejaVu Sans` is a proportional face and a strange choice for verbatim
    /// text. That is the point: both families are embedded, so the theme is
    /// valid, and only a renderer that honours the theme will use it.
    fn theme_with_proportional_verbatim() -> ThemeSet {
        let default = built_in_default_theme();
        let typography = Typography::new(
            default.typography().family().clone(),
            default.typography().size(),
            default.typography().leading(),
        )
        .with_monospace_family(FontFamily::new("DejaVu Sans").expect("a real family"));

        ThemeSet::new(Theme::new(*default.geometry(), typography))
    }

    #[test]
    fn a_page_break_starts_a_new_page() {
        let pages = pages("Before the break.\n\n<<<\n\nAfter the break.\n");

        let before = pages
            .iter()
            .position(|page| page.contains("Before the break"))
            .expect("the first paragraph is somewhere");
        let after = pages
            .iter()
            .position(|page| page.contains("After the break"))
            .expect("the second paragraph is somewhere");

        assert!(
            after > before,
            "the page break must move the second paragraph to a later page"
        );
    }

    #[test]
    fn a_thematic_break_does_not_start_a_new_page() {
        let pages = pages("Before the break.\n\n'''\n\nAfter the break.\n");

        assert!(
            pages[0].contains("Before the break") && pages[0].contains("After the break"),
            "a thematic break divides the text without breaking the page"
        );
    }

    #[test]
    fn an_admonition_carries_its_label_onto_the_page() {
        let pages = pages("WARNING: Mind the gap.\n");

        assert!(pages[0].contains("WARNING"), "got {:?}", pages[0]);
        assert!(pages[0].contains("Mind the gap"));
    }

    #[test]
    fn a_quotation_carries_its_attribution_onto_the_page() {
        let pages = pages("[quote,A Name,A Work]\n____\nThe words.\n____\n");

        assert!(pages[0].contains("The words"));
        assert!(pages[0].contains("A Name"), "got {:?}", pages[0]);
        assert!(pages[0].contains("A Work"));
    }

    #[test]
    fn an_ordered_list_numbers_its_items_on_the_page() {
        let pages = pages(". first\n. second\n. third\n");

        for number in ["1.", "2.", "3."] {
            assert!(
                pages[0].contains(number),
                "expected {number} on the page, got {:?}",
                pages[0]
            );
        }
    }

    #[test]
    fn a_verbatim_block_keeps_its_indentation_on_the_page() {
        let pages = pages("----\nfn main() {\n    indented();\n}\n----\n");

        assert!(
            pages[0].contains("    indented();") || pages[0].contains("indented();"),
            "got {:?}",
            pages[0]
        );
        assert!(pages[0].contains("fn main()"));
    }

    #[test]
    fn a_block_title_reaches_the_page_with_its_block() {
        let pages = pages(".Listing One\n----\ncode\n----\n");

        assert!(pages[0].contains("Listing One"), "got {:?}", pages[0]);
        assert!(pages[0].contains("code"));
    }

    #[test]
    fn a_verse_keeps_its_line_breaks_on_the_page() {
        // Read from the page rather than from the markup. A verse used to keep
        // its shape only because the newlines in its text reached the engine as
        // newlines, which a markup assertion cannot tell apart from prose that
        // happens not to have been filled yet.
        let pages = layout::render("[verse]\n____\nfirst line\nsecond line\nthird line\n____\n");
        let lines = pages[0].line_texts();

        assert_eq!(
            lines,
            vec![
                "first line".to_owned(),
                "second line".to_owned(),
                "third line".to_owned()
            ],
            "a verse sets each of its lines on a line of its own"
        );
    }

    #[test]
    fn a_quote_is_filled_rather_than_kept() {
        // The other half of the distinction: a quote is prose, so its source
        // wrapping is the author's editor and not the author.
        let pages = layout::render("[quote]\n____\nShort one\nshort two\nshort three.\n____\n");

        assert_eq!(
            pages[0].line_texts().len(),
            1,
            "a quote's short source lines are filled onto one line, got: {:?}",
            pages[0].line_texts()
        );
    }

    #[test]
    fn a_description_lists_terms_reach_the_page() {
        let pages = pages("apple:: a fruit\npear:: another fruit\n");

        assert!(pages[0].contains("apple"), "got {:?}", pages[0]);
        assert!(pages[0].contains("a fruit"));
        assert!(pages[0].contains("pear"));
    }

    #[test]
    fn a_verbatim_block_is_set_in_the_theme_s_monospace_family() {
        // The engine's `raw` element carries a show-set rule naming its own
        // monospace family, and a show-set beats the enclosing text style. So
        // the theme reaches verbatim content only through a show rule. Without
        // one this passed by coincidence: the built-in themes name the same
        // family the engine defaults to.
        let families = families(
            "----
code
----
",
            &theme_with_proportional_verbatim(),
        );

        assert!(
            families
                .iter()
                .any(|family| family.eq_ignore_ascii_case("DejaVu Sans")),
            "the theme's monospace family must reach the page, got: {families:?}"
        );
        assert!(
            !families
                .iter()
                .any(|family| family.eq_ignore_ascii_case("DejaVu Sans Mono")),
            "nothing may fall back to the engine's own default, got: {families:?}"
        );
    }

    #[test]
    fn inline_monospace_is_set_in_the_same_family_as_a_verbatim_block() {
        let families = families(
            "A `word` here.

----
code
----
",
            &theme_with_proportional_verbatim(),
        );

        assert!(
            !families
                .iter()
                .any(|family| family.eq_ignore_ascii_case("DejaVu Sans Mono")),
            "inline and block monospace must agree, got: {families:?}"
        );
    }
}
