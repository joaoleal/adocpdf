//! The record of tolerated fuzz defects, checked against the live guard.
//!
//! `fuzz/known-crashes.toml` tells the weekly fuzz job which panics to
//! tolerate, matched by the location upstream reports. An allowlist nobody
//! re-reads is how a regression hides, so this file re-reads it on every
//! `cargo test`: every entry's sample input must still be refused at the parse
//! boundary, and an entry that upstream fixes — or that the guard stops
//! covering — fails here rather than lingering as a permanent exemption.
//!
//! It is deliberately parsed with `std` alone, by the same rules the workflow's
//! shell uses. Reaching for a TOML crate would add a dependency and, worse,
//! give the two sides two different ideas of what the file says.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::PathBuf;

use adocpdf_domain::ports::{Date, DocumentParser};
use adocpdf_infra::parser::AsciidocParser;

/// The record's path, resolved from this crate rather than the working
/// directory, so the test passes wherever `cargo test` is invoked from.
fn record_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fuzz/known-crashes.toml")
}

/// One recorded defect.
#[derive(Debug)]
struct Entry {
    name: String,
    location: String,
    sample: Vec<u8>,
}

/// Reads the record, by the rules its own header states.
fn entries() -> Vec<Entry> {
    let text = std::fs::read_to_string(record_path()).expect("the record is readable");
    let mut entries: Vec<Entry> = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            entries.push(Entry {
                name: name.to_owned(),
                location: String::new(),
                sample: Vec::new(),
            });
            continue;
        }

        let (key, rest) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("line {number} is neither comment nor entry: {line:?}"));
        let value = rest
            .trim()
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_else(|| panic!("line {number} is not a quoted value: {line:?}"));
        let entry = entries
            .last_mut()
            .unwrap_or_else(|| panic!("line {number} sits outside any entry: {line:?}"));

        match key.trim() {
            "location" => value.clone_into(&mut entry.location),
            "sample" => entry.sample = unescape(value, number),
            "reason" => {}
            other => panic!("line {number}: {other:?} is not a key this format defines"),
        }
    }

    entries
}

/// Turns an entry's escaped text back into the bytes it stands for.
fn unescape(value: &str, line: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            let mut buffer = [0_u8; 4];
            out.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
            continue;
        }

        match characters.next() {
            Some('n') => out.push(b'\n'),
            Some('t') => out.push(b'\t'),
            Some('r') => out.push(b'\r'),
            Some('\\') => out.push(b'\\'),
            Some('u') => {
                let digits: String = characters.by_ref().take(4).collect();
                let scalar = u32::from_str_radix(&digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .unwrap_or_else(|| panic!("line {line}: \\u{digits} is not a character"));
                let mut buffer = [0_u8; 4];
                out.extend_from_slice(scalar.encode_utf8(&mut buffer).as_bytes());
            }
            other => panic!("line {line}: \\{other:?} is not an escape this format defines"),
        }
    }

    out
}

fn refuses(input: &[u8]) -> bool {
    let source = std::str::from_utf8(input).expect("a recorded sample is text");
    let today = Date::new(2026, 8, 22).expect("a real date");

    AsciidocParser
        .parse(source, "known-crash.adoc", today)
        .is_err()
}

#[test]
fn every_recorded_defect_is_still_refused_by_the_guard() {
    // The point of the whole file. A defect the job tolerates must be one the
    // shipped renderer already refuses; the day that stops being true, the
    // record is granting an exemption for a defect that reaches users.
    for entry in entries() {
        assert!(
            refuses(&entry.sample),
            "{} is tolerated by the fuzz job but its sample is no longer refused \
             by the guard. Either the guard has regressed, or upstream fixed the \
             defect and the entry should be removed.",
            entry.name
        );
    }
}

#[test]
fn every_entry_is_complete() {
    // A table missing its location tolerates nothing and would silently never
    // match; one missing its sample makes the test above vacuous.
    for entry in entries() {
        assert!(
            !entry.location.is_empty(),
            "{} records no location, so the job could never match it",
            entry.name
        );
        assert!(
            !entry.sample.is_empty(),
            "{} records no sample, so nothing checks it is still guarded",
            entry.name
        );
    }
}

#[test]
fn the_record_holds_the_defects_the_fuzzer_has_actually_found() {
    let entries = entries();

    assert_eq!(
        entries.len(),
        2,
        "two defects are recorded; adding a third is a real decision and should \
         update this count deliberately, got: {:?}",
        entries.iter().map(|entry| &entry.name).collect::<Vec<_>>()
    );
}

#[test]
fn no_defect_is_recorded_twice() {
    // A duplicate is a sign the record is being edited rather than read, and
    // two entries for one location would make the second unreachable.
    let entries = entries();

    for (index, entry) in entries.iter().enumerate() {
        for other in entries.iter().skip(index + 1) {
            assert_ne!(entry.name, other.name, "two entries share a name");
            assert_ne!(
                entry.location, other.location,
                "{} and {} record the same location",
                entry.name, other.name
            );
        }
    }
}

#[test]
fn the_escapes_the_format_defines_round_trip() {
    assert_eq!(unescape(r"a\nb", 0), b"a\nb");
    assert_eq!(unescape(r"a\tb", 0), b"a\tb");
    assert_eq!(unescape(r"a\rb", 0), b"a\rb");
    assert_eq!(unescape(r"a\\b", 0), b"a\\b");
    assert_eq!(unescape(r"a\u0002b", 0), b"a\x02b");
    assert_eq!(unescape("plain", 0), b"plain");
}
