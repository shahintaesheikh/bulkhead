//! Fixture-driven parser tests.
//!
//! Fixtures live at the repo root (`tests/fixtures/`), per the documented
//! layout, so resolve them against the crate's manifest directory.

use std::fs;
use std::path::{Path, PathBuf};

use bh_core::status::{StatusLine, StatusParseError, StatusVerb};

fn fixture(relative: &str) -> (PathBuf, String) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative);
    let contents =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    (path, contents)
}

fn non_empty_lines(contents: &str) -> impl Iterator<Item = &str> {
    contents.lines().filter(|line| !line.trim().is_empty())
}

#[test]
fn parses_every_known_verb_and_an_unknown_one() {
    let (path, contents) = fixture("status/verbs.log");

    let lines: Vec<StatusLine> = non_empty_lines(&contents)
        .map(|line| {
            StatusLine::parse(line).unwrap_or_else(|err| panic!("{path:?} {line:?}: {err}"))
        })
        .collect();

    let verbs: Vec<StatusVerb> = lines.iter().map(|line| line.verb.clone()).collect();
    assert_eq!(
        verbs,
        vec![
            StatusVerb::Working,
            StatusVerb::NeedsDecision,
            StatusVerb::Blocked,
            StatusVerb::Pr,
            StatusVerb::Done,
            StatusVerb::Unknown("deploying".to_owned()),
        ]
    );

    assert_eq!(
        lines[3].message,
        "https://github.com/shahintaesheikh/bulkhead/pull/7"
    );
}

#[test]
fn malformed_lines_produce_typed_errors() {
    let (_, contents) = fixture("status/malformed.log");
    let mut lines = non_empty_lines(&contents);

    assert_eq!(
        StatusLine::parse(lines.next().expect("a line")),
        Err(StatusParseError::MissingSeparator)
    );
    assert_eq!(
        StatusLine::parse(lines.next().expect("a line")),
        Err(StatusParseError::EmptyVerb)
    );
}
